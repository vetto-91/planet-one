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

use crate::{
    AppState, camera::OrbitCamera, entities, hut, inventory, player,
    quests::denaro_rubato as quest, sky, terrain,
};
use bevy::image::Image;
use bevy::light::Skybox;
use bevy::prelude::*;
use bevy::render::view::Tonemapping;

/// Marker sulle entità radice della scena, soggette al reset.
#[derive(Component)]
pub struct SceneNode;

/// Marker sul collider del terreno (usato dal ground-check del salto).
#[derive(Component)]
pub struct Ground;

/// Marker sui muri (pareti degli edifici, mura del paese): fermano anche abitanti e animali, che
/// invece passano tra mobili e bancarelle seguendo le strade (con le porte, `hut::DoorCollider`).
#[derive(Component)]
pub struct Wall;

/// Costruisce la camera 3D UNA sola volta, allo startup.
///
/// La camera NON viene ricreata durante il reset: il contesto egui è
/// agganciato ad essa e `bevy_egui` non lo ricrea mai. Il transform
/// iniziale viene poi guidato dal sistema di follow; in caso di reset
/// viene riportato al punto di partenza.
pub fn spawn_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    // Panoramica del cielo: convertita in cubemap da `sky::apply_sky_texture`.
    // La panoramica si legge solo sulla CPU (diventa la cubemap): non va caricata sulla GPU.
    commands.insert_resource(sky::SkyTexture(
        asset_server
            .load_builder()
            .with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
                s.asset_usage = bevy::asset::RenderAssetUsages::MAIN_WORLD;
            })
            .load("textures/sky.png"),
    ));
    // Cielo: cubemap procedurale generata a runtime (senza asset HDRI).
    let skybox = images.add(sky::gradient_skybox());

    let mut camera = commands.spawn((
        Camera3d::default(),
        // Resa fisica: HDR, tonemapping, bagliore delle luci, occlusione ambientale (ADR 0016).
        bevy::camera::Hdr,
        Tonemapping::TonyMcMapface,
        bevy::post_process::bloom::Bloom::NATURAL,
        Msaa::Off,
        bevy::anti_alias::smaa::Smaa::default(),
        bevy::pbr::ScreenSpaceAmbientOcclusion::default(),
        Transform::from_xyz(-4.0, 3.5, 6.0),
        // Orecchie dell'audio posizionale (vedi `audio::listener`).
        crate::audio::listener(),
        Skybox {
            image: Some(skybox),
            brightness: 1000.0,
            ..Default::default()
        },
    ));
    // Senza finestra la camera disegna in un'immagine (vedi `devtools::headless`).
    if crate::devtools::headless() {
        let target = crate::devtools::headless_target(&mut images);
        // Senza finestra Bevy non sa da dove scegliere il dettaglio delle ombre: è la camera.
        camera.insert((
            bevy::camera::RenderTarget::Image(target.clone().into()),
            bevy::camera::ShadowLodOrigin,
        ));
        commands.insert_resource(crate::devtools::HeadlessTarget(target));
    }
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
    asset_server: &AssetServer,
) {
    // --- Terreno: entità con segnaposto piatto; mesh e collider reali
    //     vengono sostituiti da `terrain::update_terrain` appena la heightmap
    //     è disponibile (o cambia a runtime). ---
    terrain::spawn_terrain(commands, meshes, materials, asset_server);

    // --- Luci ---
    // Luce direzionale (sole) con ombre.
    commands.spawn((
        SceneNode,
        crate::daynight::Sun,
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.85),
            illuminance: 4000.0,
            shadow_maps_enabled: true,
            // Bias alzati per evitare "shadow acne" sul player: con la luce
            // quasi verticale le facce frontali finivano in ombra propria.
            shadow_depth_bias: 0.05,
            shadow_normal_bias: 3.0,
            ..Default::default()
        },
        bevy::light::CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.5,
            first_cascade_far_bound: 12.0,
            maximum_distance: 180.0,
            overlap_proportion: 0.2,
        }
        .build(),
        Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Il sole fa ombra anche con la testa del giocatore, nascosta alla camera in prima persona.
        bevy::camera::visibility::RenderLayers::from_layers(&[0, crate::camera::HEAD_LAYER]),
    ));

    // Luna: luce direzionale bluastra, senza ombre (regolata da `daynight::update_sky`).
    commands.spawn((
        SceneNode,
        crate::daynight::Moon,
        DirectionalLight {
            color: Color::srgb(0.6, 0.7, 1.0),
            illuminance: 0.0,
            shadow_maps_enabled: false,
            ..Default::default()
        },
        Transform::from_xyz(-4.0, -10.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // --- Player ---
    player::spawn(commands, asset_server);

    // Il resto (casetta del Viandante, abitanti di Fontevivo, ladro) vive solo sull'isola.
    if crate::region::current() != crate::region::Region::Isola {
        crate::villagers::spawn_villagers(commands, meshes, materials, asset_server);
        return;
    }

    // --- Casetta-chiosco con il Viandante come negoziante. Il fronte (-Z locale,
    //     con il banco) guarda verso il player, che parte a -X; la quota viene
    //     appoggiata al terreno da `props::snap_to_ground`. ---
    let hut = hut::spawn_hut(
        commands,
        meshes,
        materials,
        asset_server,
        Vec3::new(6.5, 0.0, 0.7),
        std::f32::consts::FRAC_PI_2,
    );
    entities::spawn_npc(commands, asset_server, hut, hut::KEEPER_SHOP);

    // --- Pozione sul tavolo dentro la casetta (figlia della casetta: ne segue la quota) ---
    let potion = inventory::spawn_pickup(
        commands,
        meshes,
        materials,
        inventory::ItemKind::HealthPotion,
        1,
        hut::POTION_ON_TABLE,
    );
    commands.entity(potion).insert(ChildOf(hut));

    // --- Abitanti e guardie di Fontevivo ---
    crate::villagers::spawn_villagers(commands, meshes, materials, asset_server);

    // --- Ladro della quest: posizione fissa richiesta (x 31.2, z -34.9).
    //     La quota viene appoggiata al terreno da `quest::place_quest_thief`. ---
    quest::spawn_thief(
        commands,
        asset_server,
        Vec3::new(31.2, 0.0, -34.9),
        Vec3::new(27.2, 0.0, -34.9),
    );
}

/// Adapter per l'uso come sistema di startup: converte i `ResMut`
/// della firma di sistema nei semplici reference di [`spawn_world`].
pub fn spawn_world_startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    spawn_world(&mut commands, &mut meshes, &mut materials, &asset_server);
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
#[allow(clippy::too_many_arguments)]
pub fn reset_scene(
    mut commands: Commands,
    scene_nodes: Query<Entity, With<SceneNode>>,
    mut orbit: ResMut<OrbitCamera>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut next: ResMut<NextState<AppState>>,
    mut quest: ResMut<quest::QuestState>,
    asset_server: Res<AssetServer>,
    mut texts: ResMut<crate::quests::QuestTexts>,
    mut homes: ResMut<crate::world_props::Homes>,
    load: Res<crate::save::LoadRequest>,
    mut dbg: ResMut<crate::debug_place::DebugPlace>,
    mut village: ResMut<crate::village::VillageInfo>,
    mut terrain_assets: ResMut<terrain::TerrainAssets>,
) {
    // 0. Cambio di regione (viaggio o salvataggio fatto altrove): dati del mondo dell'altra regione.
    if let Some(region) = load.region()
        && region != crate::region::current()
    {
        crate::region::set(region);
        dbg.reload();
        *village = crate::village::VillageInfo::build(&dbg.items);
        terrain_assets.built_mesh = None;
        homes.list.clear();
        info!("[REGIONE] {}", region.title());
    }

    // 1. Despawn di tutte le entità radice della scena.
    //    In Bevy 0.20 despawn() è già ricorsivo sui Children (relationship).
    for entity in &scene_nodes {
        commands.entity(entity).despawn();
    }

    // 2. Camera, orbita e stato quest tornano ai valori di partenza.
    *orbit = OrbitCamera::default();
    *quest = quest::QuestState::default();
    *texts = crate::quests::QuestTexts::load(); // rilegge dialoghi.txt
    for mut transform in &mut cameras {
        *transform = Transform::from_xyz(-4.0, 3.5, 6.0);
    }

    // 3. Ricostruzione del mondo: la camera esistente viene solo riposizionata.
    spawn_world(&mut commands, &mut meshes, &mut materials, &asset_server);

    // 4. Attesa che terreno ed edifici siano pronti, poi `save::finish_loading` entra in gioco.
    homes.built = false;
    next.set(AppState::Loading);
}
