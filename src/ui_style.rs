//! Stile UI condiviso: palette dei pannelli di dialogo (fondo scuro, bordo
//! oro, testo chiaro), targhette nel mondo e tema egui globale.

use bevy_egui::egui::{self, Color32};

/// Fondo dei pannelli.
pub const PANEL_FILL: Color32 = Color32::from_rgba_premultiplied(10, 12, 20, 245);
/// Bordo e nomi in oro (come il pannello del dialogo).
pub const GOLD: Color32 = Color32::from_rgb(255, 214, 102);
/// Bordo oro leggermente più scuro.
pub const GOLD_EDGE: Color32 = Color32::from_rgb(230, 190, 90);
/// Testo principale.
pub const TEXT: Color32 = Color32::from_rgb(245, 248, 255);

/// Targhetta: testo con riquadro scuro e bordo oro, con il bordo inferiore
/// centrato su `center_bottom`.
pub fn plate(
    painter: &egui::Painter,
    center_bottom: egui::Pos2,
    text: &str,
    size: f32,
    color: Color32,
) {
    let galley = painter.layout_no_wrap(text.to_string(), egui::FontId::proportional(size), color);
    let pad = egui::vec2(12.0, 5.0);
    let rect = egui::Rect::from_min_size(
        egui::pos2(
            center_bottom.x - galley.size().x / 2.0 - pad.x,
            center_bottom.y - galley.size().y - pad.y * 2.0,
        ),
        galley.size() + pad * 2.0,
    );
    painter.rect_filled(rect, egui::CornerRadius::same(7), PANEL_FILL);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(7),
        egui::Stroke::new(1.5, GOLD_EDGE),
        egui::StrokeKind::Inside,
    );
    painter.galley(rect.min + pad, galley, color);
}

/// Tema egui globale: gli stessi colori e dimensioni dell'HUD in gioco.
pub fn apply_style(ctx: &egui::Context) {
    ctx.global_style_mut(|style| {
        use egui::{FontId, TextStyle};
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(30.0));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(19.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(19.0));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(15.0));
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace(17.0));
        style.spacing.button_padding = egui::vec2(16.0, 7.0);
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);

        let v = &mut style.visuals;
        v.override_text_color = Some(TEXT);
        v.window_fill = PANEL_FILL;
        v.panel_fill = PANEL_FILL;
        v.window_stroke = egui::Stroke::new(1.5, GOLD_EDGE);
        v.window_corner_radius = egui::CornerRadius::same(10);
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(70, 78, 100));
        v.widgets.noninteractive.bg_fill = Color32::from_rgb(22, 28, 42);
        v.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(22, 28, 42);
        v.widgets.inactive.bg_fill = Color32::from_rgb(32, 38, 54);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(32, 38, 54);
        v.widgets.hovered.bg_fill = Color32::from_rgb(70, 58, 28);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(70, 58, 28);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, GOLD);
        v.widgets.active.bg_fill = Color32::from_rgb(110, 88, 34);
        v.widgets.active.weak_bg_fill = Color32::from_rgb(110, 88, 34);
        v.selection.bg_fill = Color32::from_rgb(110, 88, 34);
        v.selection.stroke = egui::Stroke::new(1.0, GOLD);
    });
}
