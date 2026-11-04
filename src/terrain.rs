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
const TERRAIN_SIZE: f32 = 40.0;
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
            base_color: Color::srgb(0.55, 0.65, 0.25),
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
        for (index, position) in positions.iter_mut().enumerate() {
            let x = index as u32 % width;
            let y = index as u32 / width;
            position[1] = sample_height(image, x, y) * height_scale;
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

    let mut heights: Vec<Vec<f32>> = Vec::with_capacity(height as usize);
    for y in 0..height {
        let mut row: Vec<f32> = Vec::with_capacity(width as usize);
        for x in 0..width {
            row.push(sample_height(image, x, y) * height_scale);
        }
        heights.push(row);
    }

    Collider::heightfield(heights, Vec3::new(size / 2.0, 1.0, size / 2.0))
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
