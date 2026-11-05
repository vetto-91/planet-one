//! Planet One — RPG 3D open source, cel-shaded.
//!
//! Fase 2.1: character controller cinematico (Avian MoveAndSlide),
//! terreno 100 m, debug wireframe collider (F3).

mod camera;
mod combat;
mod entities;
mod hud;
mod inventory;
mod menu;
mod player;
mod quests;
mod scene;
mod sky;
mod terrain;

use avian3d::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::gizmos::config::GizmoConfigStore;
use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use quests::denaro_rubato as quest;

/// Se true, mostra wireframe globale delle mesh (diagnostica).
/// Default false per non coprire la scena in produzione.
const DEBUG_WIREFRAME: bool = false;

/// Stati principali dell'applicazione.
///
/// - `Playing`: gameplay attivo (movimento, camera, fisica).
/// - `Paused`: menu Esc aperto; i sistemi di gameplay sono sospesi.
/// - `Resetting`: transitorio: in un singolo frame despawn e ricostruzione
///   della scena da zero; evita che sistemi di gameplay tentino di scrivere
///   su entità che stanno per essere distrutte.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Gameplay attivo.
    #[default]
    Playing,
    /// Menu aperto (Esc): gameplay sospeso.
    Paused,
    /// Diario delle quest aperto (J): gameplay sospeso, cursore libero.
    Journal,
    /// Reset della scena in corso (un frame, non interattivo).
    Resetting,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Planet One".into(),
                    resolution: (1280u32, 720u32).into(),
                    // Avvio a schermo intero di default; F11 torna a finestra.
                    mode: bevy::window::WindowMode::Fullscreen(
                        bevy::window::MonitorSelection::Current,
                        bevy::window::VideoModeSelection::Current,
                    ),
                    ..default()
                }),
                // In Bevy 0.20 Esc non chiude più la finestra di default:
                // lo lasciamo gestire solo al menu di gioco.
                ..default()
            }),
            PhysicsPlugins::default(),
            // Debug fisica Avian (collider wireframe): disabilitato di
            // default, attivabile con F3. Utile per confrontare mesh e collider.
            PhysicsDebugPlugin,
            EguiPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        // Stato applicativo + risorse.
        .init_state::<AppState>()
        // Wireframe delle mesh disattivato di default (era diagnostico);
        // riattivabile con DEBUG_WIREFRAME.
        .insert_resource(WireframeConfig {
            global: DEBUG_WIREFRAME,
            ..default()
        })
        .insert_resource(ClearColor(Color::srgb(0.02, 0.03, 0.05)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.9, 0.92, 0.98),
            brightness: 80.0,
            ..Default::default()
        })
        .init_resource::<camera::OrbitCamera>()
        .init_resource::<menu::ShowFps>()
        .init_resource::<entities::DialogState>()
        .init_resource::<entities::NearestInteract>()
        .init_resource::<inventory::Inventory>()
        .init_resource::<quest::QuestState>()
        .init_resource::<quests::QuestTexts>()
        .init_resource::<quests::Barks>()
        .init_resource::<menu::JournalTab>()
        .init_resource::<combat::ContactDamageCd>()
        // Asset del terreno: heightmap PNG + handle del mesh generato.
        .add_systems(
            Startup,
            (
                scene::spawn_camera,
                scene::spawn_world_startup,
                terrain::setup_assets,
            ),
        )
        // All'avvio il wireframe dei collider è DISATTIVO; F3 lo attiva
        // (utile in sviluppo per confrontare mesh e collider del terreno).
        .add_systems(Startup, disable_physics_debug_default)
        // Disabilita temporaneamente fullscreen di default per test input (F11 per riattivare)
        .add_systems(
            Startup,
            |mut q: Query<&mut bevy::window::Window, With<bevy::window::PrimaryWindow>>| {
                if let Ok(mut w) = q.single_mut() {
                    w.mode = bevy::window::WindowMode::Windowed;
                }
            },
        )
        // Ricostruzione del terreno quando la heightmap è disponibile o cambia.
        .add_systems(Update, (terrain::update_terrain, sky::apply_sky_texture))
        // Fisica e movimento del player in FixedUpdate (delta stabile):
        // ground-check, input/gravita'/salto, MoveAndSlide, facing in un'unica
        // pipeline fixed: salto e contatto restano coerenti anche in corsa.
        .add_systems(
            FixedUpdate,
            (
                player::update_grounded,
                player::movement,
                player::run_move_and_slide,
                player::facing,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        // Nemico in FixedUpdate: stesso pipeline del player per MoveAndSlide.
        .add_systems(
            FixedUpdate,
            entities::enemy_patrol.run_if(in_state(AppState::Playing)),
        )
        // Camera in Update (dopo il FixedUpdate): segue il pg con delta reale.
        .add_systems(
            Update,
            (
                camera::rmb_raw,
                camera::orbit,
                camera::mode_input,
                camera::follow,
                camera::player_visibility,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        // Interazione (E) e avanzamento dialogo in Update, solo in Playing.
        .add_systems(
            Update,
            (
                quest::quest_on_dialog_close,
                quest::quest_on_thief_death,
                quest::collect_quest_coins,
                quest::update_thief_hint,
                quest::place_quest_thief,
                quest::tick_title,
                quest::thief_notice_player,
                quests::tick_barks,
                entities::dialog_walk_away.before(entities::interact),
                // advance_dialog PRIMA di interact: la E che apre il dialogo
                // non deve anche avanzarlo nello stesso frame.
                entities::advance_dialog.before(entities::interact),
                entities::interact,
                inventory::collect_pickups,
                inventory::use_hotbar_items,
                combat::player_attack,
                combat::contact_damage,
                combat::handle_enemy_death,
                combat::enemy_respawn_tick,
            )
                .run_if(in_state(AppState::Playing)),
        )
        // Menu e overlay FPS: il disegno della UI egui DEVE girare nella
        // schedule EguiPrimaryContextPass (in egui 0.36 i font sono
        // disponibili solo durante il pass).
        .add_systems(EguiPrimaryContextPass, menu::ui)
        // Toggle Esc: bidirezionale (Playing ↔ Paused), esclude Resetting.
        // (dopo advance_dialog e prima di interact: vedi `DialogState::block_interact`)
        .add_systems(
            Update,
            menu::toggle
                .after(entities::advance_dialog)
                .before(entities::interact),
        )
        // F3: toggle debug wireframe dei collider (PhysicsDebugPlugin).
        .add_systems(
            Update,
            (
                toggle_physics_debug.run_if(in_state(AppState::Playing)),
                toggle_fullscreen,
                apply_cursor_for_mode,
                warp_cursor_to_center,
                apply_cursor_warp,
                log_input_events,
            ),
        )
        .add_systems(OnEnter(AppState::Resetting), scene::reset_scene)
        .run();
}

/// Disattiva di default il wireframe dei collider all'avvio.
/// F3 lo riattiva a runtime per confrontare mesh e collider.
fn disable_physics_debug_default(mut store: ResMut<GizmoConfigStore>) {
    store
        .config_mut::<avian3d::prelude::PhysicsGizmos>()
        .0
        .enabled = false;
}

/// Applica le opzioni del cursore in base alla modalità finestra:
/// - fullscreen (F11): cursore nascosto e bloccato (grab) → tutti gli input restano nel gioco.
/// - finestra: cursore visibile e libero → può uscire e interagire col sistema.
fn apply_cursor_for_mode(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut cursor_opts: Query<&mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
    app_state: Res<State<AppState>>,
    mut last: Local<(Option<bevy::window::WindowMode>, bool)>,
) {
    use bevy::window::{CursorGrabMode, WindowMode};
    let Ok(window) = windows.single() else {
        return;
    };
    let fullscreen = !matches!(window.mode, WindowMode::Windowed);
    // A schermo intero catturiamo SOLO in Playing: in Paused (menu) il cursore
    // deve tornare visibile e libero per cliccare i bottoni.
    let capture = fullscreen && *app_state.get() == AppState::Playing;
    if *last == (Some(window.mode), capture) {
        return;
    }
    *last = (Some(window.mode), capture);
    let Ok(mut opts) = cursor_opts.single_mut() else {
        return;
    };
    if capture {
        opts.visible = false;
        // Su X11 Locked ricade su Confined, ma è il massimo disponibile.
        opts.grab_mode = CursorGrabMode::Locked;
        info!("[CURSOR] cattura ON (fullscreen+playing): nascosto+bloccato");
    } else {
        opts.visible = true;
        opts.grab_mode = CursorGrabMode::None;
        if fullscreen {
            info!("[CURSOR] fullscreen ma menu aperto: cursore libero");
        } else {
            info!("[CURSOR] finestra: cursore visibile e libero");
        }
    }
}

/// In fullscreen + Playing, riporta il cursore al centro della finestra ogni
/// frame: su X11 Confined blocca ai bordi, ma il delta del movimento continua
/// ad arrivare, così la rotazione yaw resta infinita (360°).
fn warp_cursor_to_center(
    mut commands: Commands,
    windows: Query<(Entity, &Window), With<bevy::window::PrimaryWindow>>,
    app_state: Res<State<AppState>>,
) {
    let Ok((entity, window)) = windows.single() else {
        return;
    };
    let fullscreen = !matches!(window.mode, bevy::window::WindowMode::Windowed);
    if !fullscreen || *app_state.get() != AppState::Playing {
        return;
    }
    let size = window.resolution.size();
    // set_cursor_position e' un metodo su &mut Window: usiamo un EntityCommand
    // via Commands per ottenere il borrow mutabile in modo sicuro.
    commands
        .entity(entity)
        .insert(CursorWarpRequest(size / 2.0));
}

/// Componente temporaneo: posizione a cui warpare il cursore (centro finestra).
#[derive(Component)]
struct CursorWarpRequest(Vec2);

/// Applica il warp del cursore richiesto nel frame precedente.
fn apply_cursor_warp(
    mut commands: Commands,
    mut windows: Query<(&mut Window, Entity), With<bevy::window::PrimaryWindow>>,
    requests: Query<(Entity, &CursorWarpRequest)>,
) {
    for (req_entity, req) in &requests {
        if let Ok((mut window, win_entity)) = windows.get_mut(req_entity) {
            window.set_cursor_position(Some(req.0));
            let _ = win_entity;
        }
        commands.entity(req_entity).remove::<CursorWarpRequest>();
    }
}

/// Logga TUTTI gli eventi di input (tastiera, mouse, wheel, finestra) con
/// prefisso [INPUT]/[EVENT] per debug via grep. Attivo sempre in dev.
fn log_input_events(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut mouse_btn: MessageReader<bevy::input::mouse::MouseButtonInput>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut cursor_moved: MessageReader<bevy::window::CursorMoved>,
) {
    for ev in keys.read() {
        info!("[INPUT] key {:?} {:?}", ev.key_code, ev.state);
    }
    for ev in mouse_btn.read() {
        info!("[INPUT] mouse btn {:?} {:?}", ev.button, ev.state);
    }
    for ev in mouse_motion.read() {
        info!("[INPUT] mouse motion delta={:?}", ev.delta);
    }
    for ev in wheel.read() {
        info!("[INPUT] wheel {:?}", ev.unit);
    }
    for ev in cursor_moved.read() {
        info!(
            "[EVENT] cursor moved to {:?} delta={:?}",
            ev.position, ev.delta
        );
    }
}

/// F11: toggle fullscreen (usa il sistema operativo per catturare input).
fn toggle_fullscreen(
    keys: Res<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    use bevy::window::{MonitorSelection, VideoModeSelection, WindowMode};
    if !keys.just_pressed(KeyCode::F11) {
        return;
    }
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    window.mode = match window.mode {
        WindowMode::Windowed => {
            WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
        }
        _ => WindowMode::Windowed,
    };
    info!("fullscreen: {:?}", window.mode);
}

/// Toggle con F3 del rendering debug dei collider (wireframe arancione).
/// Utile in fase di sviluppo per confrontare il collider fisico con la mesh.
fn toggle_physics_debug(keys: Res<ButtonInput<KeyCode>>, mut store: ResMut<GizmoConfigStore>) {
    if keys.just_pressed(KeyCode::F3) {
        let cfg = store.config_mut::<avian3d::prelude::PhysicsGizmos>();
        cfg.0.enabled = !cfg.0.enabled;
        info!("physics debug wireframe: {}", cfg.0.enabled);
    }
}
