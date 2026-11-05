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
mod scene;
mod sky;
mod terrain;

use avian3d::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::gizmos::config::GizmoConfigStore;
use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

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
        // Ricostruzione del terreno quando la heightmap è disponibile o cambia.
        .add_systems(Update, terrain::update_terrain)
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
                entities::interact,
                entities::advance_dialog,
                inventory::collect_pickups,
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
        .add_systems(Update, menu::toggle)
        // F3: toggle debug wireframe dei collider (PhysicsDebugPlugin).
        .add_systems(
            Update,
            toggle_physics_debug.run_if(in_state(AppState::Playing)),
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

/// Toggle con F3 del rendering debug dei collider (wireframe arancione).
/// Utile in fase di sviluppo per confrontare il collider fisico con la mesh.
fn toggle_physics_debug(keys: Res<ButtonInput<KeyCode>>, mut store: ResMut<GizmoConfigStore>) {
    if keys.just_pressed(KeyCode::F3) {
        let cfg = store.config_mut::<avian3d::prelude::PhysicsGizmos>();
        cfg.0.enabled = !cfg.0.enabled;
        info!("physics debug wireframe: {}", cfg.0.enabled);
    }
}
