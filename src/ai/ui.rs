//! Pannello egui della chat con l'NPC (stato `AppState::Chat`).

use super::{AiChat, AiStatus, ChatAction};
use bevy_egui::egui;

const GOLD: egui::Color32 = egui::Color32::from_rgb(255, 214, 102);

/// Disegna la chat in basso, stile del pannello dei dialoghi. Scrive l'azione scelta in `chat.action`.
pub fn chat_panel(ctx: &egui::Context, chat: &mut AiChat, status: &AiStatus) {
    let size = ctx.viewport_rect().size();
    let w = 820.0_f32.min(size.x - 40.0);
    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgba_unmultiplied(10, 12, 20, 250))
        .stroke(egui::Stroke::new(2.0, GOLD))
        .inner_margin(egui::Margin::symmetric(26, 18))
        .corner_radius(egui::CornerRadius::same(10));

    egui::Area::new(egui::Id::new("ai_chat_panel"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -40.0))
        .show(ctx, |ui| {
            ui.set_width(w);
            frame.show(ui, |ui| {
                ui.set_min_width(w - 52.0);
                ui.label(
                    egui::RichText::new(&chat.npc_name)
                        .strong()
                        .size(26.0)
                        .color(GOLD),
                );
                ui.add_space(6.0);
                // Ultima battuta dell'NPC (o stato).
                if chat.thinking {
                    let dots = ".".repeat(1 + (ctx.input(|i| i.time) * 3.0) as usize % 3);
                    ui.label(
                        egui::RichText::new(format!("{} sta pensando{dots}", chat.npc_name))
                            .italics()
                            .size(21.0)
                            .color(egui::Color32::from_rgb(190, 198, 215)),
                    );
                    ctx.request_repaint();
                } else if let Some(err) = &chat.error {
                    ui.label(
                        egui::RichText::new(err)
                            .size(21.0)
                            .color(egui::Color32::from_rgb(255, 140, 120)),
                    );
                }
                if let Some(last) = chat.history.iter().rev().find(|t| !t.player) {
                    ui.label(
                        egui::RichText::new(&last.text)
                            .size(23.0)
                            .color(egui::Color32::from_rgb(250, 252, 255)),
                    );
                }
                // Ultima cosa detta dal giocatore, discreta.
                if let Some(me) = chat.history.last().filter(|t| t.player) {
                    ui.label(
                        egui::RichText::new(format!("Tu: {}", me.text))
                            .size(17.0)
                            .color(egui::Color32::from_rgb(160, 170, 190)),
                    );
                }
                ui.add_space(10.0);

                // Repliche suggerite dalla IA.
                let mut chosen: Option<String> = None;
                if !chat.thinking {
                    for (i, opt) in chat.options.iter().enumerate() {
                        let label = format!("{}  {}", i + 1, opt);
                        if ui
                            .add(egui::Button::new(egui::RichText::new(label).size(20.0)))
                            .clicked()
                        {
                            chosen = Some(opt.clone());
                        }
                    }
                    let typing = ctx.memory(|m| m.focused()).is_some();
                    if !typing {
                        for (i, key) in [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3]
                            .into_iter()
                            .enumerate()
                        {
                            if ctx.input(|inp| inp.key_pressed(key))
                                && let Some(o) = chat.options.get(i)
                            {
                                chosen = Some(o.clone());
                            }
                        }
                    }
                }
                ui.add_space(8.0);
                // Testo libero.
                let mut send_text = false;
                ui.horizontal(|ui| {
                    let resp = ui.add_enabled(
                        !chat.thinking,
                        egui::TextEdit::singleline(&mut chat.input)
                            .hint_text("Scrivi quello che vuoi dire…")
                            .desired_width(w - 190.0)
                            .font(egui::FontId::proportional(20.0)),
                    );
                    if chat.focus_input && !chat.thinking {
                        resp.request_focus();
                        chat.focus_input = false;
                    }
                    let enter = resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter));
                    if enter {
                        resp.request_focus();
                    }
                    if ui
                        .add_enabled(
                            !chat.thinking && !chat.input.trim().is_empty(),
                            egui::Button::new(egui::RichText::new("Invia").size(20.0)),
                        )
                        .clicked()
                        || (enter && !chat.input.trim().is_empty())
                    {
                        send_text = true;
                    }
                });
                if send_text {
                    chosen = Some(chat.input.clone());
                }
                if let Some(text) = chosen {
                    chat.action = Some(ChatAction::Send(text));
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(egui::RichText::new("Arrivederci").size(18.0)))
                        .clicked()
                    {
                        chat.action = Some(ChatAction::Close);
                    }
                    ui.label(
                        egui::RichText::new(format!(
                            "1–3 o clic = rispondi   •   Invio = invia   •   Esc = chiudi   •   IA locale: {}",
                            status.label()
                        ))
                        .size(14.0)
                        .color(egui::Color32::from_rgb(175, 185, 205)),
                    );
                });
            });
        });
}
