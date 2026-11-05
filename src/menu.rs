//! Menu Esc (egui): toggle pausa, lista tasti, slider sensibilità, reset scena e FPS overlay.

use crate::{
    AppState,
    camera::OrbitCamera,
    entities::{DialogState, NearestInteract},
    hud,
    inventory::Inventory,
    player::PlayerHealth,
    quest::{QuestStage, QuestState},
};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::ecs::system::SystemParam;
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
// Parametri "query-heavy" della UI, raggruppati in uno struct SystemParam:
// Bevy 0.20 supporta al massimo 16 parametri per sistema.
#[derive(SystemParam)]
pub struct UiQueries<'w, 's> {
    pub health: Query<'w, 's, &'static PlayerHealth, With<crate::player::Player>>,
    pub enemies: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            &'static crate::combat::EnemyHealth,
        ),
        With<crate::entities::Enemy>,
    >,
    pub cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform)>,
    pub player_pos: Query<'w, 's, &'static GlobalTransform, With<crate::player::Player>>,
    pub thief_pos: Query<
        'w,
        's,
        &'static GlobalTransform,
        (
            With<crate::quest::QuestThief>,
            Without<crate::player::Player>,
        ),
    >,
    pub coins_pos: Query<'w, 's, &'static GlobalTransform, With<crate::quest::QuestCoin>>,
    pub npc_pos: Query<
        'w,
        's,
        &'static GlobalTransform,
        (With<crate::entities::Npc>, Without<crate::player::Player>),
    >,
}

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
    inventory: Res<Inventory>,
    mut exit: MessageWriter<AppExit>,
    quest: Res<QuestState>,
    keys: Res<ButtonInput<KeyCode>>,
    q: UiQueries,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return; // Contesto egui non ancora pronto (primi frame).
    };

    match state.get() {
        AppState::Playing => {
            // Hint sempre visibile + contatore monete in possesso (alto-sinistra).
            egui::Area::new(egui::Id::new("hint"))
                .fixed_pos(egui::pos2(12.0, 12.0))
                .show(ctx, |ui| {
                    hud_chip(ui, "ESC = menu");
                });
            let coins = inventory.qty(crate::inventory::ItemKind::Coin);
            egui::Area::new(egui::Id::new("coins"))
                .fixed_pos(egui::pos2(12.0, 48.0))
                .show(ctx, |ui| {
                    hud_chip(ui, &format!("Monete: {coins}"));
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

            // HUD di gioco: barra vita (basso-sx), crosshair, inventario.
            if let Ok(hp) = q.health.single() {
                hud::draw_hud(ctx, hp, &inventory);
            }

            // Barre vita dei nemici sopra le loro teste (proiezione world→screen).
            hud::draw_enemy_bars(ctx, &q.enemies, &q.cameras);

            // Bussola in alto al centro + titolo quest + obiettivi (TAB).
            if let Ok(pp) = q.player_pos.single() {
                draw_compass(
                    ctx,
                    &quest,
                    pp.translation(),
                    &q.thief_pos,
                    &q.coins_pos,
                    &q.npc_pos,
                    &orbit,
                );
            }
            if let Some((text, _t)) = &quest.title {
                draw_title_banner(ctx, text);
            }
            if keys.pressed(KeyCode::Tab) {
                draw_objectives(ctx, &quest);
            }

            // Contatore FPS in alto a destra (se attivato dal menu).
            if show_fps.0
                && let Some(fps) = diagnostics
                    .get(&FrameTimeDiagnosticsPlugin::FPS)
                    .and_then(|d| d.smoothed())
            {
                let size = ctx.viewport_rect().size();
                egui::Area::new(egui::Id::new("fps"))
                    .fixed_pos(egui::pos2(size.x - 120.0, 12.0))
                    .show(ctx, |ui| {
                        hud_chip(ui, &format!("FPS: {:.0}", fps));
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
                    ui.label("• F3 — wireframe collider (debug)");
                    ui.label("• F11 — schermo intero on/off");
                    ui.label("• V — prima persona on/off, RMB tenuto = prima persona temporanea");
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

                    ui.add_space(8.0);
                    // Esci dal gioco: chiude la finestra con AppExit::Success.
                    if ui.button("Esci dal gioco").clicked() {
                        info!("[EVENT] richiesta uscita dal gioco");
                        exit.write(AppExit::Success);
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

/// Disegna la bussola in alto al centro: cerchio con freccia verso il target
/// della quest + coordinate x,z del player.
fn draw_compass(
    ctx: &egui::Context,
    quest: &QuestState,
    player_pos: Vec3,
    thief: &Query<
        &GlobalTransform,
        (
            With<crate::quest::QuestThief>,
            Without<crate::player::Player>,
        ),
    >,
    coins: &Query<&GlobalTransform, With<crate::quest::QuestCoin>>,
    npc: &Query<&GlobalTransform, (With<crate::entities::Npc>, Without<crate::player::Player>)>,
    orbit: &crate::camera::OrbitCamera,
) {
    let size = ctx.viewport_rect().size();
    let center = egui::pos2(size.x / 2.0, 30.0);

    // Scegli il target in base allo stage.
    let target: Option<Vec3> = match quest.stage {
        QuestStage::SeekThief => thief.single().ok().map(|t| t.translation()),
        QuestStage::Collecting => coins
            .iter()
            .min_by(|a, b| {
                let da = a.translation().distance(player_pos);
                let db = b.translation().distance(player_pos);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|t| t.translation()),
        QuestStage::Returning | QuestStage::Completed => npc.single().ok().map(|t| t.translation()),
        QuestStage::Idle => npc.single().ok().map(|t| t.translation()),
    };

    // Heading della camera in gradi compass (0 = N = -Z, 90 = E = +X).
    // La camera guarda in direzione (-sin yaw, -cos yaw): bearing = -yaw.
    let heading_deg = (-orbit.yaw).to_degrees().rem_euclid(360.0);
    const BAR_W: f32 = 440.0;
    const BAR_H: f32 = 46.0;
    const WIN_HALF_DEG: f32 = 45.0; // finestra visibile ~90°

    egui::Area::new(egui::Id::new("compass"))
        .fixed_pos(egui::pos2(center.x - BAR_W / 2.0, 10.0))
        .show(ctx, |ui| {
            let (rect, _resp) =
                ui.allocate_exact_size(egui::vec2(BAR_W, BAR_H), egui::Sense::hover());
            let painter = ui.painter().clone();
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(8),
                egui::Color32::from_rgba_unmultiplied(10, 14, 22, 205),
            );

            // Disco: tacche ogni 10°, centrate sullo sguardo.
            let mid_x = rect.center().x;
            let px_per_deg = (BAR_W - 40.0) / (2.0 * WIN_HALF_DEG);
            for step in 0..36 {
                let bearing = step as f32 * 10.0;
                let mut off = (bearing - heading_deg).rem_euclid(360.0);
                if off > 180.0 {
                    off -= 360.0;
                }
                if off.abs() > WIN_HALF_DEG {
                    continue;
                }
                let x = mid_x + off * px_per_deg;
                // Cardinali: lettera (N/E/S/W) grande; altri gradi: numero piccolo.
                let cardinal = match bearing as i32 {
                    0 => Some(("N", egui::Color32::from_rgb(255, 210, 80))),
                    90 => Some(("E", egui::Color32::from_rgb(245, 248, 255))),
                    180 => Some(("S", egui::Color32::from_rgb(245, 248, 255))),
                    270 => Some(("W", egui::Color32::from_rgb(245, 248, 255))),
                    _ => None,
                };
                let (text, color, font, tick_len) = match cardinal {
                    Some((letter, c)) => (
                        letter.to_string(),
                        c,
                        egui::FontId::proportional(18.0),
                        12.0,
                    ),
                    None => (
                        format!("{}", bearing as i32),
                        egui::Color32::from_rgb(170, 180, 200),
                        egui::FontId::monospace(12.0),
                        7.0,
                    ),
                };
                painter.line_segment(
                    [
                        egui::pos2(x, rect.top() + 4.0),
                        egui::pos2(x, rect.top() + 4.0 + tick_len),
                    ],
                    egui::Stroke::new(1.5, color),
                );
                painter.text(
                    egui::pos2(x, rect.top() + 30.0),
                    egui::Align2::CENTER_CENTER,
                    text,
                    font,
                    color,
                );
            }

            // Indicatore fisso al centro (direzione di sguardo).
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(mid_x - 5.0, rect.top() + 1.0),
                    egui::pos2(mid_x + 5.0, rect.top() + 1.0),
                    egui::pos2(mid_x, rect.top() + 8.0),
                ],
                egui::Color32::WHITE,
                egui::Stroke::NONE,
            ));

            // Freccia verso il target, relativa allo sguardo (lato destro della barra).
            if let Some(t) = target {
                let to = t - player_pos;
                let world_angle = to.x.atan2(-to.z); // radianti, 0 = -Z (N)
                let rel = world_angle - heading_deg.to_radians();
                let r = 12.0_f32;
                let c = egui::pos2(rect.right() - 18.0, rect.center().y);
                let dir = egui::vec2(rel.sin(), -rel.cos());
                let tip = c + dir * r;
                let left = c + egui::vec2(-dir.y, dir.x) * 5.0;
                let right = c + egui::vec2(dir.y, -dir.x) * 5.0;
                painter.add(egui::Shape::convex_polygon(
                    vec![tip, left, right],
                    egui::Color32::from_rgb(255, 80, 60),
                    egui::Stroke::new(1.0, egui::Color32::WHITE),
                ));
            }

            // Coordinate del player sotto la barra.
            ui.label(
                egui::RichText::new(format!(
                    "X {:6.1}   Z {:6.1}",
                    player_pos.x, player_pos.z
                ))
                .monospace()
                .size(12.0)
                .color(egui::Color32::from_rgb(200, 210, 230)),
            );
        });
}

/// Etichetta HUD leggibile: testo chiaro e grande su sfondo scuro semi-opaco.
fn hud_chip(ui: &mut egui::Ui, text: &str) {
    egui::Frame::default()
        .fill(egui::Color32::from_rgba_unmultiplied(10, 14, 22, 215))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .size(15.0)
                    .strong()
                    .color(egui::Color32::from_rgb(245, 248, 255)),
            );
        });
}

/// Banner titolo quest in alto al centro (es. "Denaro Rubato").
fn draw_title_banner(ctx: &egui::Context, text: &str) {
    let size = ctx.viewport_rect().size();
    egui::Area::new(egui::Id::new("quest_title"))
        .fixed_pos(egui::pos2(size.x / 2.0 - 200.0, 70.0))
        .show(ctx, |ui| {
            let (rect, _resp) =
                ui.allocate_exact_size(egui::vec2(400.0, 44.0), egui::Sense::hover());
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(6),
                egui::Color32::from_rgba_unmultiplied(20, 14, 8, 225),
            );
            ui.painter().rect_stroke(
                rect,
                egui::CornerRadius::same(6),
                egui::Stroke::new(1.5, egui::Color32::from_rgb(230, 190, 80)),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(20.0),
                egui::Color32::from_rgb(240, 210, 120),
            );
        });
}

/// Pannello obiettivi (tenendo premuto TAB): checklist stage quest.
fn draw_objectives(ctx: &egui::Context, quest: &QuestState) {
    let size = ctx.viewport_rect().size();
    egui::Area::new(egui::Id::new("objectives"))
        .fixed_pos(egui::pos2(size.x - 260.0, size.y / 2.0 - 90.0))
        .show(ctx, |ui| {
            let (rect, _resp) =
                ui.allocate_exact_size(egui::vec2(240.0, 180.0), egui::Sense::hover());
            let frame = egui::Frame::default()
                .fill(egui::Color32::from_rgba_unmultiplied(12, 16, 24, 230))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 90, 120)))
                .inner_margin(egui::Margin::symmetric(12, 10))
                .corner_radius(egui::CornerRadius::same(6));
            frame.show(ui, |ui| {
                ui.set_min_width(rect.width());
                ui.label(
                    egui::RichText::new("Obiettivi — Denaro Rubato")
                        .strong()
                        .size(15.0),
                );
                ui.add_space(6.0);
                let mark = |done: bool| if done { "[✓]" } else { "[ ]" };
                let thief_done = quest.stage == QuestStage::Collecting
                    || quest.stage == QuestStage::Returning
                    || quest.stage == QuestStage::Completed;
                let coins_done =
                    quest.stage == QuestStage::Returning || quest.stage == QuestStage::Completed;
                let ret_done = quest.stage == QuestStage::Completed;
                ui.label(format!("{} Vai dal ladro", mark(quest.accepted)));
                ui.label(format!("{} Uccidi il ladro", mark(thief_done)));
                ui.label(format!(
                    "{} Monete {}/{}",
                    mark(coins_done),
                    quest.coins_collected,
                    quest.coins_total
                ));
                ui.label(format!("{} Torna dal Viandante", mark(ret_done)));
            });
        });
}
