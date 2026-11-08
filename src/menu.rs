//! Menu Esc (egui): toggle pausa, lista tasti, slider sensibilità, reset scena e FPS overlay.

use crate::{
    AppState,
    camera::OrbitCamera,
    entities::{DialogState, NearestInteract},
    hud,
    inventory::Inventory,
    map,
    player::PlayerHealth,
    quests::{
        Barks, ObjState, QuestEntry, QuestStatus, QuestTexts,
        denaro_rubato::{QuestStage, QuestState},
    },
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
    // J: diario, T: "Aspetta" (non durante un dialogo).
    if !dialog.open && !matches!(*state.get(), AppState::Chat | AppState::Shop) {
        let (key, panel) = if keys.just_pressed(KeyCode::KeyJ) {
            (true, AppState::Journal)
        } else if keys.just_pressed(KeyCode::KeyT) {
            (true, AppState::Wait)
        } else if keys.just_pressed(KeyCode::KeyM) {
            (true, AppState::Map)
        } else {
            (false, AppState::Playing)
        };
        if key {
            if *state.get() == AppState::Playing {
                next.set(panel);
            } else if *state.get() == panel {
                next.set(AppState::Playing);
            }
            return;
        }
    }
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    // Se un dialogo è aperto, Esc lo chiude (gestito da advance_dialog):
    // qui non apriamo il menu. `block_interact` copre il caso in cui il dialogo
    // sia già stato chiuso da advance_dialog nello stesso frame.
    if dialog.open || dialog.block_interact {
        return;
    }
    match state.get() {
        AppState::Playing => next.set(AppState::Paused),
        AppState::Paused
        | AppState::Journal
        | AppState::Wait
        | AppState::Map
        | AppState::Chat
        | AppState::Shop => next.set(AppState::Playing),
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
            Entity,
            &'static GlobalTransform,
            &'static crate::combat::EnemyHealth,
            Option<&'static crate::quests::denaro_rubato::QuestThief>,
        ),
        With<crate::entities::Enemy>,
    >,
    pub cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform)>,
    pub transforms: Query<'w, 's, &'static GlobalTransform>,
    pub npcs: Query<
        'w,
        's,
        (
            Entity,
            &'static GlobalTransform,
            &'static crate::entities::Npc,
            &'static Visibility,
        ),
    >,
    pub anchors: Query<'w, 's, &'static crate::hud::NameAnchor>,
    pub styled: Local<'s, bool>,
    pub audio: ResMut<'w, crate::audio::AudioSettings>,
    pub chat: ResMut<'w, crate::ai::AiChat>,
    pub ai: Res<'w, crate::ai::AiState>,
    pub weather: Res<'w, crate::weather::WeatherState>,
    pub tts: Res<'w, crate::ai::tts::TtsState>,
    pub village: Res<'w, crate::village::VillageInfo>,
    pub equip: Res<'w, crate::inventory::Equipment>,
    pub shop: ResMut<'w, crate::shop::ShopState>,
    pub occluded: Res<'w, hud::Occluded>,
    pub map_view: Res<'w, map::MapView>,
    pub clock: ResMut<'w, crate::daynight::GameClock>,
    pub wait: ResMut<'w, crate::daynight::WaitUi>,
    pub player_pos: Query<'w, 's, &'static GlobalTransform, With<crate::player::Player>>,
    pub thief_pos: Query<
        'w,
        's,
        &'static GlobalTransform,
        (
            With<crate::quests::denaro_rubato::QuestThief>,
            Without<crate::player::Player>,
        ),
    >,
    pub coins_pos:
        Query<'w, 's, &'static GlobalTransform, With<crate::quests::denaro_rubato::QuestCoin>>,
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
    texts: Res<QuestTexts>,
    barks: Res<Barks>,
    mut tab: ResMut<JournalTab>,
    mut q: UiQueries,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return; // Contesto egui non ancora pronto (primi frame).
    };
    // Tema condiviso (colori e dimensioni dell'HUD) applicato una volta sola.
    if !*q.styled {
        crate::ui_style::apply_style(ctx);
        *q.styled = true;
    }

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
                .fixed_pos(egui::pos2(12.0, 58.0))
                .show(ctx, |ui| {
                    hud_chip(ui, &format!("Monete: {coins}"));
                });
            egui::Area::new(egui::Id::new("weather"))
                .fixed_pos(egui::pos2(12.0, 150.0))
                .show(ctx, |ui| {
                    hud_chip(ui, &q.weather.label());
                });
            egui::Area::new(egui::Id::new("clock"))
                .fixed_pos(egui::pos2(12.0, 104.0))
                .show(ctx, |ui| {
                    hud_chip(
                        ui,
                        &format!("{} — {}", q.clock.date_label(), q.clock.label()),
                    );
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
                hud::draw_hud(ctx, hp, &inventory, &q.equip);
            }

            // Barre vita dei nemici sopra le loro teste (proiezione world→screen).
            if let Ok(pp) = q.player_pos.single() {
                hud::draw_enemy_bars(
                    ctx,
                    &q.enemies,
                    &q.cameras,
                    pp.translation(),
                    &barks,
                    &q.occluded,
                );
                hud::draw_npc_labels(
                    ctx,
                    &q.npcs,
                    &q.cameras,
                    pp.translation(),
                    &barks,
                    &q.occluded,
                );
            }

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
                draw_coords(ctx, pp.translation());
            }
            if let Some((text, _t)) = &quest.title {
                draw_title_banner(ctx, text);
            }
            // Obiettivi (TAB): solo a quest accettata e non ancora completata.
            if keys.pressed(KeyCode::Tab)
                && quest.accepted
                && quest.stage != QuestStage::Completed
                && let Some(entry) = crate::quests::journal_entries(&quest, &texts)
                    .into_iter()
                    .find(|e| e.status == QuestStatus::Active)
            {
                draw_objectives(ctx, &entry);
            }
            draw_barks(ctx, &barks, &q.transforms, &q.cameras, &q.anchors);

            // Contatore FPS in alto a destra (se attivato dal menu).
            if show_fps.0
                && let Some(fps) = diagnostics
                    .get(&FrameTimeDiagnosticsPlugin::FPS)
                    .and_then(|d| d.smoothed())
            {
                let size = ctx.viewport_rect().size();
                egui::Area::new(egui::Id::new("fps"))
                    .pivot(egui::Align2::RIGHT_TOP)
                    .fixed_pos(egui::pos2(size.x - 12.0, 12.0))
                    .show(ctx, |ui| {
                        hud_chip(ui, &format!("FPS: {:.0}", fps));
                    });
            }
        }
        AppState::Paused => {
            // Versione del gioco in basso a destra.
            egui::Area::new(egui::Id::new("game_version"))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
                .show(ctx, |ui| {
                    hud_chip(ui, &format!("Planet One {}", crate::version::GAME_VERSION));
                });

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
                .fill(egui::Color32::from_rgba_unmultiplied(8, 10, 15, 225))
                .inner_margin(egui::Margin::symmetric(40, 24));

            egui::CentralPanel::default()
                .frame(frame)
                .show(&mut viewport_ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("Planet One")
                                    .strong()
                                    .size(34.0)
                                    .color(crate::ui_style::GOLD),
                            );
                            ui.label("— pausa —");
                        });
                        ui.separator();

                        // Lista dei controlli disponibili.
                        ui.label(
                            egui::RichText::new("Controlli")
                                .strong()
                                .color(crate::ui_style::GOLD),
                        );
                        ui.label("• W/A/S/D o frecce — movimento");
                        ui.label("• Shift — corsa");
                        ui.label("• Space — salto");
                        ui.label("• Mouse — orbita camera");
                        ui.label("• Esc — menu / riprendi");
                        ui.label("• J — diario delle quest, T — aspetta, TAB (tenuto) — obiettivi");
                        ui.label("• E / Invio — dialogo, 1–0 — usa oggetti");
                        ui.label("• F3 — wireframe collider (debug)");
                        ui.label("• F11 — schermo intero on/off");
                        ui.label(
                            "• V — prima persona on/off, RMB tenuto = prima persona temporanea",
                        );
                        ui.separator();

                        // Sensibilità del mouse: modifica la resource in tempo reale.
                        ui.add(
                            egui::Slider::new(&mut orbit.sensitivity, 0.0005..=0.010)
                                .text("Sensibilità mouse"),
                        );

                        // Volume delle sole musiche (salvato su file).
                        let mut pct = q.audio.music_volume * 100.0;
                        if ui
                            .add(
                                egui::Slider::new(&mut pct, 0.0..=100.0)
                                    .text("Volume musica %")
                                    .fixed_decimals(0),
                            )
                            .changed()
                        {
                            q.audio.music_volume = pct / 100.0;
                            q.audio.dirty = true;
                        }

                        // Volume delle voci dei personaggi.
                        let mut vpct = q.audio.voice_volume * 100.0;
                        if ui
                            .add(
                                egui::Slider::new(&mut vpct, 0.0..=100.0)
                                    .text("Volume voci %")
                                    .fixed_decimals(0),
                            )
                            .changed()
                        {
                            q.audio.voice_volume = vpct / 100.0;
                            q.audio.dirty = true;
                        }

                        // Toggle contatore FPS.
                        ui.checkbox(&mut show_fps.0, "Mostra FPS");
                        ui.horizontal(|ui| {
                            ui.label("Velocità del tempo:");
                            for speed in [1u32, 2, 4, 8] {
                                if ui
                                    .selectable_label(
                                        q.audio.time_speed == speed,
                                        format!("x{speed}"),
                                    )
                                    .clicked()
                                {
                                    q.audio.time_speed = speed;
                                    q.audio.dirty = true;
                                }
                            }
                        });
                        ui.separator();

                        // Dialoghi con IA locale (Ollama, solo su questo PC).
                        ui.label(
                            egui::RichText::new("Dialoghi con IA (locale)")
                                .strong()
                                .color(crate::ui_style::GOLD),
                        );
                        if ui
                            .checkbox(&mut q.audio.ai_enabled, "Dialoghi IA (spegni sui PC lenti)")
                            .changed()
                        {
                            q.audio.dirty = true;
                        }
                        ui.label(format!("Stato: {}", q.ai.status.label()));
                        if ui
                            .checkbox(&mut q.audio.tts_enabled, "Voce IA (Piper locale)")
                            .changed()
                        {
                            q.audio.dirty = true;
                        }
                        ui.label(format!("Voce: {}", q.tts.status.label()));
                        if !q.tts.voices.is_empty() {
                            let current = if q.audio.tts_voice.is_empty() {
                                "automatica (dal personaggio)".to_string()
                            } else {
                                q.audio.tts_voice.clone()
                            };
                            egui::ComboBox::from_label("Voce")
                                .selected_text(current)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            q.audio.tts_voice.is_empty(),
                                            "automatica (dal personaggio)",
                                        )
                                        .clicked()
                                    {
                                        q.audio.tts_voice.clear();
                                        q.audio.dirty = true;
                                    }
                                    for v in q.tts.voices.clone() {
                                        if ui.selectable_label(q.audio.tts_voice == v, &v).clicked()
                                        {
                                            q.audio.tts_voice = v;
                                            q.audio.dirty = true;
                                        }
                                    }
                                });
                        }
                        ui.horizontal(|ui| {
                            ui.label("Modello:");
                            if ui.text_edit_singleline(&mut q.audio.ai_model).lost_focus() {
                                q.audio.dirty = true;
                            }
                        });
                        ui.separator();

                        // Riprendi (equivalente a Esc).
                        let btn = |t: &str| egui::Button::new(egui::RichText::new(t).strong());
                        let bsize = egui::vec2(240.0, 40.0);
                        if ui.add_sized(bsize, btn("Riprendi")).clicked() {
                            next.set(AppState::Playing);
                        }

                        // Reset della scena: ricostruzione da zero.
                        // Entra nello stato transitorio `Resetting`, in cui tutti i
                        // sistemi di gameplay sono spenti; questo evita il panic
                        // "Entity despawned" che accadeva quando il reset era
                        // eseguito in `Update` insieme al ground-check.
                        if ui.add_sized(bsize, btn("Reset scena")).clicked() {
                            next.set(AppState::Resetting);
                        }

                        ui.add_space(8.0);
                        // Esci dal gioco: chiude la finestra con AppExit::Success.
                        if ui.add_sized(bsize, btn("Esci dal gioco")).clicked() {
                            info!("[EVENT] richiesta uscita dal gioco");
                            exit.write(AppExit::Success);
                        }
                    });
                });
        }
        AppState::Wait => {
            let mut wait_now = false;
            let mut close = false;
            egui::Window::new("Aspetta")
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} — {}",
                            q.clock.date_label(),
                            q.clock.label()
                        ))
                        .size(18.0),
                    );
                    ui.add_space(6.0);
                    ui.add(egui::Slider::new(&mut q.wait.hours, 1..=24).text("ore da attendere"));
                    let mut after = q.clock.clone();
                    after.advance(q.wait.hours as f32);
                    ui.label(format!(
                        "Dopo l'attesa: {} — {}",
                        after.date_label(),
                        after.label()
                    ));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Aspetta").clicked() {
                            wait_now = true;
                        }
                        if ui.button("Annulla").clicked() {
                            close = true;
                        }
                    });
                    ui.label("T / Esc = chiudi");
                });
            if wait_now {
                q.clock.advance(q.wait.hours as f32);
                info!(
                    "[TIME] attesa di {} h: ora {}",
                    q.wait.hours,
                    q.clock.label()
                );
                next.set(AppState::Playing);
            } else if close {
                next.set(AppState::Playing);
            }
        }
        AppState::Map => {
            let heading = (-orbit.yaw).to_degrees().rem_euclid(360.0);
            if let Ok(pp) = q.player_pos.single() {
                map::draw_map_overlay(
                    ctx,
                    q.cameras.single().ok(),
                    pp.translation(),
                    heading,
                    q.npc_pos.single().ok().map(|t| t.translation()),
                    (q.village.poly.len() >= 3).then(|| {
                        let north = q.village.poly.iter().map(|p| p.y).fold(f32::MAX, f32::min);
                        (
                            q.village.spec.name.as_str(),
                            Vec3::new(q.village.center.x, 0.0, north - 10.0),
                        )
                    }),
                    q.map_view.zoom,
                );
            }
        }
        AppState::Journal => {
            let entries = crate::quests::journal_entries(&quest, &texts);
            draw_journal(ctx, &entries, &mut tab);
        }
        AppState::Shop => {
            let coins = inventory.qty(crate::inventory::ItemKind::Coin);
            crate::shop::shop_panel(ctx, &mut q.shop, coins);
        }
        AppState::Chat => {
            let status = q.ai.status.clone();
            crate::ai::ui::chat_panel(ctx, &mut q.chat, &status);
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
    if dialog.open {
        return;
    }
    let Some(prompt) = nearest_prompt else {
        return;
    };
    let size = ctx.viewport_rect().size();
    // Il tasto fra parentesi quadre (es. "[E]") in oro, il resto in bianco.
    let (key, action) = match prompt.split_once("] ") {
        Some((k, rest)) => (format!("{k}]"), rest.to_string()),
        None => (String::new(), prompt.clone()),
    };
    let font = egui::FontId::proportional(26.0);
    let painter = ctx.layer_painter(egui::LayerId::background());
    let mut job = egui::text::LayoutJob::default();
    if !key.is_empty() {
        job.append(
            &format!("{key} "),
            0.0,
            egui::TextFormat::simple(font.clone(), crate::ui_style::GOLD),
        );
    }
    job.append(
        &action,
        0.0,
        egui::TextFormat::simple(font, crate::ui_style::TEXT),
    );
    let galley = painter.layout_job(job);
    let pad = egui::vec2(16.0, 8.0);
    let rect = egui::Rect::from_center_size(
        egui::pos2(size.x / 2.0, size.y - 120.0),
        galley.size() + pad * 2.0,
    );
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(8),
        crate::ui_style::PANEL_FILL,
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(8),
        egui::Stroke::new(2.0, crate::ui_style::GOLD_EDGE),
        egui::StrokeKind::Inside,
    );
    painter.galley(rect.min + pad, galley, crate::ui_style::TEXT);
}

/// Pannello del dialogo in basso, stile RPG: nome parlante + riga corrente +
/// hint per avanzare/chiudere.
fn show_dialog_panel(ctx: &egui::Context, dialog: &DialogState) {
    let size = ctx.viewport_rect().size();
    let panel_w = 780.0_f32.min(size.x - 40.0);
    let panel_h = if dialog.choosing { 250.0 } else { 190.0 };
    let gold = egui::Color32::from_rgb(255, 214, 102);
    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgba_unmultiplied(10, 12, 20, 250))
        .stroke(egui::Stroke::new(2.0, gold))
        .inner_margin(egui::Margin::symmetric(26, 18))
        .corner_radius(egui::CornerRadius::same(10));

    egui::Area::new(egui::Id::new("dialog_panel"))
        .fixed_pos(egui::pos2(
            size.x / 2.0 - panel_w / 2.0,
            size.y - panel_h - 48.0,
        ))
        .show(ctx, |ui| {
            ui.set_width(panel_w);
            ui.set_min_height(panel_h);
            frame.show(ui, |ui| {
                ui.set_min_width(panel_w - 52.0);
                // Nome del parlante: grande e colorato.
                ui.label(
                    egui::RichText::new(&dialog.speaker)
                        .strong()
                        .size(26.0)
                        .color(gold),
                );
                ui.add_space(8.0);
                // Riga corrente: testo grande, bianco pieno.
                if let Some(line) = dialog.lines.get(dialog.index) {
                    ui.label(
                        egui::RichText::new(line)
                            .size(23.0)
                            .color(egui::Color32::from_rgb(250, 252, 255)),
                    );
                }
                // Scelte (dopo l'ultima riga): quella evidenziata è in oro.
                if dialog.choosing {
                    ui.add_space(12.0);
                    for (i, choice) in dialog.choices.iter().enumerate() {
                        let selected = i == dialog.selected;
                        let (prefix, color) = if selected {
                            ("> ", gold)
                        } else {
                            ("   ", egui::Color32::from_rgb(190, 198, 215))
                        };
                        let mut text = egui::RichText::new(format!("{prefix}{choice}"))
                            .size(22.0)
                            .color(color);
                        if selected {
                            text = text.strong();
                        }
                        ui.label(text);
                    }
                }
                ui.add_space(10.0);
                let hint = if dialog.choosing {
                    "Su/Giù = scegli   •   E / Invio = conferma   •   Esc = chiudi"
                } else {
                    "E / Invio = continua   •   Esc = chiudi"
                };
                ui.label(
                    egui::RichText::new(hint)
                        .size(15.0)
                        .color(egui::Color32::from_rgb(175, 185, 205)),
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
            With<crate::quests::denaro_rubato::QuestThief>,
            Without<crate::player::Player>,
        ),
    >,
    coins: &Query<&GlobalTransform, With<crate::quests::denaro_rubato::QuestCoin>>,
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
    const BAR_H: f32 = 52.0;
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

            // Indicatore del target sulla tacca del suo bearing (stesso asse
            // dei gradi): allineato alla posizione reale. Fuori dalla finestra
            // di ±45° resta sul bordo, con la punta rivolta verso il target.
            if let Some(t) = target {
                let to = t - player_pos;
                let bearing = to.x.atan2(-to.z).to_degrees().rem_euclid(360.0);
                let mut off = (bearing - heading_deg).rem_euclid(360.0);
                if off > 180.0 {
                    off -= 360.0;
                }
                let inside = off.abs() <= WIN_HALF_DEG;
                let x = mid_x + off.clamp(-WIN_HALF_DEG, WIN_HALF_DEG) * px_per_deg;
                let red = egui::Color32::from_rgb(255, 80, 60);
                let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                if inside {
                    // Linea verticale sulla tacca + triangolo che punta in alto.
                    painter.line_segment(
                        [
                            egui::pos2(x, rect.top() + 4.0),
                            egui::pos2(x, rect.bottom() - 12.0),
                        ],
                        egui::Stroke::new(1.5, red),
                    );
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            egui::pos2(x, rect.bottom() - 12.0),
                            egui::pos2(x - 6.0, rect.bottom() - 2.0),
                            egui::pos2(x + 6.0, rect.bottom() - 2.0),
                        ],
                        red,
                        stroke,
                    ));
                } else {
                    // Target fuori vista: freccia sul bordo verso il lato giusto.
                    let cy = rect.center().y;
                    let s = if off > 0.0 { 1.0 } else { -1.0 };
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            egui::pos2(x + s * 6.0, cy),
                            egui::pos2(x - s * 4.0, cy - 7.0),
                            egui::pos2(x - s * 4.0, cy + 7.0),
                        ],
                        red,
                        stroke,
                    ));
                }
            }
        });
}

/// Coordinate x,z del player in un chip centrato sotto la bussola.
fn draw_coords(ctx: &egui::Context, player_pos: Vec3) {
    let size = ctx.viewport_rect().size();
    egui::Area::new(egui::Id::new("coords"))
        .pivot(egui::Align2::CENTER_TOP)
        .fixed_pos(egui::pos2(size.x / 2.0, 70.0))
        .show(ctx, |ui| {
            hud_chip(
                ui,
                &format!(
                    "X {:.1}   Y {:.1}   Z {:.1}",
                    player_pos.x,
                    player_pos.y - 0.85, // quota dei piedi (centro capsula - metà altezza)
                    player_pos.z
                ),
            );
        });
}

/// Etichetta HUD leggibile: testo chiaro e grande su sfondo scuro semi-opaco.
fn hud_chip(ui: &mut egui::Ui, text: &str) {
    egui::Frame::default()
        .fill(egui::Color32::from_rgba_unmultiplied(8, 11, 18, 245))
        .stroke(egui::Stroke::new(
            1.5,
            egui::Color32::from_rgb(170, 180, 205),
        ))
        .corner_radius(egui::CornerRadius::same(7))
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(text)
                        .size(19.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .wrap_mode(egui::TextWrapMode::Extend),
            );
        });
}

/// Banner titolo quest (es. "Denaro Rubato"), sotto bussola e coordinate.
fn draw_title_banner(ctx: &egui::Context, text: &str) {
    let size = ctx.viewport_rect().size();
    let painter = ctx.layer_painter(egui::LayerId::background());
    let galley = painter.layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(24.0),
        crate::ui_style::GOLD,
    );
    let pad = egui::vec2(24.0, 10.0);
    let rect = egui::Rect::from_center_size(
        egui::pos2(
            size.x / 2.0,
            (size.y * 0.20).max(150.0) + galley.size().y / 2.0,
        ),
        galley.size() + pad * 2.0,
    );
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(8),
        crate::ui_style::PANEL_FILL,
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(8),
        egui::Stroke::new(2.0, crate::ui_style::GOLD_EDGE),
        egui::StrokeKind::Inside,
    );
    painter.galley(rect.min + pad, galley, crate::ui_style::GOLD);
}

/// Stato di un obiettivo della quest nel pannello TAB.
/// Disegna l'icona di stato di un obiettivo (con il painter, senza emoji:
/// i font di egui non garantiscono quei glifi).
fn draw_obj_icon(painter: &egui::Painter, c: egui::Pos2, state: ObjState) {
    let r = 9.0;
    match state {
        ObjState::Done => {
            painter.circle_filled(c, r, egui::Color32::from_rgb(52, 190, 90));
            painter.add(egui::Shape::line(
                vec![
                    egui::pos2(c.x - 4.5, c.y + 0.5),
                    egui::pos2(c.x - 1.0, c.y + 4.0),
                    egui::pos2(c.x + 5.0, c.y - 3.5),
                ],
                egui::Stroke::new(2.4, egui::Color32::WHITE),
            ));
        }
        ObjState::Current => {
            painter.circle_filled(c, r, egui::Color32::from_rgb(56, 150, 255));
            painter.circle_stroke(
                c,
                r,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(190, 225, 255)),
            );
        }
        ObjState::Pending => {
            painter.circle_stroke(
                c,
                r - 1.0,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 130, 150)),
            );
        }
        ObjState::Failed => {
            let red = egui::Color32::from_rgb(235, 60, 55);
            let d = 5.5;
            painter.line_segment(
                [egui::pos2(c.x - d, c.y - d), egui::pos2(c.x + d, c.y + d)],
                egui::Stroke::new(3.0, red),
            );
            painter.line_segment(
                [egui::pos2(c.x - d, c.y + d), egui::pos2(c.x + d, c.y - d)],
                egui::Stroke::new(3.0, red),
            );
        }
    }
}

/// Pannello obiettivi (tenendo premuto TAB) della quest attiva: icone di stato
/// (spunta verde = completato, sfera azzurra = in corso, X rossa = fallito).
fn draw_objectives(ctx: &egui::Context, entry: &QuestEntry) {
    let size = ctx.viewport_rect().size();
    egui::Area::new(egui::Id::new("objectives"))
        .fixed_pos(egui::pos2(size.x - 330.0, size.y / 2.0 - 90.0))
        .show(ctx, |ui| {
            let frame = egui::Frame::default()
                .fill(egui::Color32::from_rgba_unmultiplied(12, 16, 24, 240))
                .stroke(egui::Stroke::new(
                    1.5,
                    egui::Color32::from_rgb(230, 190, 90),
                ))
                .inner_margin(egui::Margin::symmetric(16, 12))
                .corner_radius(egui::CornerRadius::same(8));
            frame.show(ui, |ui| {
                ui.set_min_width(268.0);
                ui.label(
                    egui::RichText::new(format!("Obiettivi — {}", entry.title))
                        .strong()
                        .size(17.0)
                        .color(egui::Color32::from_rgb(255, 214, 102)),
                );
                ui.add_space(8.0);
                objective_rows(ui, &entry.objectives, 268.0, 16.0);
            });
        });
}

/// Righe obiettivo con icona di stato.
fn objective_rows(ui: &mut egui::Ui, objectives: &[(String, ObjState)], w: f32, font: f32) {
    for (text, st) in objectives {
        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(w, 28.0), egui::Sense::hover());
        draw_obj_icon(
            ui.painter(),
            egui::pos2(rect.left() + 11.0, rect.center().y),
            *st,
        );
        let color = match st {
            ObjState::Pending => egui::Color32::from_rgb(150, 158, 178),
            _ => egui::Color32::from_rgb(245, 248, 255),
        };
        ui.painter().text(
            egui::pos2(rect.left() + 30.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional(font),
            color,
        );
    }
}

/// Tab selezionato del diario.
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum JournalTab {
    #[default]
    Todo,
    Done,
}

/// Diario delle quest (tasto J): due tab, ogni quest espandibile.
fn draw_journal(ctx: &egui::Context, entries: &[QuestEntry], tab: &mut JournalTab) {
    let todo = entries
        .iter()
        .filter(|e| e.status == QuestStatus::Active)
        .count();
    let done = entries.len() - todo;
    egui::Window::new("Diario")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .fixed_size(egui::vec2(600.0, 440.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(tab, JournalTab::Todo, format!("Da fare ({todo})"));
                ui.selectable_value(tab, JournalTab::Done, format!("Completate ({done})"));
            });
            ui.separator();
            let wanted = if *tab == JournalTab::Todo {
                QuestStatus::Active
            } else {
                QuestStatus::Completed
            };
            let shown: Vec<&QuestEntry> = entries.iter().filter(|e| e.status == wanted).collect();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if shown.is_empty() {
                    ui.label("Nessuna quest.");
                }
                for e in shown {
                    egui::CollapsingHeader::new(
                        egui::RichText::new(&e.title)
                            .size(19.0)
                            .color(egui::Color32::from_rgb(255, 214, 102)),
                    )
                    .id_salt(&e.title)
                    .show(ui, |ui| {
                        ui.label(&e.summary);
                        ui.add_space(6.0);
                        objective_rows(ui, &e.objectives, 540.0, 17.0);
                    });
                }
            });
            ui.separator();
            ui.label("J / Esc = chiudi");
        });
}

/// Fumetti sopra la testa di chi parla (proiezione world→screen).
fn draw_barks(
    ctx: &egui::Context,
    barks: &Barks,
    transforms: &Query<&GlobalTransform>,
    cameras: &Query<(&Camera, &GlobalTransform)>,
    anchors: &Query<&crate::hud::NameAnchor>,
) {
    let Ok((camera, cam_t)) = cameras.single() else {
        return;
    };
    for (i, b) in barks.0.iter().enumerate() {
        let Ok(t) = transforms.get(b.entity) else {
            continue;
        };
        // Il fumetto sta SOTTO la targhetta del nome (stesso punto di ancoraggio).
        let h = anchors.get(b.entity).map_or(1.3, |a| a.0);
        let Ok(screen) = camera.world_to_viewport(cam_t, t.translation() + Vec3::Y * h) else {
            continue;
        };
        egui::Area::new(egui::Id::new(("bark", i)))
            .pivot(egui::Align2::CENTER_TOP)
            .fixed_pos(egui::pos2(screen.x, screen.y + 8.0))
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgba_unmultiplied(14, 18, 28, 235))
                    .stroke(egui::Stroke::new(
                        1.5,
                        egui::Color32::from_rgb(230, 190, 90),
                    ))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .corner_radius(egui::CornerRadius::same(8))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&b.speaker)
                                .strong()
                                .size(14.0)
                                .color(egui::Color32::from_rgb(255, 214, 102)),
                        );
                        ui.label(
                            egui::RichText::new(&b.text)
                                .size(18.0)
                                .color(egui::Color32::WHITE),
                        );
                    });
            });
    }
}
