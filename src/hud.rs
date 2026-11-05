//! HUD di gioco (Fase 4.1): barra vita, crosshair, inventario in basso.
//!
//! Tutto disegnato con egui Area dentro `EguiPrimaryContextPass`, così non
//! servono caméra 2D né노 Text2d (coordinate world instabili con la camera orbit).

use crate::inventory::Inventory;
use crate::player::PlayerHealth;
use bevy::prelude::*;
use bevy_egui::egui;

/// Disegna l'HUD in `Playing`: barra vita in alto a sinistra, crosshair al
/// centro, slot inventario in basso al centro.
pub fn draw_hud(ctx: &egui::Context, health: &PlayerHealth, inventory: &Inventory) {
    let size = ctx.viewport_rect().size();

    // --- Barra vita (alto a sinistra) ---
    let bar_w = 220.0;
    let bar_h = 18.0;
    let pos = egui::pos2(16.0, 16.0);
    egui::Area::new(egui::Id::new("health_bar"))
        .fixed_pos(pos)
        .show(ctx, |ui| {
            let (rect, _resp) =
                ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
            // Sfondo scuro.
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(3),
                egui::Color32::from_rgba_unmultiplied(20, 20, 25, 220),
            );
            // Riempimento proporzionale alla vita.
            let frac = (health.current / health.max).clamp(0.0, 1.0);
            let fill_w = bar_w * frac;
            let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(fill_w, bar_h));
            let color = if frac > 0.5 {
                egui::Color32::from_rgb(60, 200, 80)
            } else if frac > 0.25 {
                egui::Color32::from_rgb(230, 180, 40)
            } else {
                egui::Color32::from_rgb(220, 60, 50)
            };
            ui.painter()
                .rect_filled(fill_rect, egui::CornerRadius::same(3), color);
            // Testo vita sopra la barra.
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{:.0}/{:.0}", health.current, health.max),
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        });

    // --- Crosshair al centro ---
    let center = egui::pos2(size.x / 2.0, size.y / 2.0);
    egui::Area::new(egui::Id::new("crosshair"))
        .fixed_pos(egui::pos2(center.x - 6.0, center.y - 6.0))
        .show(ctx, |ui| {
            let (rect, _resp) =
                ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            let p = rect.center();
            let c = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200);
            let r = 5.0;
            ui.painter().line_segment(
                [egui::pos2(p.x - r, p.y), egui::pos2(p.x - 2.0, p.y)],
                egui::Stroke::new(1.5, c),
            );
            ui.painter().line_segment(
                [egui::pos2(p.x + 2.0, p.y), egui::pos2(p.x + r, p.y)],
                egui::Stroke::new(1.5, c),
            );
            ui.painter().line_segment(
                [egui::pos2(p.x, p.y - r), egui::pos2(p.x, p.y - 2.0)],
                egui::Stroke::new(1.5, c),
            );
            ui.painter().line_segment(
                [egui::pos2(p.x, p.y + 2.0), egui::pos2(p.x, p.y + r)],
                egui::Stroke::new(1.5, c),
            );
        });

    // --- Inventario in basso al centro: slot quadrati con icona colore + qty ---
    // Filtra gli slot con quantita' zero (utile se in futuro si consumano item).
    let visible: Vec<_> = inventory
        .slots
        .iter()
        .filter(|s| inventory.qty(s.kind) > 0)
        .collect();
    if !visible.is_empty() {
        let slot = 44.0_f32;
        let gap = 6.0;
        let total_w = visible.len() as f32 * (slot + gap) - gap;
        let start_x = size.x / 2.0 - total_w / 2.0;
        let start_y = size.y - slot - 16.0;

        egui::Area::new(egui::Id::new("inventory_bar"))
            .fixed_pos(egui::pos2(start_x, start_y))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for stack in &visible {
                        let (rect, _resp) =
                            ui.allocate_exact_size(egui::vec2(slot, slot), egui::Sense::hover());
                        // Sfondo slot.
                        ui.painter().rect_filled(
                            rect,
                            egui::CornerRadius::same(4),
                            egui::Color32::from_rgba_unmultiplied(25, 28, 38, 230),
                        );
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(4),
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 100, 130)),
                            egui::StrokeKind::Inside,
                        );
                        // Icona: quadrato colorato al centro.
                        let icon = 22.0;
                        let icon_rect = egui::Rect::from_center_size(
                            rect.center() + egui::vec2(0.0, -5.0),
                            egui::vec2(icon, icon),
                        );
                        let c = stack.kind.color().to_srgba();
                        ui.painter().rect_filled(
                            icon_rect,
                            egui::CornerRadius::same(3),
                            egui::Color32::from_rgb(
                                (c.red * 255.0) as u8,
                                (c.green * 255.0) as u8,
                                (c.blue * 255.0) as u8,
                            ),
                        );
                        // Tooltip con nome al passaggio del mouse.
                        _resp.on_hover_ui(|ui| {
                            ui.label(stack.kind.name());
                        });
                        // Quantità in basso a destra dello slot.
                        ui.painter().text(
                            egui::pos2(rect.max.x - 4.0, rect.max.y - 4.0),
                            egui::Align2::RIGHT_BOTTOM,
                            format!("{}", stack.qty),
                            egui::FontId::proportional(13.0),
                            egui::Color32::WHITE,
                        );
                        ui.add_space(gap);
                    }
                });
            });
    }
}
