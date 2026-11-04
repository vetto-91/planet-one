//! Planet One — RPG 3D open source, cel-shaded.
//!
//! Fase 1.2: stato applicativo (Playing/Paused), menu Esc con egui,
//! salto + corsa, sensibilità mouse regolabile e reset della scena.

mod camera;
mod hud;
mod menu;
mod player;
mod scene;

use avian3d::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

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
            EguiPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        // Stato applicativo + risorse.
        .init_state::<AppState>()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.03, 0.05)))
        .init_resource::<camera::OrbitCamera>()
        .init_resource::<hud::ShowFps>()
        // Costruzione iniziale della scena (terreno, luci, camera, player).
        .add_systems(
            Startup,
            (
                scene::spawn_camera,
                scene::spawn_world_startup,
                hud::setup_fps,
            ),
        )
        // Gameplay: attivo solo in `Playing`.
        .add_systems(
            Update,
            (
                player::movement,
                player::facing,
                player::update_grounded,
                player::jump,
                camera::orbit,
                camera::follow,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        // HUD: aggiornamento FPS (sempre attivo, ma mostrato solo se ShowFps).
        .add_systems(Update, hud::update_fps)
        // Menu: il disegno della UI egui DEVI girare nella schedule
        // EguiPrimaryContextPass (in egui 0.36 i font sono disponibili solo
        // durante il pass).
        .add_systems(EguiPrimaryContextPass, menu::ui)
        // Toggle Esc e reset: lo spostiamo su OnEnter(Resetting).
        // Il toggle Esc in Paused permette anche di chiudere il menu;
        // il reset vero è gestito dallo stato Resetting, non più da un
        // booleano, per evitare il panic "Entity despawned".
        .add_systems(Update, menu::toggle.run_if(in_state(AppState::Paused)))
        .add_systems(OnEnter(AppState::Resetting), scene::reset_scene)
        .run();
}
