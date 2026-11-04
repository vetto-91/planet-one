//! Menu Esc (egui): toggle pausa, lista tasti, slider sensibilità e reset scena.

use crate::{AppState, camera::OrbitCamera, scene::ResetSceneRequest};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

/// Toggle del menu: `Esc` alterna `Playing` ↔ `Paused`.
///
/// La finestra di Bevy NON si chiude con Esc perché `close_on_escape`
/// è disattivato in `main.rs`: Esc serve solo al menu.
pub fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    match state.get() {
        AppState::Playing => next.set(AppState::Paused),
        AppState::Paused => next.set(AppState::Playing),
    }
}

/// UI egui: hint discreto in gioco, menu completo in pausa.
///
/// - In `Playing`: solo un piccolo hint in alto a sinistra ("ESC = menu").
/// - In `Paused`: pannello centrale con la lista dei tasti, lo slider della
///   sensibilità del mouse, il pulsante "Riprendi" e il pulsante
///   "Reset scena" (ricostruzione da zero di terreno, luci e player).
pub fn ui(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut orbit: ResMut<OrbitCamera>,
    mut reset: ResMut<ResetSceneRequest>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return; // Contesto egui non ancora pronto (primi frame).
    };

    match state.get() {
        AppState::Playing => {
            // Hint minimale sempre visibile durante il gioco.
            egui::Area::new(egui::Id::new("hint"))
                .fixed_pos(egui::pos2(12.0, 12.0))
                .show(ctx, |ui| {
                    ui.label("ESC = menu");
                });
        }
        AppState::Paused => {
            // In egui 0.36 i CentralPanel si mostrano dentro una Ui radice:
            // la creiamo dal contesto con il pattern degli esempi bevy_egui.
            let mut viewport_ui = egui::Ui::new(
                ctx.clone(),
                "menu_root".into(),
                egui::UiBuilder::new()
                    .layer_id(egui::LayerId::background())
                    .max_rect(ctx.viewport_rect()),
            );

            // Frame scuro semi-trasparente per coprire il mondo in pausa.
            let frame = egui::Frame::default()
                .fill(egui::Color32::from_rgba_unmultiplied(8, 10, 15, 220))
                .inner_margin(egui::Margin::symmetric(24, 16));

            egui::CentralPanel::default()
                .frame(frame)
                .show(&mut viewport_ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Planet One");
                        ui.label("— pausa —");
                    });
                    ui.separator();

                    // Lista dei controlli disponibili.
                    ui.label("Controlli");
                    ui.label("• W/A/S/D o frecce — movimento");
                    ui.label("• Shift — corsa");
                    ui.label("• Space — salto");
                    ui.label("• Mouse — orbita camera");
                    ui.label("• Esc — menu / riprendi");
                    ui.separator();

                    // Sensibilità del mouse: modifica la resource in tempo reale.
                    ui.add(
                        egui::Slider::new(&mut orbit.sensitivity, 0.0005..=0.010)
                            .text("Sensibilità mouse"),
                    );

                    ui.separator();

                    // Riprendi (equivalente a Esc).
                    if ui.button("Riprendi").clicked() {
                        next.set(AppState::Playing);
                    }

                    // Reset della scena: ricostruzione da zero.
                    // Pulsante (non tasto) per evitare pressioni accidentali.
                    if ui.button("Reset scena").clicked() {
                        reset.0 = true;
                        next.set(AppState::Playing);
                    }
                });
        }
    }
}
