//! Terreno: heightmap PNG, mesh generato a runtime e collider heightfield.
//!
//! Il terreno usa una texture PNG in scala di grigi come heightmap: ogni pixel
//! definisce l’altezza del terreno in un punto della griglia. Il mesh viene
//! costruito a runtime da un piano suddiviso, e il collider è un heightfield
//! Avian, così il player cammina realmente sulle colline.
//!
//! Il flusso è pensato per il reset della scena e l’hot-reload dell’asset:
//!
//! - `setup_assets` carica la heightmap e registra la resource `TerrainAssets`.
//! - `spawn_terrain` crea l’entità terreno con un piano piatto segnaposto.
//! - `update_terrain` osserva l’asset e, quando la heightmap è disponibile
//!   (o cambia a runtime), genera mesh e collider e li sostituisce sul posto.

use crate::scene::{Ground, SceneNode};
use avian3d::prelude::*;
use bevy::asset::AssetEvent;
use bevy::image::Image;
use bevy::math::{UVec3, Vec2};
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;

/// Estensione del terreno sul piano XZ (m): il piano è centrato in origine.
const TERRAIN_SIZE: f32 = 100.0;
/// Scala verticale della heightmap: i valori 0..255 diventano -1..+1 e poi
/// vengono moltiplicati per `HEIGHT_SCALE` (m).
const HEIGHT_SCALE: f32 = 4.0;

/// Asset e stato del terreno.
#[derive(Resource)]
pub struct TerrainAssets {
    /// Handle della heightmap PNG in scala di grigi.
    pub heightmap: Handle<Image>,
    /// Handle del mesh generato dall’ultima heightmap caricata.
    /// `None` finché la heightmap non è pronta (o dopo un hot-reload).
    pub built_mesh: Option<Handle<Mesh>>,
}

/// Marker dell’entità terreno (soggetta a mesh/collider dinamici).
#[derive(Component)]
pub struct TerrainMesh;

/// Carica la heightmap e inserisce la resource [`TerrainAssets`].
///
/// Da chiamare in `Startup`: la resource vive fuori dal reset della scena,
/// quindi il terreno può essere ricostruito senza rilanciare il load dell’asset.
pub fn setup_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let heightmap = asset_server.load("textures/terrain_heightmap.png");
    commands.insert_resource(TerrainAssets {
        heightmap,
        built_mesh: None,
    });
}

/// Spawna l’entità terreno: piano piatto segnaposto + collider cuboid.
///
/// Il mesh e il collider vengono sostituiti da [`update_terrain`] appena la
/// heightmap è disponibile. Il segnaposto serve solo a far esistere l’entità
/// fin dal primo frame, così camera e player hanno un terreno su cui appoggiarsi.
pub fn spawn_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let placeholder_mesh = meshes.add(Plane3d {
        half_size: Vec2::splat(TERRAIN_SIZE / 2.0),
        ..Default::default()
    });

    commands.spawn((
        SceneNode,
        Ground,
        TerrainMesh,
        RigidBody::Static,
        Collider::cuboid(TERRAIN_SIZE, 1.0, TERRAIN_SIZE),
        Mesh3d(placeholder_mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.50, 0.22),
            perceptual_roughness: 0.85,
            metallic: 0.0,
            ..Default::default()
        })),
    ));
}

/// Ricostruisce mesh e collider del terreno quando serve:
///
/// - al primo caricamento della heightmap;
/// - dopo un reset della scena (il segnaposto è di nuovo attivo);
/// - dopo un hot-reload dell’asset (`AssetEvent::Modified`).
pub fn update_terrain(
    mut commands: Commands,
    mut assets: ResMut<TerrainAssets>,
    images: Res<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut events: MessageReader<AssetEvent<Image>>,
    terrains: Query<(Entity, &Mesh3d), With<TerrainMesh>>,
) {
    // Hot-reload: se la heightmap è stata modificata, invalida il mesh attuale.
    for event in events.read() {
        if let AssetEvent::Modified { id } = event
            && *id == assets.heightmap.id()
        {
            assets.built_mesh = None;
        }
    }

    // Nessuna entità terreno (reset in corso o scena vuota): niente da fare.
    let Ok((entity, mesh3d)) = terrains.single() else {
        return;
    };

    // Se il mesh attuale è già quello costruito, niente da aggiornare.
    if let Some(built) = &assets.built_mesh
        && mesh3d.0 == *built
    {
        return;
    }

    // La heightmap non è ancora caricata: mantiene il segnaposto.
    let Some(image) = images.get(&assets.heightmap) else {
        return;
    };

    // Genera mesh e collider dalla heightmap.
    let mesh = build_terrain_mesh(image, TERRAIN_SIZE, HEIGHT_SCALE);
    let collider = build_terrain_collider(image, TERRAIN_SIZE, HEIGHT_SCALE);

    // Log diagnostico: dimensioni della heightmap e range delle altezze.
    let (mut h_min, mut h_max) = (f32::INFINITY, f32::NEG_INFINITY);
    for y in 0..image.height() {
        for x in 0..image.width() {
            let h = sample_height(image, x, y) * HEIGHT_SCALE;
            h_min = h_min.min(h);
            h_max = h_max.max(h);
        }
    }
    info!(
        "terrain diagnostics: image {}x{}, height range [{:.2}, {:.2}] m, size {} m, collider scale = ({}, 1, {})",
        image.width(),
        image.height(),
        h_min,
        h_max,
        TERRAIN_SIZE,
        TERRAIN_SIZE,
        TERRAIN_SIZE
    );

    let mesh_handle = meshes.add(mesh);
    assets.built_mesh = Some(mesh_handle.clone());

    commands
        .entity(entity)
        .insert((Mesh3d(mesh_handle), collider));
}

/// Costruisce il mesh del terreno da una heightmap.
///
/// Il mesh parte da un piano suddiviso con `PlaneMeshBuilder` e poi solleva i
/// vertici in base alla heightmap: il numero di suddivisioni è scelto per
/// avere un vertice per pixel della heightmap (il piano ha già i vertici
/// dei bordi, quindi servono `width-2` suddivisioni interne).
fn build_terrain_mesh(image: &Image, size: f32, height_scale: f32) -> Mesh {
    let width = image.width();
    let height = image.height();

    let mut mesh = Plane3d {
        half_size: Vec2::splat(size / 2.0),
        ..Default::default()
    }
    .mesh()
    .subdivisions_x(width.saturating_sub(2))
    .subdivisions_z(height.saturating_sub(2))
    .build();

    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        // PlaneMeshBuilder genera vertici con loop esterno su Z e interno su X:
        // index = z * width + x  =>  x = index % width, z = index / width.
        // Usiamo sample_height(image, x, z) con z = riga immagine (y).
        for (index, position) in positions.iter_mut().enumerate() {
            let x = index as u32 % width;
            let z = index as u32 / width;
            position[1] = sample_height(image, x, z) * height_scale;
        }
    }

    mesh.compute_normals();
    mesh
}

/// Costruisce il collider heightfield Avian dalla stessa heightmap.
///
/// Avian si aspetta una matrice di altezze e un fattore di scala: il campo
/// occupa ±`size/2` sul piano XZ, con altezze comprese tra -`height_scale`
/// e +`height_scale` (la heightmap è centrata sul grigio medio).
fn build_terrain_collider(image: &Image, size: f32, height_scale: f32) -> Collider {
    let width = image.width();
    let height = image.height();

    // PASSO 2 DEFINITIVO (fix orientamento): la matrice Parry `heights_zx`
    // e' indicizzata [i, j] con i = riga (asse Z) e j = colonna (asse X).
    // La mesh usa index = z*width + x,cioè' riga=z, colonna=x.
    // Quindi heights[z][x] = sample(x, z) allinea perfettamente i due.
    // Verifica numerica: la mappa ASCII della heightmap mostra che NW e'
    // basso e SE e' alto; il wireframe F3 deve mostrare la stessa forma.
    // FIX FINALE COLLIDER: la matrice deve essere heights[x][z] (riga=X,
    // colonna=Z) perche' Parry legge nrows->Z? NO: nrows conta le righe
    // che scorrono su Z, ma noi vogliamo che l'indice di riga corrisponda
    // a X perche' la mesh ha x come indice veloce. In realta' il problema
    // e' la trasposizione: costruiamo heights[x][z] = sample(x,z).
    let mut heights: Vec<Vec<f32>> = Vec::with_capacity(width as usize);
    for x in 0..width {
        let mut row: Vec<f32> = Vec::with_capacity(height as usize);
        for z in 0..height {
            row.push(sample_height(image, x, z) * height_scale);
        }
        heights.push(row);
    }

    // PASSAGGI 1+2 (fix scala e copertura): Parry heightfield scala i vertici
    // locali (che stanno in [-0.5, 0.5]) per `scale`, quindi l'estensione
    // reale su XZ e' `scale.x`/`scale.z`. Per coprire `size` metri:
    // scale.x = size, scale.z = size. Il vecchio `size/2` dimezzava il campo.
    // Parry: i vertici locali vanno da -0.5 a 0.5 e vengono moltiplicati
    // per `scale`, quindi l'estensione reale su XZ e' proprio `scale.x/z`.
    // Per coprire `size` metri: scale.x = scale.z = size.
    Collider::heightfield(heights, Vec3::new(size, 1.0, size))
}

/// Campiona la heightmap come valore -1..+1 (centrata sul grigio medio).
///
/// La PNG è in scala di grigi, ma il loader può conservarla come 1 byte
/// (R8Unorm) o 4 byte (Rgba8UnormSrgb): in entrambi i casi il primo canale
/// contiene l’intensità che ci serve. Mappiamo 0..255 su -1..+1 così il
/// grigio medio (128) è l’altezza zero, utile per avere colline sopra e
/// sotto il piano di base.
fn sample_height(image: &Image, x: u32, y: u32) -> f32 {
    match image.pixel_bytes(UVec3::new(x, y, 0)) {
        Ok(bytes) if !bytes.is_empty() => {
            let raw = bytes[0] as f32 / 255.0;
            raw * 2.0 - 1.0
        }
        _ => 0.0,
    }
}
