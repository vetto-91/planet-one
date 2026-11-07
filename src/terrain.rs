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
use bevy::image::{
    Image, ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
    ImageSamplerDescriptor,
};
use bevy::math::{Affine2, UVec3, Vec2};
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;

/// Estensione del terreno sul piano XZ (m): il piano è centrato in origine.
/// È un'isola: il quadrato centrale di `HEIGHTMAP_SIZE` m usa la heightmap, il
/// resto è procedurale (colline dolci e costa irregolare) e scende nel mare.
pub const TERRAIN_SIZE: f32 = 300.0;
/// Lato (m) del quadrato coperto dalla heightmap PNG (centrato).
const HEIGHTMAP_SIZE: f32 = 100.0;
/// Passo (m) della griglia di vertici.
const STEP: f32 = 0.5;
/// Quota (m) del livello del mare e della riva (dove si ferma il giocatore).
pub const SEA_LEVEL: f32 = -1.0;
const SHORE_LEVEL: f32 = -0.3;
/// Scala verticale della heightmap: i valori 0..255 diventano -1..+1 e poi
/// vengono moltiplicati per `HEIGHT_SCALE` (m).
const HEIGHT_SCALE: f32 = 4.0;
/// Lato (m) di un tile della texture erba.
const GRASS_TILE_M: f32 = 10.0;

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

/// Muro invisibile lungo la riva (il giocatore non entra in acqua).
#[derive(Component)]
pub struct ShoreWall;

/// Piano d'acqua del mare.
#[derive(Component)]
struct Sea;

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
    asset_server: &AssetServer,
) {
    // Texture erba ripetuta: sampler in modalità Repeat, un tile ogni GRASS_TILE_M metri.
    let grass = asset_server
        .load_builder()
        .with_settings(|s: &mut ImageLoaderSettings| {
            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                mipmap_filter: ImageFilterMode::Linear,
                anisotropy_clamp: 16,
                ..Default::default()
            });
        })
        .load("textures/grass.png");
    let placeholder_mesh = meshes.add(Plane3d {
        half_size: Vec2::splat(TERRAIN_SIZE / 2.0),
        ..Default::default()
    });

    // Mare: piano enorme, senza collider, semitrasparente.
    commands.spawn((
        SceneNode,
        Sea,
        Mesh3d(meshes.add(Plane3d {
            half_size: Vec2::splat(4000.0),
            ..Default::default()
        })),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.12, 0.38, 0.55, 0.82),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.15,
            metallic: 0.0,
            ..Default::default()
        })),
        Transform::from_xyz(0.0, SEA_LEVEL, 0.0),
        bevy::light::NotShadowCaster,
    ));

    commands.spawn((
        SceneNode,
        Ground,
        TerrainMesh,
        RigidBody::Static,
        Collider::cuboid(TERRAIN_SIZE, 1.0, TERRAIN_SIZE),
        Mesh3d(placeholder_mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(grass),
            uv_transform: Affine2::from_scale(Vec2::splat(TERRAIN_SIZE / GRASS_TILE_M)),
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
    walls: Query<Entity, With<ShoreWall>>,
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
    let field = build_field(image);
    let mesh = build_terrain_mesh(&field);
    let collider = build_terrain_collider(&field);
    let n = field.n;

    // Log diagnostico: range delle altezze.
    let (mut h_min, mut h_max) = (f32::INFINITY, f32::NEG_INFINITY);
    for h in &field.h {
        h_min = h_min.min(*h);
        h_max = h_max.max(*h);
    }
    info!(
        "terrain diagnostics: island {n}x{n} vertices, height range [{h_min:.2}, {h_max:.2}] m, size {TERRAIN_SIZE} m"
    );

    let mesh_handle = meshes.add(mesh);
    assets.built_mesh = Some(mesh_handle.clone());

    // Muro invisibile lungo la riva (ricostruito a ogni rigenerazione del terreno).
    for w in &walls {
        commands.entity(w).despawn();
    }
    let segments = shore_segments(&field);
    info!("[TERRAIN] muro di riva: {} segmenti", segments.len());
    commands
        .entity(entity)
        .insert((Mesh3d(mesh_handle), collider))
        .with_children(|c| {
            for (mid, len, yaw) in segments {
                c.spawn((
                    ShoreWall,
                    Collider::cuboid(len, 8.0, 1.0),
                    Transform::from_xyz(mid.x, 3.0, mid.y)
                        .with_rotation(Quat::from_rotation_y(yaw)),
                ));
            }
        });
}

/// Campo di altezze dell'isola: `n`×`n` vertici, indice `z * n + x`.
struct Field {
    n: usize,
    h: Vec<f32>,
    /// Colore di vertice (sabbia sulla riva).
    color: Vec<[f32; 4]>,
}

impl Field {
    fn at(&self, x: usize, z: usize) -> f32 {
        self.h[z * self.n + x]
    }
}

fn hash2(ix: i32, iz: i32) -> f32 {
    let mut h = (ix as u32).wrapping_mul(374_761_393) ^ (iz as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xFFFF) as f32 / 65_535.0
}

/// Value noise liscio 0..1.
fn noise2(x: f32, z: f32) -> f32 {
    let (ix, iz) = (x.floor() as i32, z.floor() as i32);
    let (fx, fz) = (x - ix as f32, z - iz as f32);
    let (u, v) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let a = hash2(ix, iz) * (1.0 - u) + hash2(ix + 1, iz) * u;
    let b = hash2(ix, iz + 1) * (1.0 - u) + hash2(ix + 1, iz + 1) * u;
    a * (1.0 - v) + b * v
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Raggio dell'isola nella direzione dell'angolo `a` (m): costa frastagliata.
fn island_radius(a: f32) -> f32 {
    let (c, s) = (a.cos(), a.sin());
    // Rumore periodico: campionato su un cerchio, così si chiude senza cucitura.
    let n1 = noise2(c * 2.2 + 11.0, s * 2.2 + 7.0);
    let n2 = noise2(c * 5.5 + 3.0, s * 5.5 + 19.0);
    104.0 + 34.0 * n1 + 9.0 * n2
}

/// Vero se il punto (x, z) cade in mare (solo la parte procedurale: il quadrato
/// centrale della heightmap è sempre terra). Usato per non piazzare oggetti in acqua.
pub fn is_water(x: f32, z: f32) -> bool {
    if x.abs() <= HEIGHTMAP_SIZE / 2.0 && z.abs() <= HEIGHTMAP_SIZE / 2.0 {
        return false;
    }
    let m = (x * x + z * z).sqrt() / island_radius(z.atan2(x));
    m > 0.9
}

/// Altezza (m) del terreno in (x, z).
fn island_height(image: &Image, x: f32, z: f32) -> f32 {
    // Parte interna: heightmap campionata (bilinear), con coordinate bloccate al riquadro.
    let half = HEIGHTMAP_SIZE / 2.0;
    let (w, h) = (image.width() as f32, image.height() as f32);
    let u = ((x.clamp(-half, half) + half) / HEIGHTMAP_SIZE * (w - 1.0)).clamp(0.0, w - 1.0);
    let v = ((z.clamp(-half, half) + half) / HEIGHTMAP_SIZE * (h - 1.0)).clamp(0.0, h - 1.0);
    let (x0, z0) = (u.floor() as u32, v.floor() as u32);
    let (x1, z1) = ((x0 + 1).min(w as u32 - 1), (z0 + 1).min(h as u32 - 1));
    let (fu, fv) = (u - x0 as f32, v - z0 as f32);
    let top = sample_height(image, x0, z0) * (1.0 - fu) + sample_height(image, x1, z0) * fu;
    let bot = sample_height(image, x0, z1) * (1.0 - fu) + sample_height(image, x1, z1) * fu;
    let inner = (top * (1.0 - fv) + bot * fv) * HEIGHT_SCALE;

    // Parte esterna: colline dolci.
    let hills = (noise2(x * 0.035 + 50.0, z * 0.035 + 20.0) - 0.5) * 3.0
        + (noise2(x * 0.09, z * 0.09 + 100.0) - 0.5) * 1.0
        + 0.8;
    // Raccordo dentro/fuori sul bordo del riquadro (distanza di Chebyshev).
    let d = x.abs().max(z.abs());
    let land = inner + (hills - inner) * smooth(half, half + 25.0, d);

    // Costa: la terra scende nel mare verso il bordo irregolare dell'isola.
    let m = (x * x + z * z).sqrt() / island_radius(z.atan2(x));
    land + (-3.5 - land) * smooth(0.72, 1.0, m)
}

fn build_field(image: &Image) -> Field {
    let n = (TERRAIN_SIZE / STEP) as usize + 1;
    let mut h = Vec::with_capacity(n * n);
    let mut color = Vec::with_capacity(n * n);
    for iz in 0..n {
        for ix in 0..n {
            let x = -TERRAIN_SIZE / 2.0 + ix as f32 * STEP;
            let z = -TERRAIN_SIZE / 2.0 + iz as f32 * STEP;
            let y = island_height(image, x, z);
            h.push(y);
            // Sabbia vicino al livello del mare, erba altrove.
            let sand = 1.0 - smooth(SEA_LEVEL + 0.2, SEA_LEVEL + 1.1, y);
            color.push([
                1.0 + (0.95 - 1.0) * sand,
                1.0 + (0.85 - 1.0) * sand,
                1.0 + (0.55 - 1.0) * sand,
                1.0,
            ]);
        }
    }
    Field { n, h, color }
}

/// Costruisce il mesh del terreno dal campo di altezze.
///
/// Il mesh parte da un piano suddiviso con `PlaneMeshBuilder` (un vertice per
/// cella del campo) e poi solleva i vertici.
fn build_terrain_mesh(field: &Field) -> Mesh {
    let n = field.n;
    let mut mesh = Plane3d {
        half_size: Vec2::splat(TERRAIN_SIZE / 2.0),
        ..Default::default()
    }
    .mesh()
    .subdivisions_x(n as u32 - 2)
    .subdivisions_z(n as u32 - 2)
    .build();

    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        // PlaneMeshBuilder genera vertici con loop esterno su Z e interno su X:
        // index = z * n + x.
        for (index, position) in positions.iter_mut().enumerate() {
            position[1] = field.h[index];
        }
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, field.color.clone());
    mesh.compute_normals();
    mesh
}

/// Collider heightfield Avian dalla stessa griglia: heights[x][z] (riga = X, colonna = Z).
fn build_terrain_collider(field: &Field) -> Collider {
    let n = field.n;
    let heights: Vec<Vec<f32>> = (0..n)
        .map(|x| (0..n).map(|z| field.at(x, z)).collect())
        .collect();
    Collider::heightfield(heights, Vec3::new(TERRAIN_SIZE, 1.0, TERRAIN_SIZE))
}

/// Segmenti del muro di riva: (punto medio (x,z), lunghezza, rotazione Y). Si segue la
/// linea di costa (livello `SHORE_LEVEL`) con 180 raggi dal centro.
fn shore_segments(field: &Field) -> Vec<(Vec2, f32, f32)> {
    let sample = |x: f32, z: f32| -> f32 {
        let fx = ((x + TERRAIN_SIZE / 2.0) / STEP).clamp(0.0, (field.n - 1) as f32);
        let fz = ((z + TERRAIN_SIZE / 2.0) / STEP).clamp(0.0, (field.n - 1) as f32);
        field.at(fx.round() as usize, fz.round() as usize)
    };
    let count = 180;
    let mut pts: Vec<Vec2> = Vec::with_capacity(count);
    for i in 0..count {
        let a = i as f32 / count as f32 * std::f32::consts::TAU;
        let dir = Vec2::new(a.cos(), a.sin());
        // Dal centro verso l'esterno: primo punto sotto la riva.
        let mut r = 60.0;
        while r < TERRAIN_SIZE / 2.0 && sample(dir.x * r, dir.y * r) > SHORE_LEVEL {
            r += 0.5;
        }
        pts.push(dir * r);
    }
    (0..count)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % count]);
            let d = b - a;
            // Il muro è lungo un po' di più per non lasciare fessure; yaw: asse X locale lungo d.
            ((a + b) / 2.0, d.length() + 0.6, (-d.y).atan2(d.x))
        })
        .collect()
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
