//! Scena: costruzione, marcatura e reset completo.
//!
//! Ogni entità radice della scena 3D (terreno, luci, player) è marcata con
//! [`SceneNode`]: il reset del menu fa despawn ricorsivo di tutte queste
//! entità e ricostruisce la scena da zero.
//!
//! Nota: la camera NON è marcata [`SceneNode`]. Il contesto egui (la UI del
//! menu) è agganciato alla prima camera creata e `bevy_egui` non lo ricrea
//! mai: despawnare la camera ucciderebbe il menu. Il reset la riposiziona
//! al punto di partenza invece di ricrearla. Per questo lo spawn della
//! camera è separato (`spawn_camera`, chiamato solo allo startup) dallo
//! spawn degli altri elementi (`spawn_world`, chiamato sia allo startup
//! sia durante il reset).

use crate::{AppState, camera::OrbitCamera, player};
use avian3d::prelude::*;
use bevy::prelude::*;

/// Marker sulle entità radice della scena, soggette al reset.
#[derive(Component)]
pub struct SceneNode;

/// Marker sul collider del terreno (usato dal ground-check del salto).
#[derive(Component)]
pub struct Ground;

/// Costruisce la camera 3D UNA sola volta, allo startup.
///
/// La camera NON viene ricreata durante il reset: il contesto egui è
/// agganciato ad essa e `bevy_egui` non lo ricrea mai. Il transform
/// iniziale viene poi guidato dal sistema di follow; in caso di reset
/// viene riportato al punto di partenza.
///
/// Nota: all'avvio Bevy 0.20 stampa un warning "render graph not
/// configured" riferito a entità Observer interne (upstream, non nostro):
/// la nostra camera ha il graph corretto tramite le required components di
/// `Camera3d`.
pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(-4.0, 3.5, 6.0)));
}

/// Costruisce gli elementi della scena soggetti a reset: terreno, luci e
/// player. Chiamata sia dallo startup sia dal sistema di reset.
///
/// I parametri sono reference espliciti (non `ResMut`) così la stessa
/// funzione è riusabile sia dallo startup sia dal reset.
pub fn spawn_world(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // --- Terreno: collider statico + mesh visuale ---
    commands.spawn((
        SceneNode,
        Ground,
        RigidBody::Static,
        Collider::cuboid(40.0, 1.0, 40.0),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));
    commands.spawn((
        SceneNode,
        Mesh3d(meshes.add(Plane3d {
            half_size: Vec2::splat(20.0),
            ..default()
        })),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.12, 0.10),
            ..default()
        })),
    ));

    // --- Luci ---
    // Luce ambientale soft (riempie le zone in ombra).
    commands.spawn((
        SceneNode,
        AmbientLight {
            color: Color::srgb(0.6, 0.7, 0.9),
            brightness: 80.0,
            ..default()
        },
    ));
    // Luce direzionale (sole) con ombre.
    commands.spawn((
        SceneNode,
        DirectionalLight {
            illuminance: 2500.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // --- Player ---
    player::spawn(commands, meshes, materials);
}

/// Adapter per l'uso come sistema di startup: converte i `ResMut`
/// della firma di sistema nei semplici reference di [`spawn_world`].
pub fn spawn_world_startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_world(&mut commands, &mut meshes, &mut materials);
}

/// Reset completo della scena, eseguito su `OnEnter(AppState::Resetting)`.
///
/// Perché usare uno stato dedicato invece di un booleano in `Update`?
/// Durante il reset il player e altre entità vengono despawnate; se nel
/// medesimo frame qualche sistema di gameplay (es. ground-check) tentasse
/// di inserire un componente sul player, si otterrebbe un panic
/// "Entity despawned". Mettendo il reset in `OnEnter(Resetting)` tutti i
/// sistemi di gameplay (che hanno `run_if(in_state(Playing))`) sono
/// garantiti spenti; al termine torniamo a `Playing`.
///
/// Passaggi:
/// 1. Despawn ricorsivo di tutte le entità [`SceneNode`]
///    (terreno, luci, player con i suoi figli mesh).
/// 2. Ripristino della camera persistente al punto di partenza
///    e dei parametri di orbita ai default.
/// 3. Ricostruzione del mondo con [`spawn_world`]
///    (la camera NON viene ricreata).
/// 4. Transizione verso `AppState::Playing`.
pub fn reset_scene(
    mut commands: Commands,
    scene_nodes: Query<Entity, With<SceneNode>>,
    mut orbit: ResMut<OrbitCamera>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut next: ResMut<NextState<AppState>>,
) {
    // 1. Despawn di tutte le entità radice della scena.
    //    In Bevy 0.20 despawn() è già ricorsivo sui Children (relationship).
    for entity in &scene_nodes {
        commands.entity(entity).despawn();
    }

    // 2. Camera e orbita tornano ai valori di partenza.
    *orbit = OrbitCamera::default();
    for mut transform in &mut cameras {
        *transform = Transform::from_xyz(-4.0, 3.5, 6.0);
    }

    // 3. Ricostruzione del mondo: la camera esistente viene solo riposizionata.
    spawn_world(&mut commands, &mut meshes, &mut materials);

    // 4. Rientro in Playing; i sistemi di gameplay riprenderono dal frame successivo.
    next.set(AppState::Playing);
}
