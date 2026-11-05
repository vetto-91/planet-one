//! HUD di gioco (Fase 4.1): barra vita, crosshair, inventario in basso.
//!
//! Tutto disegnato con egui Area dentro `EguiPrimaryContextPass`, così non
//! servono caméra 2D né노 Text2d (coordinate world instabili con la camera orbit).

use crate::combat::EnemyHealth;
use crate::entities::Enemy;
use crate::inventory::Inventory;
use crate::player::PlayerHealth;
use bevy::prelude::*;
use bevy_egui::egui;

/// Disegna l'HUD in `Playing`: barra vita in alto a sinistra, crosshair al
/// centro, slot inventario in basso al centro.
pub fn draw_hud(ctx: &egui::Context, health: &PlayerHealth, inventory: &Inventory) {
    let size = ctx.viewport_rect().size();

    // --- Barra vita del player: in basso a sinistra, meta' lunghezza ---
    let bar_w = 110.0;
    let bar_h = 18.0;
    let pos = egui::pos2(16.0, size.y - 34.0);
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
    // Le monete NON compaiono qui (solo nel contatore in alto a sinistra).
    // Gli slot si creano in ordine di raccolta (vedi Inventory::add), max 10 (tasti 1..0).
    let visible = inventory.hotbar();
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
                    for (i, stack) in visible.iter().enumerate() {
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
                        // Tasto associato allo slot: 1..9, poi 0 (10 slot in tutto).
                        let key = if i == 9 {
                            "0".to_string()
                        } else {
                            (i + 1).to_string()
                        };
                        ui.painter().text(
                            egui::pos2(rect.min.x + 4.0, rect.min.y + 3.0),
                            egui::Align2::LEFT_TOP,
                            key,
                            egui::FontId::proportional(12.0),
                            egui::Color32::from_rgb(200, 210, 235),
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

/// Disegna sopra ogni nemico: nome "enemy" + barra vita rossa.
/// Usa la proiezione camera→viewport per mettere la barra sopra la testa.
pub fn draw_enemy_bars(
    ctx: &egui::Context,
    enemies: &Query<(&GlobalTransform, &EnemyHealth), With<Enemy>>,
    cameras: &Query<(&Camera, &GlobalTransform)>,
) {
    let Ok((camera, cam_t)) = cameras.single() else {
        return;
    };
    let viewport = ctx.viewport_rect();

    let painter = ctx.layer_painter(egui::LayerId::background());
    for (t, hp) in enemies.iter() {
        // Punto sopra la testa del nemico: il cubo e' alto 1.2 m, quindi
        // il top e' a +0.6; aggiungiamo margine per nome+barra (1.1 m).
        let world_pos = t.translation() + Vec3::Y * 1.1;
        let Ok(screen) = camera.world_to_viewport(cam_t, world_pos) else {
            continue; // fuori dallo schermo o dietro la camera
        };
        // Bevy 0.20: world_to_viewport restituisce gia' coordinate viewport
        // in pixel con origine in ALTO a sinistra (y-down), pronte per egui.
        // Non invertire y: quella conversione sposta la barra sotto il nemico.
        let x = screen.x;
        let y = screen.y;
        if x < 0.0 || x > viewport.width() || y < 0.0 || y > viewport.height() {
            continue;
        }

        let bar_w = 60.0_f32;
        let bar_h = 6.0_f32;
        let name_h = 16.0_f32;
        let center = egui::pos2(x, y - name_h - bar_h - 4.0);

        // Nome "enemy" sopra la barra, centrato.
        painter.text(
            egui::pos2(center.x, center.y + name_h / 2.0),
            egui::Align2::CENTER_CENTER,
            "enemy",
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(255, 220, 220),
        );

        // Barra vita: sfondo scuro + riempimento rosso proporzionale.
        let bar_rect = egui::Rect::from_center_size(
            egui::pos2(center.x, center.y + name_h + bar_h / 2.0 + 2.0),
            egui::vec2(bar_w, bar_h),
        );
        painter.rect_filled(
            bar_rect,
            egui::CornerRadius::same(2),
            egui::Color32::from_rgba_unmultiplied(30, 10, 10, 220),
        );
        let frac = (hp.current / hp.max).clamp(0.0, 1.0);
        let fill_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(bar_w * frac, bar_h));
        painter.rect_filled(
            fill_rect,
            egui::CornerRadius::same(2),
            egui::Color32::from_rgb(220, 50, 40),
        );
    }
}
