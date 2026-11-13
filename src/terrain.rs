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
pub fn size() -> f32 {
    crate::region::current().size()
}
/// Lato (m) del quadrato coperto dalla heightmap PNG (centrato).
const HEIGHTMAP_SIZE: f32 = 100.0;
/// Passo (m) della griglia di vertici (dipende dalla regione).
pub(crate) fn step() -> f32 {
    crate::region::current().step()
}
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

/// Materiale del suolo, per i passi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Grass,
    Dirt,
    Sand,
    Stone,
    Wood,
    Water,
    Snow,
    /// Campo di grano o orto: gli steli sfregano contro le gambe.
    Crops,
}

impl Surface {
    pub const ALL: [Surface; 8] = [
        Surface::Grass,
        Surface::Dirt,
        Surface::Sand,
        Surface::Stone,
        Surface::Wood,
        Surface::Water,
        Surface::Snow,
        Surface::Crops,
    ];
}

/// Campo di altezze del terreno (dopo la generazione) e punti della riva per il debug F3.
/// Serve ad appoggiare gli oggetti a terra senza dipendere dalla fisica.
#[derive(Resource)]
pub struct TerrainField {
    n: usize,
    h: Vec<f32>,
    shore: Vec<Vec2>,
    /// Vertici coperti da ruscello e laghetto.
    wet: Vec<bool>,
    /// Materiale del suolo per vertice.
    surface: Vec<Surface>,
    /// Punti radi di ruscello e laghetto, per la distanza dall'acqua.
    wet_points: Vec<Vec2>,
    /// Laghetto: centro, raggio e quota dell'acqua.
    lake: Option<(Vec2, f32, f32)>,
    /// Buchi nel terreno (imbocchi sotterranei), vedi [`TerrainHole`].
    holes: Vec<TerrainHole>,
}

/// Rettangolo in cui il terreno non c'è: niente mesh e il giocatore ne ignora il collider
/// (un heightfield non può avere cavità). Serve all'imbocco delle gallerie della grotta.
#[derive(Clone, Debug)]
pub struct TerrainHole {
    center: Vec2,
    yaw: f32,
    min: Vec2,
    max: Vec2,
}

impl TerrainHole {
    /// Il punto (x, z) del mondo cade nel buco?
    pub fn contains(&self, p: Vec2) -> bool {
        let (sin, cos) = self.yaw.sin_cos();
        let d = p - self.center;
        let l = Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos);
        l.x >= self.min.x && l.x <= self.max.x && l.y >= self.min.y && l.y <= self.max.y
    }
}

impl TerrainField {
    /// Il punto è in un buco del terreno (imbocco sotterraneo)?
    pub fn in_hole(&self, x: f32, z: f32) -> bool {
        self.holes.iter().any(|h| h.contains(Vec2::new(x, z)))
    }

    /// Quota (m) del terreno in (x, z), interpolazione bilineare tra i vertici.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let max = (self.n - 1) as f32;
        let fx = ((x + size() / 2.0) / step()).clamp(0.0, max);
        let fz = ((z + size() / 2.0) / step()).clamp(0.0, max);
        let (x0, z0) = (fx.floor() as usize, fz.floor() as usize);
        let (x1, z1) = ((x0 + 1).min(self.n - 1), (z0 + 1).min(self.n - 1));
        let (u, v) = (fx - x0 as f32, fz - z0 as f32);
        let at = |x: usize, z: usize| self.h[z * self.n + x];
        let top = at(x0, z0) * (1.0 - u) + at(x1, z0) * u;
        let bot = at(x0, z1) * (1.0 - u) + at(x1, z1) * u;
        top * (1.0 - v) + bot * v
    }

    /// Materiale del suolo nel vertice più vicino a (x, z).
    pub fn surface_at(&self, x: f32, z: f32) -> Surface {
        let i = |w: f32| ((w + size() / 2.0) / step()).round() as isize;
        let (ix, iz) = (i(x), i(z));
        let n = self.n as isize;
        if (0..n).contains(&ix) && (0..n).contains(&iz) {
            self.surface[(iz * n + ix) as usize]
        } else {
            Surface::Sand
        }
    }

    /// Laghetto: centro, raggio e quota dell'acqua.
    pub fn lake(&self) -> Option<(Vec2, f32, f32)> {
        self.lake
    }

    /// Distanza (m) dal punto più vicino della riva.
    pub fn distance_to_shore(&self, p: Vec2) -> f32 {
        nearest(&self.shore, p)
    }

    /// Distanza (m) da ruscello e laghetto.
    pub fn distance_to_fresh_water(&self, p: Vec2) -> f32 {
        nearest(&self.wet_points, p)
    }

    /// C'è acqua dolce in (x, z)?
    pub fn in_fresh_water(&self, x: f32, z: f32) -> bool {
        let i = |w: f32| ((w + size() / 2.0) / step()).round() as isize;
        let (ix, iz) = (i(x), i(z));
        let n = self.n as isize;
        (0..n).contains(&ix) && (0..n).contains(&iz) && self.wet[(iz * n + ix) as usize]
    }
}

fn nearest(points: &[Vec2], p: Vec2) -> f32 {
    points
        .iter()
        .map(|q| q.distance_squared(p))
        .fold(f32::MAX, f32::min)
        .sqrt()
}

/// Lato (m) di una piastrella della texture sabbia e del selciato.
const SAND_TILE_M: f32 = 12.0;
const ROAD_TILE_M: f32 = 2.4;
const ROCK_TILE_M: f32 = 6.0;

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
        half_size: Vec2::splat(size() / 2.0),
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
        Collider::cuboid(size(), 1.0, size()),
        Mesh3d(placeholder_mesh),
        MeshMaterial3d(materials.add(crate::pbr::material(
            asset_server,
            "grass",
            Vec2::splat(size() / GRASS_TILE_M),
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut events: MessageReader<AssetEvent<Image>>,
    terrains: Query<(Entity, &Mesh3d), With<TerrainMesh>>,
    extras: Query<Entity, With<TerrainExtra>>,
    village: Option<Res<VillageInfo>>,
    dbg: Option<Res<crate::debug_place::DebugPlace>>,
    pbr: Res<crate::pbr::PbrMaterials>,
    mut images: ResMut<Assets<Image>>,
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
                // Il terreno della casa del giocatore è piano fin dall'inizio (ADR 0042).
                .chain(
                    (crate::region::current() == crate::region::Region::Isola).then_some(
                        BuildingPad {
                            center: crate::house::PLOT,
                            half: crate::house::HALF,
                            yaw: 0.0,
                        },
                    ),
                )
                .collect()
        })
        .unwrap_or_default();
    let island = crate::water::load_spec();
    let (field, water) = build_field(image, &vil, &pads, &island);
    let holes: Vec<TerrainHole> = dbg
        .as_ref()
        .map(|d| {
            d.items
                .iter()
                .filter(|p| p.kind == "grotta")
                .flat_map(|p| {
                    p.points.iter().enumerate().map(move |(j, q)| TerrainHole {
                        center: *q,
                        yaw: p.yaw(j),
                        min: crate::places::CAVE_HOLE.0,
                        max: crate::places::CAVE_HOLE.1,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let mut mesh = build_terrain_mesh(&field);
    cut_holes(&mut mesh, &holes);
    let collider = build_terrain_collider(&field);
    let n = field.n;

    // Log diagnostico: range delle altezze.
    let (mut h_min, mut h_max) = (f32::INFINITY, f32::NEG_INFINITY);
    for h in &field.h {
        h_min = h_min.min(*h);
        h_max = h_max.max(*h);
    }
    info!(
        "terrain diagnostics: {} {n}x{n} vertices, height range [{h_min:.2}, {h_max:.2}] m, size {} m",
        crate::region::current().key(),
        size()
    );

    // Mare solo attorno all'isola (flood-fill dal bordo), sabbia sfumata, barriera di riva.
    let ocean = flood_ocean(&field, SEA_LEVEL);
    let shore_mask = flood_ocean(&field, SHORE_LEVEL);
    let sand_w = sand_weights(&field, &ocean);
    let sand_mesh = build_sand_mesh(&mesh, &sand_w);
    let (paved, paths): (Vec<Road>, Vec<Road>) =
        vil.spec.roads.iter().cloned().partition(|r| !r.dirt);
    let road_w = road_weights(&field, &paved);
    let road_mesh = build_sand_mesh(&mesh, &road_w);
    let dirt_w = road_weights(&field, &paths);
    let dirt_mesh = build_sand_mesh(&mesh, &dirt_w);
    let rock_w = rock_weights(&field);
    let rock_mesh = build_sand_mesh(&mesh, &rock_w);
    let foam_mesh = build_foam_mesh(&mesh, &field, &ocean);
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
    let fence = cliff_fence(&field, &island);
    let barrier = build_barrier(&field, &shore_mask, &fence, &piers);
    let shore = shore_points(&field, &shore_mask);
    info!(
        "[TERRAIN] riva: {} punti; sabbia: {} vertici",
        shore.len(),
        sand_w.iter().filter(|w| **w > 0.02).count()
    );
    let surface = (0..field.h.len())
        .map(|i| {
            if road_w[i] > 0.4 || rock_w[i] > 0.5 {
                Surface::Stone
            } else if dirt_w[i] > 0.4 {
                Surface::Dirt
            } else if sand_w[i] > 0.5 {
                Surface::Sand
            } else {
                Surface::Grass
            }
        })
        .collect();
    // Un punto ogni 2,5 m basta per sapere quanto è vicina l'acqua.
    let wet_points = (0..n)
        .step_by(5)
        .flat_map(|z| (0..n).step_by(5).map(move |x| (x, z)))
        .filter(|(x, z)| water.1[z * n + x])
        .map(|(x, z)| {
            Vec2::new(
                x as f32 * step() - size() / 2.0,
                z as f32 * step() - size() / 2.0,
            )
        })
        .collect();
    commands.insert_resource(TerrainField {
        holes,
        n,
        h: field.h.clone(),
        shore,
        wet: water.1,
        surface,
        wet_points,
        lake: water.0.lake,
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
            ..crate::pbr::material(&asset_server, "sand", Vec2::splat(size() / SAND_TILE_M))
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
            ..crate::pbr::material(&asset_server, "cobble", Vec2::splat(size() / ROAD_TILE_M))
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
    // Sentieri di terra battuta e roccia sui pendii ripidi, sopra l'erba.
    commands.spawn((
        SceneNode,
        TerrainExtra,
        Mesh3d(meshes.add(dirt_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.62, 0.48, 0.34),
            alpha_mode: AlphaMode::Blend,
            depth_bias: 1.2,
            ..crate::pbr::material(&asset_server, "sand", Vec2::splat(size() / SAND_TILE_M))
        })),
        Transform::default(),
        bevy::light::NotShadowCaster,
    ));
    commands.spawn((
        SceneNode,
        TerrainExtra,
        Mesh3d(meshes.add(rock_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            depth_bias: 0.8,
            ..crate::pbr::material(&asset_server, "stone", Vec2::splat(size() / ROCK_TILE_M))
        })),
        Transform::default(),
        bevy::light::NotShadowCaster,
    ));
    // Mare, ruscello e schiuma condividono la stessa mappa delle onde.
    let waves = images.add(crate::water::wave_normals(256));
    commands.spawn((
        SceneNode,
        TerrainExtra,
        crate::water::SeaSurface,
        Mesh3d(meshes.add(sea_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.05, 0.22, 0.33, 0.86),
            reflectance: 0.6,
            alpha_mode: AlphaMode::Blend,
            // Il mare si disegna dopo sabbia e strade (anche dall'alto, nella mappa).
            depth_bias: 4.0,
            perceptual_roughness: 0.2,
            metallic: 0.0,
            normal_map_texture: Some(waves.clone()),
            ..Default::default()
        })),
        Transform::from_xyz(0.0, SEA_LEVEL, 0.0),
        bevy::light::NotShadowCaster,
    ));
    commands.spawn((
        SceneNode,
        TerrainExtra,
        crate::water::StreamSurface,
        Mesh3d(meshes.add(crate::water::water_mesh(&water.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.1, 0.3, 0.36, 0.78),
            // Liscia e opaca di luce: con superficie lucida e normali fitte il sole fa brillantini.
            reflectance: 0.25,
            alpha_mode: AlphaMode::Blend,
            depth_bias: 3.0,
            perceptual_roughness: 0.5,
            normal_map_texture: Some(waves),
            ..Default::default()
        })),
        Transform::default(),
        bevy::light::NotShadowCaster,
    ));
    commands.spawn((
        SceneNode,
        TerrainExtra,
        crate::water::ShoreFoam,
        Mesh3d(meshes.add(foam_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.95, 0.97, 1.0, 0.6),
            alpha_mode: AlphaMode::Blend,
            depth_bias: 5.0,
            perceptual_roughness: 0.6,
            unlit: false,
            ..Default::default()
        })),
        Transform::default(),
        Visibility::default(),
        bevy::light::NotShadowCaster,
    ));
    // Ponti sul ruscello: tavolato arcuato da sponda a sponda.
    let planks = pbr.get(crate::pbr::Mat::Planks);
    let ground_at = |p: Vec2| field_height(&field, p);
    for (a, b) in &island.bridges {
        let (a3, b3) = (
            Vec3::new(a.x, ground_at(*a) + 0.1, a.y),
            Vec3::new(b.x, ground_at(*b) + 0.1, b.y),
        );
        let root = commands
            .spawn((
                SceneNode,
                TerrainExtra,
                RigidBody::Static,
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        for (center, rotation, length) in crate::water::bridge_segments(a3, b3, 8) {
            let deck = meshes.add(crate::pbr::with_tangents(Cuboid::new(
                1.8,
                0.12,
                length + 0.05,
            )));
            let rail = meshes.add(crate::pbr::with_tangents(Cuboid::new(
                0.08,
                0.08,
                length + 0.05,
            )));
            let post = meshes.add(crate::pbr::with_tangents(Cuboid::new(0.09, 0.8, 0.09)));
            commands.entity(root).with_children(|c| {
                c.spawn((
                    Ground,
                    Mesh3d(deck),
                    MeshMaterial3d(planks.clone()),
                    Collider::cuboid(1.8, 0.12, length + 0.05),
                    Transform::from_translation(center).with_rotation(rotation),
                ));
                for side in [-0.85f32, 0.85] {
                    let across = rotation * Vec3::X * side;
                    c.spawn((
                        Mesh3d(rail.clone()),
                        MeshMaterial3d(planks.clone()),
                        Transform::from_translation(center + across + Vec3::Y * 0.8)
                            .with_rotation(rotation),
                    ));
                    c.spawn((
                        Mesh3d(post.clone()),
                        MeshMaterial3d(planks.clone()),
                        Transform::from_translation(center + across + Vec3::Y * 0.4),
                    ));
                }
            });
        }
    }
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
    let world = |i: usize| -size() / 2.0 + i as f32 * step();
    let idx = |w: f32| (((w + size() / 2.0) / step()).round().max(0.0) as usize).min(n - 1);
    let (cx, cz) = (idx(pos.x), idx(pos.z));
    let r = (35.0 / step()) as usize;
    let skip = (2.0 / step()) as usize; // passo 2 m
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
pub(crate) fn noise2(x: f32, z: f32) -> f32 {
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
    if crate::region::current() != crate::region::Region::Isola {
        // Terraferma: mare a sud, lago e rio (dati della regione, letti una volta per regione).
        return continent_water(&crate::water::load_spec_cached(), Vec2::new(x, z));
    }
    if x.abs() <= HEIGHTMAP_SIZE / 2.0 && z.abs() <= HEIGHTMAP_SIZE / 2.0 {
        return false;
    }
    let m = (x * x + z * z).sqrt() / island_radius(z.atan2(x));
    m > 0.9
}

/// Quanto un punto è dentro il grande lago: 0 al centro, 1 sul bordo (frastagliato).
fn basin_m(island: &crate::water::IslandSpec, p: Vec2) -> Option<f32> {
    let (c, r) = island.basin?;
    let d = (p - c) / r;
    let edge = 1.0 + (noise2(p.x * 0.03 + 2.0, p.y * 0.03 + 8.0) - 0.5) * 0.28;
    Some(d.length() / edge)
}

/// Distanza (m) dall'asse del canale (il rio).
fn channel_dist(island: &crate::water::IslandSpec, p: Vec2) -> f32 {
    island
        .channel
        .windows(2)
        .map(|s| dist_to_segment(p, s[0], s[1]))
        .fold(f32::MAX, f32::min)
}

/// z della riva sud della terraferma all'ascissa `x` (costa ondulata).
fn coast_z(coast: f32, x: f32) -> f32 {
    // Davanti all'approdo (x = 0) la riva è dritta, così molo e sentiero cadono giusti.
    coast + (noise2(x * 0.02 + 3.0, 1.5) - 0.5) * 22.0 * smooth(20.0, 90.0, x.abs())
}

/// Acqua della terraferma: mare a sud, lago (fuori dall'isolotto) e rio.
fn continent_water(island: &crate::water::IslandSpec, p: Vec2) -> bool {
    let Some(coast) = island.coast else {
        return false;
    };
    let on_islet = island.islet.is_some_and(|(c, r)| p.distance(c) < r);
    p.y > coast_z(coast, p.x) - 6.0
        || (basin_m(island, p).is_some_and(|m| m < 0.97) && !on_islet)
        || channel_dist(island, p) < island.channel_half + 1.5
}

/// Altezza (m) della terraferma in (x, z): piana ondulata, mare a sud, lago e rio a livello
/// del mare (li allaga lo stesso riempimento del mare), rilievi oltre il confine.
fn continent_height(x: f32, z: f32, island: &crate::water::IslandSpec, poly: &[Vec2]) -> f32 {
    let p = Vec2::new(x, z);
    let coast = island.coast.unwrap_or(f32::MAX);
    let mut land = 2.3
        + (noise2(x * 0.012 + 5.0, z * 0.012 + 9.0) - 0.5) * 2.6
        + (noise2(x * 0.05, z * 0.05 + 40.0) - 0.5) * 0.9;
    // Alture: mai dentro il paese né vicino alle mura.
    let hills = island.hill_height(p);
    let keep_out = if hills.abs() > 0.01 && poly.len() >= 3 {
        smooth(8.0, 22.0, signed_distance(poly, p).max(0.0))
    } else {
        1.0
    };
    land += hills * keep_out;
    // Il paese sta in piano, a una quota sola, con un raccordo dolce fuori dalle mura.
    if poly.len() >= 3 {
        let d = signed_distance(poly, p);
        land += (2.4 - land) * smooth(26.0, 6.0, d);
    }
    // Confine: oltre il limite percorribile il terreno sale in un crinale.
    if let Some((xb, zb)) = island.bounds {
        let out = (x.abs() - xb).max(-zb - z).max(0.0);
        land += smooth(0.0, 70.0, out) * 26.0;
    }
    // Lago: sponda dolce e fondo sotto il livello del mare.
    if let Some(m) = basin_m(island, p) {
        land += (-3.4 - land) * smooth(1.12, 0.82, m);
    }
    if let Some((c, r)) = island.islet {
        land += (1.9 - land) * smooth(1.0, 0.5, p.distance(c) / r);
    }
    // Rio: canale sotto il livello del mare, con le sponde raccordate.
    if island.channel.len() >= 2 {
        let d = channel_dist(island, p);
        let half = island.channel_half;
        land += (-2.6 - land) * smooth(half + 4.0, half, d);
    }
    // Mare a sud.
    let cz = coast_z(coast, x);
    land + (-3.6 - land) * smooth(cz - 12.0, cz + 8.0, z)
}

/// Altezza (m) del terreno in (x, z).
fn island_height(
    image: &Image,
    x: f32,
    z: f32,
    island: &crate::water::IslandSpec,
    poly: &[Vec2],
) -> f32 {
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

    // Alture: mai dentro il paese né vicino alle mura.
    let p = Vec2::new(x, z);
    let hills = island.hill_height(p);
    let keep_out = if hills.abs() > 0.01 && poly.len() >= 3 {
        smooth(8.0, 22.0, signed_distance(poly, p).max(0.0))
    } else {
        1.0
    };
    let land = land + hills * keep_out;

    // Costa: la terra scende nel mare verso il bordo irregolare dell'isola; a tratti è una scogliera.
    let angle = z.atan2(x);
    let m = (x * x + z * z).sqrt() / island_radius(angle);
    let (cliff, height) = island.cliff_at(angle);
    let land = land + height * cliff * smooth(0.66, 0.84, m);
    let start = 0.72 + (0.94 - 0.72) * cliff;
    let end = 1.0 + (0.995 - 1.0) * cliff;
    land + (-3.5 - land) * smooth(start, end, m)
}

fn build_field(
    image: &Image,
    village: &VillageInfo,
    pads: &[BuildingPad],
    island: &crate::water::IslandSpec,
) -> (Field, (crate::water::WaterPlan, Vec<bool>)) {
    let n = (size() / step()) as usize + 1;
    let mut h = Vec::with_capacity(n * n);
    for iz in 0..n {
        for ix in 0..n {
            let x = -size() / 2.0 + ix as f32 * step();
            let z = -size() / 2.0 + iz as f32 * step();
            h.push(if island.coast.is_some() {
                continent_height(x, z, island, &village.poly)
            } else {
                island_height(image, x, z, island, &village.poly)
            });
        }
    }
    flatten_village(&mut h, n, &village.poly);
    // Il ruscello scende sul terreno già definitivo e scava solo fuori dal paese.
    let field = Field { n, h };
    let plan = crate::water::plan_water(island, |p| field_height(&field, p));
    let Field { n, mut h } = field;
    let wet = crate::water::carve(&plan, &mut h, n, step());
    flatten_pads(&mut h, n, pads);
    (Field { n, h }, (plan, wet))
}

/// Quota del campo in (x, z), per il vertice più vicino.
fn field_height(field: &Field, p: Vec2) -> f32 {
    let i = |w: f32| {
        (((w + size() / 2.0) / step()).round() as isize).clamp(0, field.n as isize - 1) as usize
    };
    field.at(i(p.x), i(p.y))
}

/// Peso della roccia: dove il terreno è ripido.
fn rock_weights(field: &Field) -> Vec<f32> {
    let n = field.n;
    (0..n * n)
        .map(|i| {
            let (x, z) = (i % n, i / n);
            let dx = field.at((x + 1).min(n - 1), z) - field.at(x.saturating_sub(1), z);
            let dz = field.at(x, (z + 1).min(n - 1)) - field.at(x, z.saturating_sub(1));
            let slope = (dx * dx + dz * dz).sqrt() / (2.0 * step());
            smooth(0.55, 0.95, slope)
        })
        .collect()
}

/// Oltre il ciglio delle scogliere non si passa: nessuno resta incastrato ai piedi della parete.
fn cliff_fence(field: &Field, island: &crate::water::IslandSpec) -> Vec<bool> {
    let n = field.n;
    (0..n * n)
        .map(|i| {
            let x = -size() / 2.0 + (i % n) as f32 * step();
            let z = -size() / 2.0 + (i / n) as f32 * step();
            // Terraferma: il recinto è il confine percorribile (a ovest, a est e a nord).
            if island.coast.is_some() {
                return island
                    .bounds
                    .is_some_and(|(xb, zb)| x.abs() > xb + 6.0 || z < -zb - 6.0);
            }
            let angle = z.atan2(x);
            let m = (x * x + z * z).sqrt() / island_radius(angle);
            island.cliff_at(angle).0 > 0.5 && m > 0.925
        })
        .collect()
}

/// Schiuma: una striscia sul mare lungo la riva, alta sul livello dell'acqua.
fn build_foam_mesh(terrain: &Mesh, field: &Field, ocean: &[bool]) -> Mesh {
    let n = field.n;
    let weights: Vec<f32> = (0..n * n)
        .map(|i| {
            if !ocean[i] {
                return 0.0;
            }
            let (x, z) = (i % n, i / n);
            let near_land = (x.saturating_sub(3)..=(x + 3).min(n - 1))
                .any(|xx| (z.saturating_sub(3)..=(z + 3).min(n - 1)).any(|zz| !ocean[zz * n + xx]));
            if !near_land {
                return 0.0;
            }
            0.5 + 0.5 * noise2(x as f32 * 0.45, z as f32 * 0.45)
        })
        .collect();
    let mut mesh = build_sand_mesh(terrain, &weights);
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for p in positions.iter_mut() {
            p[1] = SEA_LEVEL + 0.03;
        }
    }
    mesh
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
    let world = |ix: usize| -size() / 2.0 + ix as f32 * step();
    let index = |w: f32| ((w + size() / 2.0) / step()).round() as isize;
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
    let idx =
        |w: f32| (((w + size() / 2.0) / step()).round() as isize).clamp(0, n as isize - 1) as usize;
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
                -size() / 2.0 + x as f32 * step(),
                -size() / 2.0 + z as f32 * step(),
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
    let idx =
        |v: f32| (((v + size() / 2.0) / step()).round() as isize).clamp(0, n as isize - 1) as usize;
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
                        -size() / 2.0 + x as f32 * step(),
                        -size() / 2.0 + z as f32 * step(),
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
        half_size: Vec2::splat(size() / 2.0),
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
            let t = Vec3::new(2.0 * step(), right - left, 0.0).normalize();
            [t.x, t.y, t.z, TANGENT_W]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    let colors: Vec<[f32; 4]> = (0..n * n)
        .map(|i| {
            let (x, z) = ((i % n) as f32 * step(), (i / n) as f32 * step());
            let low = (x * 0.041).sin() * (z * 0.033).cos() + (x * 0.013 + z * 0.021).sin();
            let high = (x * 0.31 + 1.3).sin() * (z * 0.27).cos();
            let k = 1.0 + low * 0.12 + high * 0.04;
            [k * 0.97, k * (1.04 + 0.05 * high), k * 0.92, 1.0]
        })
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
}

/// Toglie dalla mesh i triangoli che toccano un buco.
fn cut_holes(mesh: &mut Mesh, holes: &[TerrainHole]) {
    if holes.is_empty() {
        return;
    }
    let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    let inside: Vec<bool> = pos
        .iter()
        .map(|p| holes.iter().any(|h| h.contains(Vec2::new(p[0], p[2]))))
        .collect();
    if let Some(Indices::U32(idx)) = mesh.indices() {
        let keep: Vec<u32> = idx
            .as_chunks::<3>()
            .0
            .iter()
            .filter(|t| !t.iter().any(|v| inside[*v as usize]))
            .flatten()
            .copied()
            .collect();
        mesh.insert_indices(Indices::U32(keep));
    }
}

/// Collider heightfield Avian dalla stessa griglia: heights[x][z] (riga = X, colonna = Z).
fn build_terrain_collider(field: &Field) -> Collider {
    let n = field.n;
    let heights: Vec<Vec<f32>> = (0..n)
        .map(|x| (0..n).map(|z| field.at(x, z)).collect())
        .collect();
    Collider::heightfield(heights, Vec3::new(size(), 1.0, size()))
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
    let max_d = 56u16; // celle (28 m)
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
                -size() / 2.0 + (i % n) as f32 * step(),
                -size() / 2.0 + (i / n) as f32 * step(),
            );
            // Bordo irregolare: la distanza è modulata da rumore.
            let d = dist[i] as f32 * step() + (noise2(x * 0.09 + 5.0, z * 0.09 + 9.0) - 0.5) * 7.0;
            let by_dist = 1.0 - smooth(9.0, 23.0, d);
            let by_height = 1.0 - smooth(SEA_LEVEL + 3.0, SEA_LEVEL + 5.5, field.h[i]);
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
    let w = |i: usize| -size() / 2.0 + i as f32 * 2.0 * step();
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
    let (h, big) = (size() / 2.0, 4000.0);
    quad(-big, -big, big, -h);
    quad(-big, h, big, big);
    quad(-big, -h, -h, h);
    quad(h, -h, big, h);
    let count = pos.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    // UV in metri (una piastrella di onde ogni 16 m) e tangenti per la mappa delle normali.
    let uv: Vec<[f32; 2]> = pos.iter().map(|p| [p[0] / 16.0, p[2] / 16.0]).collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_TANGENT,
        vec![[1.0f32, 0.0, 0.0, 1.0]; count],
    );
    mesh.insert_indices(Indices::U32(idx));
    mesh
}

/// Barriera di riva: heightfield con +12 m sul mare (soglia `SHORE_LEVEL`) e −40 m altrove.
fn build_barrier(
    field: &Field,
    shore_mask: &[bool],
    fence: &[bool],
    piers: &[(Vec2, f32)],
) -> Collider {
    let n = field.n;
    // Dentro l'impronta di un molo (x ±1,6 m, z da −2,5 a +12,5 m locali) il muro cade.
    let on_pier = |x: usize, z: usize| {
        let p = Vec2::new(
            -size() / 2.0 + x as f32 * step(),
            -size() / 2.0 + z as f32 * step(),
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
                    if (shore_mask[z * n + x] || fence[z * n + x]) && !on_pier(x, z) {
                        12.0
                    } else {
                        -40.0
                    }
                })
                .collect()
        })
        .collect();
    Collider::heightfield(heights, Vec3::new(size(), 1.0, size()))
}

/// Punti della linea di riva (per i paletti del debug): metà tra una cella di mare e una di terra.
fn shore_points(field: &Field, mask: &[bool]) -> Vec<Vec2> {
    let n = field.n;
    let w = |i: usize| -size() / 2.0 + i as f32 * step();
    let mut v = Vec::new();
    for z in 0..n - 1 {
        for x in 0..n - 1 {
            let m = mask[z * n + x];
            if m != mask[z * n + x + 1] {
                v.push(Vec2::new(w(x) + step() / 2.0, w(z)));
            }
            if m != mask[(z + 1) * n + x] {
                v.push(Vec2::new(w(x), w(z) + step() / 2.0));
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

    fn nord_spec() -> crate::water::IslandSpec {
        crate::region::set(crate::region::Region::Nord);
        let spec = crate::water::load_spec();
        crate::region::set(crate::region::Region::Isola);
        spec
    }

    #[test]
    fn the_continent_has_sea_south_a_lake_an_islet_and_closed_borders() {
        let spec = nord_spec();
        let h = |x: f32, z: f32| continent_height(x, z, &spec, &[]);
        // Piana percorribile, sopra il livello del mare.
        assert!(h(20.0, -80.0) > 0.5, "{}", h(20.0, -80.0));
        // Mare a sud dell'approdo.
        assert!(h(0.0, 520.0) < SEA_LEVEL - 1.0);
        // Il lago è sotto il livello del mare (lo allaga il mare), l'isolotto ne emerge.
        let (c, r) = spec.basin.unwrap();
        assert!(h(c.x + r.x * 0.5, c.y) < SEA_LEVEL - 1.0);
        let (ic, _) = spec.islet.unwrap();
        assert!(h(ic.x, ic.y) > 1.0);
        // Il rio è un canale a livello del mare per tutta la lunghezza.
        for p in &spec.channel {
            assert!(h(p.x, p.y) < SEA_LEVEL - 0.5, "{p:?} {}", h(p.x, p.y));
        }
        // Oltre il confine il terreno sale: di lì non si passa.
        let (xb, zb) = spec.bounds.unwrap();
        assert!(h(xb + 70.0, 0.0) > 20.0);
        assert!(h(0.0, -zb - 26.0) > h(0.0, -zb + 30.0) + 2.0);
        // L'approdo è terra, il molo sporge sul mare.
        assert!(h(0.0, 452.0) > 0.5, "{}", h(0.0, 452.0));
        assert!(continent_water(&spec, Vec2::new(0.0, 480.0)));
        assert!(!continent_water(&spec, Vec2::new(0.0, 440.0)));
    }

    #[test]
    fn every_place_of_the_continent_is_on_dry_land_inside_the_borders() {
        let spec = nord_spec();
        let (xb, zb) = spec.bounds.unwrap();
        assert!(spec.places.len() >= 17);
        for (name, p) in &spec.places {
            assert!(p.x.abs() < xb && p.y > -zb, "{name} fuori dal confine");
            // Il molo, il ponte, l'isolotto e le palafitte stanno sull'acqua o accanto.
            let wet = ["Approdo", "Ponte", "Isolotto", "Palafitte", "Acquedotto"];
            if !wet.iter().any(|w| name.starts_with(w)) {
                assert!(!continent_water(&spec, *p), "{name} in acqua");
            }
        }
    }
    use bevy::mesh::{Mesh, Meshable, VertexAttributeValues};

    /// Le tangenti analitiche devono avere lo stesso segno di quelle generate da mikktspace.
    #[test]
    fn holes_follow_the_placement_rotation() {
        let h = TerrainHole {
            center: Vec2::new(10.0, 20.0),
            yaw: std::f32::consts::PI,
            min: Vec2::new(-3.6, -13.0),
            max: Vec2::new(-1.4, -8.0),
        };
        // Girata di 180°: locale (-2,5, -10) è a (+2,5, +10) dal centro.
        assert!(h.contains(Vec2::new(12.5, 30.0)));
        assert!(!h.contains(Vec2::new(7.5, 10.0)));
    }

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
