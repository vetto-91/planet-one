//! Menu Esc (egui): toggle pausa, lista tasti, slider sensibilità, reset scena e FPS overlay.

use crate::{
    AppState,
    camera::OrbitCamera,
    entities::{DialogState, NearestInteract},
};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

/// Risorsa che decide se mostrare il contatore FPS (default: sì).
#[derive(Resource)]
pub struct ShowFps(pub bool);

impl Default for ShowFps {
    fn default() -> Self {
        Self(true)
    }
}

/// Toggle del menu: `Esc` alterna `Playing` ↔ `Paused` in entrambe le direzioni.
///
/// La finestra di Bevy NON si chiude con Esc perché `close_on_escape`
/// è disattivato in `main.rs`: Esc serve solo al menu.
pub fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    dialog: Res<crate::entities::DialogState>,
    mut next: ResMut<NextState<AppState>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    // Se un dialogo è aperto, Esc lo chiude (gestito da advance_dialog):
    // qui non apriamo il menu.
    if dialog.open {
        return;
    }
    match state.get() {
        AppState::Playing => next.set(AppState::Paused),
        AppState::Paused => next.set(AppState::Playing),
        // Durante il reset ESC non fa nulla: è uno stato transitorio.
        AppState::Resetting => {}
    }
}

/// UI egui: hint discreto in gioco, menu completo in pausa.
///
/// - In `Playing`: solo un piccolo hint in alto a sinistra ("ESC = menu").
/// - In `Paused`: pannello centrale con la lista dei tasti, lo slider della
///   sensibilità del mouse, il checkbox per il contatore FPS, il pulsante
///   "Riprendi" e il pulsante "Reset scena" (ricostruzione da zero di
///   terreno, luci e player).
#[allow(clippy::too_many_arguments)]
pub fn ui(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut orbit: ResMut<OrbitCamera>,
    mut show_fps: ResMut<ShowFps>,
    diagnostics: Res<DiagnosticsStore>,
    dialog: Res<DialogState>,
    nearest: Res<NearestInteract>,
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

            // Prompt di interazione: appare quando il player e' vicino a un
            // NPC interagibile (il sistema entities::interact gestisce il range).
            // Mostriamo "[E] Parla" quando un dialogo non e' aperto e c'e'
            // un interagibile vicino: lo deduciamo da una query separata.
            // (semplificato: mostriamo il prompt se DialogState non e' aperto
            // e il player e' fermo vicino all'NPC — gestito sotto via query)
            show_interact_prompt(ctx, &dialog, &nearest.prompt);

            // Pannello dialogo in basso (stile RPG) quando aperto.
            if dialog.open {
                show_dialog_panel(ctx, &dialog);
            }

            // Contatore FPS in alto a destra (se attivato dal menu).
            if show_fps.0
                && let Some(fps) = diagnostics
                    .get(&FrameTimeDiagnosticsPlugin::FPS)
                    .and_then(|d| d.smoothed())
            {
                let size = ctx.viewport_rect().size();
                egui::Area::new(egui::Id::new("fps"))
                    .fixed_pos(egui::pos2(size.x - 100.0, 12.0))
                    .show(ctx, |ui| {
                        ui.label(format!("FPS: {:.0}", fps));
                    });
            }
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

                    // Toggle contatore FPS.
                    ui.checkbox(&mut show_fps.0, "Mostra FPS");
                    ui.separator();

                    // Riprendi (equivalente a Esc).
                    if ui.button("Riprendi").clicked() {
                        next.set(AppState::Playing);
                    }

                    // Reset della scena: ricostruzione da zero.
                    // Entra nello stato transitorio `Resetting`, in cui tutti i
                    // sistemi di gameplay sono spenti; questo evita il panic
                    // "Entity despawned" che accadeva quando il reset era
                    // eseguito in `Update` insieme al ground-check.
                    if ui.button("Reset scena").clicked() {
                        next.set(AppState::Resetting);
                    }
                });
        }
        // Durante Resetting non mostriamo nulla; la UI riapparirà in Playing/Paused.
        AppState::Resetting => {}
    }
}

/// Mostra il prompt di interazione in basso al centro quando il player e'
/// nel range di un interagibile (NPC). Il prompt usa il testo di
/// `Interactable::prompt` del nearest.
fn show_interact_prompt(
    ctx: &egui::Context,
    dialog: &DialogState,
    nearest_prompt: &Option<String>,
) {
    if dialog.open || nearest_prompt.is_none() {
        return;
    }
    let size = ctx.viewport_rect().size();
    egui::Area::new(egui::Id::new("interact_prompt"))
        .fixed_pos(egui::pos2(size.x / 2.0 - 60.0, size.y - 120.0))
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(nearest_prompt.as_deref().unwrap_or("[E] Interagisci"))
                    .strong(),
            );
        });
}

/// Pannello del dialogo in basso, stile RPG: nome parlante + riga corrente +
/// hint per avanzare/chiudere.
fn show_dialog_panel(ctx: &egui::Context, dialog: &DialogState) {
    let size = ctx.viewport_rect().size();
    let panel_h = 110.0;
    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgba_unmultiplied(15, 18, 28, 235))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(90, 110, 160),
        ))
        .inner_margin(egui::Margin::symmetric(16, 12))
        .corner_radius(egui::CornerRadius::same(6));

    egui::Area::new(egui::Id::new("dialog_panel"))
        .fixed_pos(egui::pos2(size.x / 2.0 - 320.0, size.y - panel_h - 24.0))
        .show(ctx, |ui| {
            ui.set_width(640.0);
            ui.set_height(panel_h);
            frame.show(ui, |ui| {
                ui.label(egui::RichText::new(&dialog.speaker).strong().size(16.0));
                ui.add_space(4.0);
                if let Some(line) = dialog.lines.get(dialog.index) {
                    ui.label(line);
                }
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Enter/Space = avanti   •   Esc = chiudi")
                        .small()
                        .color(egui::Color32::from_rgb(160, 170, 190)),
                );
            });
        });
}
