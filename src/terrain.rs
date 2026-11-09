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
use crate::village::{Road, VillageInfo, dist_to_segment, signed_distance};
use avian3d::prelude::*;
use bevy::asset::AssetEvent;
use bevy::image::Image;
use bevy::math::{UVec3, Vec2};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};

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
const GRASS_TILE_M: f32 = 4.0;
/// Segno della bitangente per la griglia del terreno (V cresce verso +Z).
const TANGENT_W: f32 = 1.0;

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

/// Parti generate con il terreno (mare, sabbia, barriera di riva): si rifanno insieme a lui.
#[derive(Component)]
pub struct TerrainExtra;

/// Campo di altezze del terreno (dopo la generazione) e punti della riva per il debug F3.
/// Serve ad appoggiare gli oggetti a terra senza dipendere dalla fisica.
#[derive(Resource)]
pub struct TerrainField {
    n: usize,
    h: Vec<f32>,
    shore: Vec<Vec2>,
}

impl TerrainField {
    /// Quota (m) del terreno in (x, z), interpolazione bilineare tra i vertici.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let max = (self.n - 1) as f32;
        let fx = ((x + TERRAIN_SIZE / 2.0) / STEP).clamp(0.0, max);
        let fz = ((z + TERRAIN_SIZE / 2.0) / STEP).clamp(0.0, max);
        let (x0, z0) = (fx.floor() as usize, fz.floor() as usize);
        let (x1, z1) = ((x0 + 1).min(self.n - 1), (z0 + 1).min(self.n - 1));
        let (u, v) = (fx - x0 as f32, fz - z0 as f32);
        let at = |x: usize, z: usize| self.h[z * self.n + x];
        let top = at(x0, z0) * (1.0 - u) + at(x1, z0) * u;
        let bot = at(x0, z1) * (1.0 - u) + at(x1, z1) * u;
        top * (1.0 - v) + bot * v
    }
}

/// Lato (m) di una piastrella della texture sabbia e del selciato.
const SAND_TILE_M: f32 = 12.0;
const ROAD_TILE_M: f32 = 2.4;

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
    let placeholder_mesh = meshes.add(Plane3d {
        half_size: Vec2::splat(TERRAIN_SIZE / 2.0),
        ..Default::default()
    });

    commands.spawn((
        SceneNode,
        Ground,
        TerrainMesh,
        crate::seasons::SeasonGround { sand: false },
        RigidBody::Static,
        // Il wireframe di F3 sul terreno (720.000 triangoli) è troppo pesante: lo sostituisce
        // `terrain_debug_gizmos`.
        DebugRender::none(),
        Collider::cuboid(TERRAIN_SIZE, 1.0, TERRAIN_SIZE),
        Mesh3d(placeholder_mesh),
        MeshMaterial3d(materials.add(crate::pbr::material(
            asset_server,
            "grass",
            Vec2::splat(TERRAIN_SIZE / GRASS_TILE_M),
        ))),
    ));
}

/// Ricostruisce mesh e collider del terreno quando serve:
///
/// - al primo caricamento della heightmap;
/// - dopo un reset della scena (il segnaposto è di nuovo attivo);
/// - dopo un hot-reload dell’asset (`AssetEvent::Modified`).
#[allow(clippy::too_many_arguments)]
pub fn update_terrain(
    mut commands: Commands,
    mut assets: ResMut<TerrainAssets>,
    images: Res<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut events: MessageReader<AssetEvent<Image>>,
    terrains: Query<(Entity, &Mesh3d), With<TerrainMesh>>,
    extras: Query<Entity, With<TerrainExtra>>,
    village: Option<Res<VillageInfo>>,
    dbg: Option<Res<crate::debug_place::DebugPlace>>,
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
    let vil = village.map(|v| (*v).clone()).unwrap_or_default();
    // Spiazzi piani sotto gli edifici visitabili.
    let pads: Vec<BuildingPad> = dbg
        .as_ref()
        .map(|d| {
            d.items
                .iter()
                .filter_map(|p| crate::buildings::interior_half(&p.kind).map(|half| (p, half)))
                .flat_map(|(p, half)| {
                    p.points.iter().enumerate().map(move |(j, q)| BuildingPad {
                        center: *q,
                        half,
                        yaw: p.yaw(j),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let field = build_field(image, &vil, &pads);
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

    // Mare solo attorno all'isola (flood-fill dal bordo), sabbia sfumata, barriera di riva.
    let ocean = flood_ocean(&field, SEA_LEVEL);
    let shore_mask = flood_ocean(&field, SHORE_LEVEL);
    let sand_w = sand_weights(&field, &ocean);
    let sand_mesh = build_sand_mesh(&mesh, &sand_w);
    let road_w = road_weights(&field, &vil.spec.roads);
    let road_mesh = build_sand_mesh(&mesh, &road_w);
    let snow_mesh = build_sand_mesh(&mesh, &snow_weights(&field, &ocean, &sand_w, &road_w));
    let sea_mesh = build_sea_mesh(&field, &ocean);
    // I moli (piazzamenti `molo`) interrompono la barriera di riva: ci si cammina sopra.
    let piers: Vec<(Vec2, f32)> = dbg
        .map(|d| {
            d.items
                .iter()
                .filter(|p| p.kind == "molo")
                .flat_map(|p| p.points.iter().enumerate().map(|(j, q)| (*q, p.yaw(j))))
                .collect()
        })
        .unwrap_or_default();
    let barrier = build_barrier(&field, &shore_mask, &piers);
    let shore = shore_points(&field, &shore_mask);
    info!(
        "[TERRAIN] riva: {} punti; sabbia: {} vertici",
        shore.len(),
        sand_w.iter().filter(|w| **w > 0.02).count()
    );
    commands.insert_resource(TerrainField {
        n,
        h: field.h.clone(),
        shore,
    });

    let mesh_handle = meshes.add(mesh);
    assets.built_mesh = Some(mesh_handle.clone());

    for e in &extras {
        commands.entity(e).despawn();
    }
    commands.spawn((
        SceneNode,
        TerrainExtra,
        crate::seasons::SeasonGround { sand: true },
        Mesh3d(meshes.add(sand_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            depth_bias: 0.5,
            ..crate::pbr::material(
                &asset_server,
                "sand",
                Vec2::splat(TERRAIN_SIZE / SAND_TILE_M),
            )
        })),
        Transform::default(),
        bevy::light::NotShadowCaster,
    ));
    commands.spawn((
        SceneNode,
        TerrainExtra,
        Mesh3d(meshes.add(road_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            depth_bias: 1.5,
            ..crate::pbr::material(
                &asset_server,
                "cobble",
                Vec2::splat(TERRAIN_SIZE / ROAD_TILE_M),
            )
        })),
        Transform::default(),
        bevy::light::NotShadowCaster,
    ));
    // Neve: stesso reticolo del terreno, alfa = quantità (la regola `seasons::apply_season`).
    commands.spawn((
        SceneNode,
        TerrainExtra,
        crate::seasons::SeasonSnowGround,
        Mesh3d(meshes.add(snow_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.93, 0.96, 1.0, 0.0),
            emissive: LinearRgba::rgb(0.22, 0.24, 0.27),
            perceptual_roughness: 0.9,
            alpha_mode: AlphaMode::Blend,
            depth_bias: 2.5,
            ..Default::default()
        })),
        Transform::default(),
        Visibility::Hidden,
        bevy::light::NotShadowCaster,
    ));
    commands.spawn((
        SceneNode,
        TerrainExtra,
        Mesh3d(meshes.add(sea_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.05, 0.22, 0.33, 0.86),
            reflectance: 0.6,
            alpha_mode: AlphaMode::Blend,
            // Il mare si disegna dopo sabbia e strade (anche dall'alto, nella mappa).
            depth_bias: 4.0,
            perceptual_roughness: 0.08,
            metallic: 0.0,
            ..Default::default()
        })),
        Transform::from_xyz(0.0, SEA_LEVEL, 0.0),
        bevy::light::NotShadowCaster,
    ));
    // Barriera invisibile lungo la riva: un heightfield con "muro" sul mare.
    commands.spawn((
        SceneNode,
        TerrainExtra,
        RigidBody::Static,
        barrier,
        DebugRender::none(),
        Transform::default(),
    ));
    commands
        .entity(entity)
        .insert((Mesh3d(mesh_handle), collider));
}

/// Debug F3: griglia sparsa di collisione attorno al giocatore e recinto della riva
/// (al posto del wireframe completo del terreno, che costa 7 FPS).
pub fn terrain_debug_gizmos(
    mut gizmos: Gizmos,
    store: Res<GizmoConfigStore>,
    data: Option<Res<TerrainField>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
) {
    let (cfg, _) = store.config::<avian3d::prelude::PhysicsGizmos>();
    let (Some(data), true, Ok(p)) = (data, cfg.enabled, players.single()) else {
        return;
    };
    let pos = p.translation();
    let n = data.n;
    let at = |ix: usize, iz: usize| data.h[iz * n + ix];
    let world = |i: usize| -TERRAIN_SIZE / 2.0 + i as f32 * STEP;
    let idx = |w: f32| (((w + TERRAIN_SIZE / 2.0) / STEP).round().max(0.0) as usize).min(n - 1);
    let (cx, cz) = (idx(pos.x), idx(pos.z));
    let r = (35.0 / STEP) as usize;
    let skip = (2.0 / STEP) as usize; // passo 2 m
    let color = Color::srgb(1.0, 0.6, 0.1);
    let (x0, x1) = (cx.saturating_sub(r) / skip * skip, (cx + r).min(n - 1));
    let (z0, z1) = (cz.saturating_sub(r) / skip * skip, (cz + r).min(n - 1));
    let mut z = z0;
    while z <= z1 {
        let mut x = x0;
        while x <= x1 {
            let a = Vec3::new(world(x), at(x, z), world(z));
            if x + skip < n {
                gizmos.line(
                    a,
                    Vec3::new(world(x + skip), at(x + skip, z), world(z)),
                    color,
                );
            }
            if z + skip < n {
                gizmos.line(
                    a,
                    Vec3::new(world(x), at(x, z + skip), world(z + skip)),
                    color,
                );
            }
            x += skip;
        }
        z += skip;
    }
    // Recinto della riva (barriera invisibile): paletti rossi.
    let red = Color::srgb(1.0, 0.1, 0.1);
    for s in data
        .shore
        .iter()
        .filter(|s| s.distance(Vec2::new(pos.x, pos.z)) < 60.0)
    {
        let y = at(idx(s.x), idx(s.y));
        gizmos.line(Vec3::new(s.x, y, s.y), Vec3::new(s.x, y + 3.0, s.y), red);
    }
}

/// Campo di altezze dell'isola: `n`×`n` vertici, indice `z * n + x`.
struct Field {
    n: usize,
    h: Vec<f32>,
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

fn build_field(image: &Image, village: &VillageInfo, pads: &[BuildingPad]) -> Field {
    let n = (TERRAIN_SIZE / STEP) as usize + 1;
    let mut h = Vec::with_capacity(n * n);
    for iz in 0..n {
        for ix in 0..n {
            let x = -TERRAIN_SIZE / 2.0 + ix as f32 * STEP;
            let z = -TERRAIN_SIZE / 2.0 + iz as f32 * STEP;
            h.push(island_height(image, x, z));
        }
    }
    flatten_village(&mut h, n, &village.poly);
    flatten_pads(&mut h, n, pads);
    Field { n, h }
}

/// Impronta di un edificio visitabile: il terreno sotto va spianato (il pavimento è 42 cm sopra la base).
struct BuildingPad {
    center: Vec2,
    half: Vec2,
    yaw: f32,
}

/// Porta il terreno alla quota media dell'impronta, con un raccordo morbido attorno, così che i
/// pavimenti non siano attraversati dal terreno e la porta sia sempre a un gradino da terra.
fn flatten_pads(h: &mut [f32], n: usize, pads: &[BuildingPad]) {
    /// Bordo pieno attorno all'impronta e larghezza del raccordo (m).
    const RIM: f32 = 1.3;
    const BLEND: f32 = 3.5;
    let world = |ix: usize| -TERRAIN_SIZE / 2.0 + ix as f32 * STEP;
    let index = |w: f32| ((w + TERRAIN_SIZE / 2.0) / STEP).round() as isize;
    let clamp = |i: isize| i.clamp(0, n as isize - 1) as usize;
    let rotations: Vec<(f32, f32)> = pads.iter().map(|p| p.yaw.sin_cos()).collect();
    // Quota di ogni spiazzo, dal terreno prima di ogni modifica.
    let targets: Vec<f32> = pads
        .iter()
        .zip(&rotations)
        .map(|(pad, &(sin, cos))| {
            let corners = [
                Vec2::ZERO,
                Vec2::new(pad.half.x, pad.half.y),
                Vec2::new(-pad.half.x, pad.half.y),
                Vec2::new(pad.half.x, -pad.half.y),
                Vec2::new(-pad.half.x, -pad.half.y),
            ];
            corners
                .iter()
                .map(|l| {
                    let p = pad.center + Vec2::new(l.x * cos + l.y * sin, -l.x * sin + l.y * cos);
                    h[clamp(index(p.y)) * n + clamp(index(p.x))]
                })
                .sum::<f32>()
                / corners.len() as f32
        })
        .collect();
    // Peso di ogni spiazzo su ogni vertice; dove due si sovrappongono prevale il più vicino.
    let mut sum = vec![(0.0f32, 0.0f32, 0.0f32); h.len()];
    for ((pad, &(sin, cos)), &target) in pads.iter().zip(&rotations).zip(&targets) {
        let reach = pad.half.length() + RIM + BLEND + 1.0;
        for iz in clamp(index(pad.center.y - reach))..=clamp(index(pad.center.y + reach)) {
            for ix in clamp(index(pad.center.x - reach))..=clamp(index(pad.center.x + reach)) {
                let d = Vec2::new(world(ix), world(iz)) - pad.center;
                let l = Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos);
                let out = (l.abs() - pad.half).max(Vec2::ZERO).length();
                let f = 1.0 - smooth(RIM, RIM + BLEND, out);
                if f > 0.0 {
                    let w = f.powi(4);
                    let cell = &mut sum[iz * n + ix];
                    cell.0 += w;
                    cell.1 += w * target;
                    cell.2 = cell.2.max(f);
                }
            }
        }
    }
    for (value, (weight, weighted, strength)) in h.iter_mut().zip(sum) {
        if weight > 0.0 {
            *value += (weighted / weight - *value) * strength;
        }
    }
}

/// Spiana dolcemente il terreno dentro le mura e per 6 m oltre (media mobile delle quote):
/// strade e case stanno su pendenze gentili.
fn flatten_village(h: &mut [f32], n: usize, poly: &[Vec2]) {
    if poly.len() < 3 {
        return;
    }
    let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    for p in poly {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let margin = 8.0;
    let idx = |w: f32| {
        (((w + TERRAIN_SIZE / 2.0) / STEP).round() as isize).clamp(0, n as isize - 1) as usize
    };
    let (x0, x1, z0, z1) = (
        idx(lo.x - margin),
        idx(hi.x + margin),
        idx(lo.y - margin),
        idx(hi.y + margin),
    );
    let r = 8isize; // 4 m
    let mut a: Vec<f32> = h.to_vec();
    for _ in 0..2 {
        let src = a.clone();
        for z in z0..=z1 {
            for x in x0..=x1 {
                let mut sum = 0.0;
                for k in -r..=r {
                    let xx = (x as isize + k).clamp(0, n as isize - 1) as usize;
                    sum += src[z * n + xx];
                }
                a[z * n + x] = sum / (2 * r + 1) as f32;
            }
        }
        let src = a.clone();
        for z in z0..=z1 {
            for x in x0..=x1 {
                let mut sum = 0.0;
                for k in -r..=r {
                    let zz = (z as isize + k).clamp(0, n as isize - 1) as usize;
                    sum += src[zz * n + x];
                }
                a[z * n + x] = sum / (2 * r + 1) as f32;
            }
        }
    }
    for z in z0..=z1 {
        for x in x0..=x1 {
            let p = Vec2::new(
                -TERRAIN_SIZE / 2.0 + x as f32 * STEP,
                -TERRAIN_SIZE / 2.0 + z as f32 * STEP,
            );
            let d = signed_distance(poly, p);
            let f = 1.0 - smooth(0.0, 6.0, d.max(0.0));
            let i = z * n + x;
            h[i] += (a[i] - h[i]) * 0.9 * f;
        }
    }
}

/// Peso (0..1) del selciato in ogni vertice: 1 sulla strada, bordo irregolare.
fn road_weights(field: &Field, roads: &[Road]) -> Vec<f32> {
    let n = field.n;
    let mut w = vec![0.0f32; n * n];
    let idx = |v: f32| {
        (((v + TERRAIN_SIZE / 2.0) / STEP).round() as isize).clamp(0, n as isize - 1) as usize
    };
    for road in roads {
        let half = road.width / 2.0;
        for seg in road.pts.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let pad = half + 1.0;
            let (x0, x1) = (idx(a.x.min(b.x) - pad), idx(a.x.max(b.x) + pad));
            let (z0, z1) = (idx(a.y.min(b.y) - pad), idx(a.y.max(b.y) + pad));
            for z in z0..=z1 {
                for x in x0..=x1 {
                    let p = Vec2::new(
                        -TERRAIN_SIZE / 2.0 + x as f32 * STEP,
                        -TERRAIN_SIZE / 2.0 + z as f32 * STEP,
                    );
                    let d = dist_to_segment(p, a, b);
                    let nz = (noise2(p.x * 0.7 + 3.0, p.y * 0.7) - 0.5) * 0.5;
                    let v = 1.0 - smooth(half - 0.4 + nz, half + 0.2 + nz, d);
                    let i = z * n + x;
                    if v > w[i] {
                        w[i] = v;
                    }
                }
            }
        }
    }
    w
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
    mesh.compute_normals();
    add_ground_detail(&mut mesh, field);
    mesh
}

/// Tangenti analitiche (per le normal map) e leggere variazioni di colore su larga scala, che
/// nascondono la ripetizione della texture.
fn add_ground_detail(mesh: &mut Mesh, field: &Field) {
    let n = field.n;
    let tangents: Vec<[f32; 4]> = (0..n * n)
        .map(|i| {
            let x = i % n;
            let z = i / n;
            let left = field.h[z * n + x.saturating_sub(1)];
            let right = field.h[z * n + (x + 1).min(n - 1)];
            let t = Vec3::new(2.0 * STEP, right - left, 0.0).normalize();
            [t.x, t.y, t.z, TANGENT_W]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    let colors: Vec<[f32; 4]> = (0..n * n)
        .map(|i| {
            let (x, z) = ((i % n) as f32 * STEP, (i / n) as f32 * STEP);
            let low = (x * 0.041).sin() * (z * 0.033).cos() + (x * 0.013 + z * 0.021).sin();
            let high = (x * 0.31 + 1.3).sin() * (z * 0.27).cos();
            let k = 1.0 + low * 0.12 + high * 0.04;
            [k * 0.97, k * (1.04 + 0.05 * high), k * 0.92, 1.0]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
}

/// Collider heightfield Avian dalla stessa griglia: heights[x][z] (riga = X, colonna = Z).
fn build_terrain_collider(field: &Field) -> Collider {
    let n = field.n;
    let heights: Vec<Vec<f32>> = (0..n)
        .map(|x| (0..n).map(|z| field.at(x, z)).collect())
        .collect();
    Collider::heightfield(heights, Vec3::new(TERRAIN_SIZE, 1.0, TERRAIN_SIZE))
}

/// Celle collegate al mare: flood-fill 4-connesso dal bordo sulle quote sotto `thr`.
fn flood_ocean(field: &Field, thr: f32) -> Vec<bool> {
    let n = field.n;
    let mut mask = vec![false; n * n];
    let mut stack: Vec<usize> = Vec::new();
    for i in 0..n {
        for idx in [i, (n - 1) * n + i, i * n, i * n + n - 1] {
            if field.h[idx] < thr && !mask[idx] {
                mask[idx] = true;
                stack.push(idx);
            }
        }
    }
    while let Some(i) = stack.pop() {
        let (x, z) = (i % n, i / n);
        let mut push = |j: usize| {
            if field.h[j] < thr && !mask[j] {
                mask[j] = true;
                stack.push(j);
            }
        };
        if x > 0 {
            push(i - 1);
        }
        if x + 1 < n {
            push(i + 1);
        }
        if z > 0 {
            push(i - n);
        }
        if z + 1 < n {
            push(i + n);
        }
    }
    mask
}

/// Peso (0..1) della sabbia in ogni vertice: 1 vicino al mare, sfuma nel prato.
fn sand_weights(field: &Field, ocean: &[bool]) -> Vec<f32> {
    let n = field.n;
    let max_d = 40u16; // celle (20 m)
    let mut dist = vec![u16::MAX; n * n];
    let mut queue = std::collections::VecDeque::new();
    for (i, o) in ocean.iter().enumerate() {
        if *o {
            dist[i] = 0;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let d = dist[i];
        if d >= max_d {
            continue;
        }
        let (x, z) = (i % n, i / n);
        let mut visit = |j: usize| {
            if dist[j] == u16::MAX {
                dist[j] = d + 1;
                queue.push_back(j);
            }
        };
        if x > 0 {
            visit(i - 1);
        }
        if x + 1 < n {
            visit(i + 1);
        }
        if z > 0 {
            visit(i - n);
        }
        if z + 1 < n {
            visit(i + n);
        }
    }
    (0..n * n)
        .map(|i| {
            if dist[i] == u16::MAX {
                return 0.0;
            }
            let (x, z) = (
                -TERRAIN_SIZE / 2.0 + (i % n) as f32 * STEP,
                -TERRAIN_SIZE / 2.0 + (i / n) as f32 * STEP,
            );
            // Bordo irregolare: la distanza è modulata da rumore.
            let d = dist[i] as f32 * STEP + (noise2(x * 0.09 + 5.0, z * 0.09 + 9.0) - 0.5) * 7.0;
            let by_dist = 1.0 - smooth(5.0, 15.0, d);
            let by_height = 1.0 - smooth(SEA_LEVEL + 2.5, SEA_LEVEL + 4.5, field.h[i]);
            by_dist * by_height
        })
        .collect()
}

/// Peso della neve per vertice: a chiazze, non sul mare, ridotta su sabbia e strade.
fn snow_weights(field: &Field, ocean: &[bool], sand_w: &[f32], road_w: &[f32]) -> Vec<f32> {
    let n = field.n;
    (0..n * n)
        .map(|i| {
            if ocean[i] {
                return 0.0;
            }
            let (x, z) = ((i % n) as f32, (i / n) as f32);
            let noise = 0.85 + 0.15 * (x * 0.37).sin() * (z * 0.29).cos();
            (noise * (1.0 - sand_w[i]) * (1.0 - 0.7 * road_w[i])).clamp(0.0, 1.0)
        })
        .collect()
}

/// Mesh della sabbia: gli stessi vertici del terreno, solo i triangoli con sabbia, alfa = peso.
fn build_sand_mesh(terrain: &Mesh, weights: &[f32]) -> Mesh {
    let mut mesh = terrain.clone();
    let colors: Vec<[f32; 4]> = weights.iter().map(|w| [1.0, 1.0, 1.0, *w]).collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    if let Some(Indices::U32(idx)) = terrain.indices() {
        let mut keep: Vec<u32> = Vec::new();
        for t in idx.as_chunks::<3>().0 {
            if t.iter().any(|v| weights[*v as usize] > 0.02) {
                keep.extend_from_slice(&t[..]);
            }
        }
        mesh.insert_indices(Indices::U32(keep));
    }
    mesh
}

/// Mesh del mare (y = 0 locale): solo i blocchi 1×1 m collegati all'oceano, uniti per
/// righe, più una cornice enorme fuori dal terreno.
fn build_sea_mesh(field: &Field, ocean: &[bool]) -> Mesh {
    let n = field.n;
    let blocks = (n - 1) / 2;
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    let mut quad = |x0: f32, z0: f32, x1: f32, z1: f32| {
        let b = pos.len() as u32;
        pos.extend([[x0, 0.0, z0], [x1, 0.0, z0], [x1, 0.0, z1], [x0, 0.0, z1]]);
        idx.extend([b, b + 2, b + 1, b, b + 3, b + 2]);
    };
    let w = |i: usize| -TERRAIN_SIZE / 2.0 + i as f32 * 2.0 * STEP;
    for bz in 0..blocks {
        let mut run: Option<usize> = None;
        for bx in 0..=blocks {
            let sea = bx < blocks
                && (0..3).any(|dz| (0..3).any(|dx| ocean[(2 * bz + dz) * n + 2 * bx + dx]));
            match (sea, run) {
                (true, None) => run = Some(bx),
                (false, Some(r0)) => {
                    quad(w(r0), w(bz), w(bx), w(bz + 1));
                    run = None;
                }
                _ => {}
            }
        }
    }
    let (h, big) = (TERRAIN_SIZE / 2.0, 4000.0);
    quad(-big, -big, big, -h);
    quad(-big, h, big, big);
    quad(-big, -h, -h, h);
    quad(h, -h, big, h);
    let count = pos.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count]);
    mesh.insert_indices(Indices::U32(idx));
    mesh
}

/// Barriera di riva: heightfield con +12 m sul mare (soglia `SHORE_LEVEL`) e −40 m altrove.
fn build_barrier(field: &Field, shore_mask: &[bool], piers: &[(Vec2, f32)]) -> Collider {
    let n = field.n;
    // Dentro l'impronta di un molo (x ±1,6 m, z da −2,5 a +12,5 m locali) il muro cade.
    let on_pier = |x: usize, z: usize| {
        let p = Vec2::new(
            -TERRAIN_SIZE / 2.0 + x as f32 * STEP,
            -TERRAIN_SIZE / 2.0 + z as f32 * STEP,
        );
        piers.iter().any(|(c, yaw)| {
            let d = p - *c;
            let (s, co) = yaw.sin_cos();
            // coordinate locali: +Z locale = (sin yaw, cos yaw), +X locale = (cos yaw, -sin yaw)
            let lz = d.x * s + d.y * co;
            let lx = d.x * co - d.y * s;
            lx.abs() < 1.6 && (-2.5..12.5).contains(&lz)
        })
    };
    let heights: Vec<Vec<f32>> = (0..n)
        .map(|x| {
            (0..n)
                .map(|z| {
                    if shore_mask[z * n + x] && !on_pier(x, z) {
                        12.0
                    } else {
                        -40.0
                    }
                })
                .collect()
        })
        .collect();
    Collider::heightfield(heights, Vec3::new(TERRAIN_SIZE, 1.0, TERRAIN_SIZE))
}

/// Punti della linea di riva (per i paletti del debug): metà tra una cella di mare e una di terra.
fn shore_points(field: &Field, mask: &[bool]) -> Vec<Vec2> {
    let n = field.n;
    let w = |i: usize| -TERRAIN_SIZE / 2.0 + i as f32 * STEP;
    let mut v = Vec::new();
    for z in 0..n - 1 {
        for x in 0..n - 1 {
            let m = mask[z * n + x];
            if m != mask[z * n + x + 1] {
                v.push(Vec2::new(w(x) + STEP / 2.0, w(z)));
            }
            if m != mask[(z + 1) * n + x] {
                v.push(Vec2::new(w(x), w(z) + STEP / 2.0));
            }
        }
    }
    v
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::{Mesh, Meshable, VertexAttributeValues};

    /// Le tangenti analitiche devono avere lo stesso segno di quelle generate da mikktspace.
    #[test]
    fn analytic_tangent_sign_matches_mikktspace() {
        let mut m = Plane3d {
            half_size: Vec2::splat(1.0),
            ..Default::default()
        }
        .mesh()
        .build();
        m.generate_tangents().unwrap();
        let Some(VertexAttributeValues::Float32x4(t)) = m.attribute(Mesh::ATTRIBUTE_TANGENT) else {
            panic!("senza tangenti");
        };
        assert!(t[0][0].abs() > 0.9, "tangente lungo X: {:?}", t[0]);
        assert_eq!(t[0][3].signum(), TANGENT_W, "{:?}", t[0]);
    }
}
