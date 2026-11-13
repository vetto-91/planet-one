//! HUD di gioco (Fase 4.1): barra vita, crosshair, inventario in basso.
//!
//! Tutto disegnato con egui Area dentro `EguiPrimaryContextPass`, così non
//! servono una camera 2D né Text2d (coordinate world instabili con la camera orbit).

use crate::combat::EnemyHealth;
use crate::entities::Enemy;
use crate::inventory::Inventory;
use crate::player::PlayerHealth;
use bevy::prelude::*;
use bevy_egui::egui;

/// Colori delle tre riserve: (pieno, scuro di fondo).
const HEALTH_COLOR: egui::Color32 = egui::Color32::from_rgb(196, 58, 52);
const VIGOR_COLOR: egui::Color32 = egui::Color32::from_rgb(98, 172, 84);
const MAGIC_COLOR: egui::Color32 = egui::Color32::from_rgb(72, 118, 214);
/// Altezza delle barre e distanza dalla barra degli oggetti.
const BAR_H: f32 = 20.0;
const BAR_GAP: f32 = 8.0;

/// Una barra di riserva nello stile dei pannelli: fondo scuro, bordo d'oro, riempimento con un
/// filo di luce in alto, nome a sinistra e valori a destra. `dim` la spegne (senza fiato).
fn reserve_bar(
    painter: &egui::Painter,
    rect: egui::Rect,
    name: &str,
    current: f32,
    max: f32,
    color: egui::Color32,
    dim: bool,
) {
    let radius = egui::CornerRadius::same(5);
    painter.rect_filled(rect, radius, crate::ui_style::PANEL_FILL);
    let inner = rect.shrink(3.0);
    painter.rect_filled(
        inner,
        egui::CornerRadius::same(3),
        color.linear_multiply(0.18),
    );
    let frac = if max > 0.0 {
        (current / max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if frac > 0.0 {
        let fill = egui::Rect::from_min_size(
            inner.min,
            egui::vec2((inner.width() * frac).max(2.0), inner.height()),
        );
        let body = if dim {
            color.linear_multiply(0.45)
        } else {
            color
        };
        painter.rect_filled(fill, egui::CornerRadius::same(3), body);
        // Filo di luce in alto: dà spessore senza sfumature.
        let shine = egui::Rect::from_min_size(fill.min, egui::vec2(fill.width(), 3.0));
        painter.rect_filled(
            shine,
            egui::CornerRadius::same(3),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 46),
        );
    }
    painter.rect_stroke(
        rect,
        radius,
        egui::Stroke::new(1.0, crate::ui_style::GOLD_EDGE),
        egui::StrokeKind::Inside,
    );
    let font = egui::FontId::proportional(12.5);
    let shadow = egui::Color32::from_black_alpha(200);
    let text = |pos: egui::Pos2, align: egui::Align2, s: String| {
        painter.text(pos + egui::vec2(1.0, 1.0), align, &s, font.clone(), shadow);
        painter.text(pos, align, s, font.clone(), crate::ui_style::TEXT);
    };
    text(
        egui::pos2(rect.min.x + 9.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        name.to_string(),
    );
    text(
        egui::pos2(rect.max.x - 9.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        format!("{current:.0}/{max:.0}"),
    );
}

/// Disegna l'HUD in `Playing`: crosshair al centro, barra degli oggetti in basso e, subito sopra,
/// le tre riserve: magia a sinistra, vita al centro, vigore a destra (ADR 0039).
pub fn draw_hud(
    ctx: &egui::Context,
    health: &PlayerHealth,
    vitals: &crate::vitals::Vitals,
    spell: Option<(&str, f32)>,
    inventory: &Inventory,
    equip: &crate::inventory::Equipment,
    icons: Option<&crate::inventory_ui::ItemIcons>,
) {
    let size = ctx.viewport_rect().size();

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

    // --- Barra in basso al centro: sempre 10 riquadri (tasti 1..9 e 0), le monete hanno il loro contatore ---
    let slot = 44.0_f32;
    let gap = 6.0;
    let total_w = 10.0 * (slot + gap) - gap;
    let left = size.x / 2.0 - total_w / 2.0;
    let bars_y = size.y - slot - 16.0 - BAR_GAP - BAR_H;
    // La vita è più larga delle altre due; insieme sono larghe quanto la barra degli oggetti.
    let side_w = (total_w - 2.0 * gap) * 0.3;
    let mid_w = total_w - 2.0 * (side_w + gap);
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("reserve_bars"),
    ));
    let bar =
        |x: f32, w: f32| egui::Rect::from_min_size(egui::pos2(x, bars_y), egui::vec2(w, BAR_H));
    reserve_bar(
        &painter,
        bar(left, side_w),
        "Magia",
        vitals.magic,
        vitals.magic_max,
        MAGIC_COLOR,
        false,
    );
    // A sinistra della magia: tasto, incantesimo scelto e quanto costa (grigio se non basta).
    if let Some((name, price)) = spell {
        let font = egui::FontId::proportional(13.0);
        let at = egui::pos2(left - 10.0, bars_y + BAR_H / 2.0);
        let shadow = egui::Color32::from_black_alpha(200);
        let text = format!("{name} · {price:.0}");
        painter.text(
            at + egui::vec2(1.0, 1.0),
            egui::Align2::RIGHT_CENTER,
            &text,
            font.clone(),
            shadow,
        );
        let body = painter.text(
            at,
            egui::Align2::RIGHT_CENTER,
            &text,
            font.clone(),
            if vitals.magic >= price {
                crate::ui_style::TEXT
            } else {
                egui::Color32::from_rgb(150, 156, 172)
            },
        );
        painter.text(
            egui::pos2(body.min.x - 4.0, at.y),
            egui::Align2::RIGHT_CENTER,
            "[Q]",
            font,
            crate::ui_style::GOLD,
        );
    }
    reserve_bar(
        &painter,
        bar(left + side_w + gap, mid_w),
        "Vita",
        health.current,
        health.max,
        HEALTH_COLOR,
        false,
    );
    reserve_bar(
        &painter,
        bar(left + side_w + mid_w + 2.0 * gap, side_w),
        "Vigore",
        vitals.vigor,
        vitals.vigor_max,
        VIGOR_COLOR,
        vitals.winded,
    );
    egui::Area::new(egui::Id::new("inventory_bar"))
        .fixed_pos(egui::pos2(
            size.x / 2.0 - total_w / 2.0,
            size.y - slot - 16.0,
        ))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Solo lo spazio voluto fra i riquadri: la barra resta centrata e larga `total_w`.
                ui.spacing_mut().item_spacing.x = 0.0;
                for i in 0..10 {
                    let stack = inventory.hotbar_slot(i);
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(slot, slot), egui::Sense::hover());
                    ui.painter().rect_filled(
                        rect,
                        egui::CornerRadius::same(4),
                        egui::Color32::from_rgba_unmultiplied(25, 28, 38, 230),
                    );
                    let equipped = stack.is_some_and(|s| equip.is_worn(s.kind));
                    ui.painter().rect_stroke(
                        rect,
                        egui::CornerRadius::same(4),
                        if equipped {
                            egui::Stroke::new(2.5, egui::Color32::from_rgb(255, 214, 102))
                        } else {
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 100, 130))
                        },
                        egui::StrokeKind::Inside,
                    );
                    if let Some(stack) = stack {
                        let icon_rect =
                            egui::Rect::from_center_size(rect.center(), egui::vec2(36.0, 36.0));
                        crate::inventory_ui::paint_icon(ui, icons, stack.kind, icon_rect);
                        resp.on_hover_ui(|ui| {
                            ui.label(stack.kind.name());
                        });
                        if stack.qty > 1 {
                            ui.painter().text(
                                egui::pos2(rect.max.x - 3.0, rect.max.y - 2.0),
                                egui::Align2::RIGHT_BOTTOM,
                                format!("{}", stack.qty),
                                egui::FontId::proportional(13.0),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                    // Tasto del riquadro: 1..9, poi 0.
                    ui.painter().text(
                        egui::pos2(rect.min.x + 4.0, rect.min.y + 3.0),
                        egui::Align2::LEFT_TOP,
                        if i == 9 {
                            "0".to_string()
                        } else {
                            (i + 1).to_string()
                        },
                        egui::FontId::proportional(12.0),
                        egui::Color32::from_rgb(200, 210, 235),
                    );
                    ui.add_space(gap);
                }
            });
        });
}

/// Distanza (m) dal player entro cui compaiono le targhette dei personaggi.
const LABEL_DIST: f32 = 10.0;

/// Entità (NPC, nemici) nascoste alla camera da un muro: niente targhetta né barra vita.
#[derive(Resource, Default)]
pub struct Occluded(pub std::collections::HashSet<Entity>);

/// Sistema: calcola quali NPC/nemici sono dietro un muro (raycast camera → corpo,
/// su tre altezze: occluso solo se tutte e tre sono bloccate).
type Occludable = Or<(With<crate::entities::Npc>, With<crate::entities::Enemy>)>;

pub fn update_occlusion(
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    targets: Query<(Entity, &GlobalTransform), Occludable>,
    spatial: avian3d::prelude::SpatialQuery,
    blockers: Query<(), With<crate::camera::BlocksCamera>>,
    mut occluded: ResMut<Occluded>,
) {
    occluded.0.clear();
    let Ok(cam) = cameras.single() else {
        return;
    };
    let origin = cam.translation();
    for (e, t) in &targets {
        let hidden = [0.3, 0.9, 1.4].into_iter().all(|h| {
            let to = t.translation() + Vec3::Y * h - origin;
            let dist = to.length();
            Dir3::new(to).is_ok_and(|d| {
                spatial
                    .cast_ray_predicate(
                        origin,
                        d,
                        dist,
                        true,
                        &avian3d::prelude::SpatialQueryFilter::default(),
                        &|ent| blockers.contains(ent),
                    )
                    .is_some_and(|hit| hit.distance < dist - 0.15)
            })
        });
        if hidden {
            occluded.0.insert(e);
        }
    }
}

/// Altezza (m, sopra il centro) del punto di ancoraggio della targhetta del
/// nome; il fumetto di chi parla si ancora allo stesso punto, sotto la targhetta.
#[derive(Component)]
pub struct NameAnchor(pub f32);

/// Disegna sopra ogni nemico: nome "enemy" + barra vita rossa.
/// Usa la proiezione camera→viewport per mettere la barra sopra la testa.
pub fn draw_enemy_bars(
    ctx: &egui::Context,
    enemies: &Query<
        (
            Entity,
            &GlobalTransform,
            &EnemyHealth,
            Option<&crate::quests::denaro_rubato::QuestThief>,
        ),
        With<Enemy>,
    >,
    cameras: &Query<(&Camera, &GlobalTransform)>,
    player_pos: Vec3,
    barks: &crate::quests::Barks,
    occluded: &Occluded,
) {
    let Ok((camera, cam_t)) = cameras.single() else {
        return;
    };
    let viewport = ctx.viewport_rect();

    let painter = ctx.layer_painter(egui::LayerId::background());
    for (entity, t, hp, thief) in enemies.iter() {
        if occluded.0.contains(&entity) {
            continue;
        }
        // Solo in prossimità del player (o se sta parlando).
        if t.translation().distance(player_pos) > LABEL_DIST && !barks.has(entity) {
            continue;
        }
        // Punto sopra la testa del nemico: il cubo e' alto 1.2 m, quindi
        // il top e' a +0.6; aggiungiamo margine per nome+barra (1.1 m).
        let world_pos = t.translation() + Vec3::Y * 1.1;
        let Ok(screen) = camera.world_to_viewport(cam_t, world_pos) else {
            continue; // fuori dallo schermo o dietro la camera
        };
        // Pixel dello schermo → punti di egui (scala dell'interfaccia).
        let screen = crate::ui_style::screen_to_ui(ctx, screen);
        // Bevy 0.20: world_to_viewport restituisce gia' coordinate viewport
        // in pixel con origine in ALTO a sinistra (y-down), pronte per egui.
        // Non invertire y: quella conversione sposta la barra sotto il nemico.
        let x = screen.x;
        let y = screen.y;
        if x < 0.0 || x > viewport.width() || y < 0.0 || y > viewport.height() {
            continue;
        }

        let bar_w = 70.0_f32;
        let bar_h = 6.0_f32;
        let name_h = 0.0_f32;
        let center = egui::pos2(x, y - name_h - bar_h - 4.0);

        // Targhetta con il nome sopra la barra vita.
        let name_color = if thief.is_some() {
            egui::Color32::from_rgb(255, 150, 140)
        } else {
            crate::ui_style::TEXT
        };
        crate::ui_style::plate(
            &painter,
            egui::pos2(center.x, center.y + name_h),
            if thief.is_some() { "Ladro" } else { "Nemico" },
            20.0,
            name_color,
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

/// Targhetta con il nome sopra ogni NPC vicino al player.
pub fn draw_npc_labels(
    ctx: &egui::Context,
    npcs: &Query<(Entity, &GlobalTransform, &crate::entities::Npc, &Visibility)>,
    cameras: &Query<(&Camera, &GlobalTransform)>,
    player_pos: Vec3,
    barks: &crate::quests::Barks,
    occluded: &Occluded,
) {
    let Ok((camera, cam_t)) = cameras.single() else {
        return;
    };
    let viewport = ctx.viewport_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    // Raccolte prima, dalla più vicina alla camera: se si coprono, quelle dietro salgono.
    let mut labels: Vec<(f32, egui::Pos2, &str)> = Vec::new();
    for (entity, t, npc, vis) in npcs.iter() {
        // Di notte gli abitanti sono in casa (nascosti): niente targhetta.
        if *vis == Visibility::Hidden || occluded.0.contains(&entity) {
            continue;
        }
        if t.translation().distance(player_pos) > LABEL_DIST && !barks.has(entity) {
            continue;
        }
        let Ok(screen) = camera.world_to_viewport(cam_t, t.translation() + Vec3::Y * 1.3) else {
            continue;
        };
        let screen = crate::ui_style::screen_to_ui(ctx, screen);
        if !viewport.contains(egui::pos2(screen.x, screen.y)) {
            continue;
        }
        let depth = t.translation().distance(cam_t.translation());
        labels.push((depth, egui::pos2(screen.x, screen.y), npc.name));
    }
    labels.sort_by(|a, b| a.0.total_cmp(&b.0));
    let rects: Vec<egui::Rect> = labels
        .iter()
        .map(|(_, p, name)| crate::ui_style::plate_rect(&painter, *p, name, 20.0))
        .collect();
    let offsets = crate::ui_style::stack_offsets(&rects);
    for ((_, p, name), dy) in labels.iter().zip(offsets) {
        crate::ui_style::plate(
            &painter,
            egui::pos2(p.x, p.y - dy),
            name,
            20.0,
            crate::ui_style::GOLD,
        );
    }
}
