//! Mappa in gioco (tasto M): vista dall'alto "da drone" della scena reale.
//!
//! Invece di disegnare una mappa a parte si rende il mondo vero con una camera
//! ortografica rivolta verso il basso (nord in alto: N = -Z, E = +X, come la
//! bussola): terreno, casetta e tutto ciò che verrà aggiunto compaiono da soli.
//! Sopra la scena egui disegna la bussola (in alto a sinistra, in una zona
//! vuota), il segnalino del giocatore e le etichette.
//!
//! Lo zoom è predisposto in `MapView` (`zoom_in`/`zoom_out`, rotella e +/-).
//! La vista si sposta con frecce/WASD o trascinando (tasto centrale; il sinistro
//! quando il debug F4 è spento); C la ricentra sul giocatore.

use bevy::camera::{ClearColorConfig, ScalingMode};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::light::Skybox;
use bevy::prelude::*;
use bevy_egui::egui;

/// Lato (m) del terreno (isola + mare attorno).
const TERRAIN_SIZE: f32 = crate::terrain::TERRAIN_SIZE;
/// Altezza della camera sopra il terreno.
const MAP_CAM_HEIGHT: f32 = 300.0;
/// Fondo della mappa (carta scura) al posto del cielo.
const MAP_BACKGROUND: Color = Color::srgb(0.10, 0.09, 0.075);
/// Luminosità minima della luce ambiente in mappa (leggibile anche di notte).
const MAP_AMBIENT: f32 = 450.0;

/// Stato dello zoom della mappa: `zoom` = altezza (m) di mondo mostrata.
#[derive(Resource)]
pub struct MapView {
    pub zoom: f32,
    pub min: f32,
    pub max: f32,
    /// Centro della vista (x, z) in metri.
    pub center: Vec2,
}

impl Default for MapView {
    fn default() -> Self {
        Self {
            zoom: 180.0,
            min: 20.0,
            max: TERRAIN_SIZE * 1.3,
            center: Vec2::ZERO,
        }
    }
}

impl MapView {
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom / 1.2).max(self.min);
    }
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom * 1.2).min(self.max);
    }
}

/// Camera salvata all'ingresso in mappa, ripristinata all'uscita.
#[derive(Resource, Default)]
pub struct SavedCamera(Option<Saved>);

pub struct Saved {
    transform: Transform,
    projection: Projection,
    clear: ClearColorConfig,
    skybox: Option<Skybox>,
}

/// OnEnter(Map): porta la camera sopra la scena, in ortografica, nord in alto.
pub fn enter_map(
    mut commands: Commands,
    mut view: ResMut<MapView>,
    mut saved: ResMut<SavedCamera>,
    mut cameras: Query<(
        Entity,
        &mut Transform,
        &mut Projection,
        &mut Camera,
        Option<&Skybox>,
    )>,
) {
    let Ok((entity, mut tf, mut proj, mut cam, skybox)) = cameras.single_mut() else {
        return;
    };
    saved.0 = Some(Saved {
        transform: *tf,
        projection: proj.clone(),
        clear: cam.clear_color,
        skybox: skybox.cloned(),
    });
    view.center = Vec2::ZERO;
    *tf = Transform::from_xyz(0.0, MAP_CAM_HEIGHT, 0.0).looking_at(Vec3::ZERO, Vec3::NEG_Z);
    *proj = Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::FixedVertical {
            viewport_height: view.zoom,
        },
        far: 2000.0,
        ..OrthographicProjection::default_3d()
    });
    cam.clear_color = ClearColorConfig::Custom(MAP_BACKGROUND);
    commands.entity(entity).remove::<Skybox>();
    info!("[MAP] mappa aperta");
}

/// OnExit(Map): ripristina camera, proiezione, sfondo e cielo.
pub fn exit_map(
    mut commands: Commands,
    mut saved: ResMut<SavedCamera>,
    mut cameras: Query<(Entity, &mut Transform, &mut Projection, &mut Camera)>,
) {
    let (Ok((entity, mut tf, mut proj, mut cam)), Some(s)) = (cameras.single_mut(), saved.0.take())
    else {
        return;
    };
    *tf = s.transform;
    *proj = s.projection;
    cam.clear_color = s.clear;
    if let Some(sky) = s.skybox {
        commands.entity(entity).insert(sky);
    }
    info!("[MAP] mappa chiusa");
}

/// Zoom (rotella, +/-) e spostamento (frecce/WASD, trascinamento, C = sul giocatore).
#[allow(clippy::too_many_arguments)]
pub fn map_zoom(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut motion: MessageReader<MouseMotion>,
    mut view: ResMut<MapView>,
    debug: Res<crate::debug_place::DebugPlace>,
    mut contexts: bevy_egui::EguiContexts,
    windows: Query<&Window>,
    mut cameras: Query<(&mut Projection, &mut Transform), With<Camera3d>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
) {
    let mut steps = 0.0;
    for ev in wheel.read() {
        steps += ev.y.signum();
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        steps += 1.0;
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        steps -= 1.0;
    }
    if steps > 0.0 {
        view.zoom_in();
    } else if steps < 0.0 {
        view.zoom_out();
    }

    let (egui_kb, egui_ptr) = contexts.ctx_mut().map_or((false, false), |c| {
        (c.egui_wants_keyboard_input(), c.is_pointer_over_egui())
    });
    // Spostamento da tastiera (la velocità cresce con lo zoom).
    let mut dir = Vec2::ZERO;
    if !egui_kb {
        if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
            dir.x -= 1.0;
        }
        if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
            dir.x += 1.0;
        }
        if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
            dir.y -= 1.0; // nord = -Z
        }
        if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
            dir.y += 1.0;
        }
    }
    let zoom = view.zoom;
    if dir != Vec2::ZERO {
        view.center += dir.normalize() * zoom * 0.6 * time.delta_secs();
    }
    // Trascinamento: tasto centrale sempre, sinistro solo a debug spento.
    let dragging = buttons.pressed(MouseButton::Middle)
        || (!debug.on && buttons.pressed(MouseButton::Left) && !egui_ptr);
    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }
    if dragging && let Ok(w) = windows.single() {
        let per_px = zoom / w.height().max(1.0);
        view.center -= Vec2::new(delta.x, delta.y) * per_px;
    }
    if keys.just_pressed(KeyCode::KeyC)
        && !egui_kb
        && let Ok(p) = players.single()
    {
        view.center = Vec2::new(p.translation().x, p.translation().z);
    }
    let limit = TERRAIN_SIZE / 2.0;
    view.center = view.center.clamp(Vec2::splat(-limit), Vec2::splat(limit));

    if let Ok((mut proj, mut tf)) = cameras.single_mut() {
        if let Projection::Orthographic(o) = &mut *proj {
            o.scaling_mode = ScalingMode::FixedVertical {
                viewport_height: view.zoom,
            };
        }
        let c = view.center;
        *tf = Transform::from_xyz(c.x, MAP_CAM_HEIGHT, c.y)
            .looking_at(Vec3::new(c.x, 0.0, c.y), Vec3::NEG_Z);
    }
}

/// In mappa la luce ambiente non scende sotto `MAP_AMBIENT` (dopo `update_sky`).
pub fn map_lighting(mut ambient: ResMut<GlobalAmbientLight>) {
    if ambient.brightness < MAP_AMBIENT {
        ambient.brightness = MAP_AMBIENT;
        ambient.color = Color::srgb(0.95, 0.95, 1.0);
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<MapView>()
        .init_resource::<SavedCamera>();
}

/// Overlay egui della mappa: titolo, bussola in alto a sinistra, segnalino del
/// giocatore, etichetta della casetta e legenda.
pub fn draw_map_overlay(
    ctx: &egui::Context,
    camera: Option<(&Camera, &GlobalTransform)>,
    player_pos: Vec3,
    heading_deg: f32,
    hut_pos: Option<Vec3>,
    village: Option<(&str, Vec3)>,
    zoom: f32,
) {
    let size = ctx.viewport_rect().size();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("map_overlay"),
    ));
    let gold = crate::ui_style::GOLD;
    // Titolo.
    crate::ui_style::plate(
        &painter,
        egui::pos2(size.x / 2.0, 52.0),
        "Mappa",
        26.0,
        gold,
    );

    // Bussola (rosa dei venti) in alto a sinistra: N = su, E = destra.
    let c = egui::pos2(86.0, 100.0);
    let r = 46.0;
    painter.circle_filled(c, r + 14.0, crate::ui_style::PANEL_FILL);
    painter.circle_stroke(
        c,
        r + 14.0,
        egui::Stroke::new(1.5, crate::ui_style::GOLD_EDGE),
    );
    painter.circle_stroke(
        c,
        r,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(110, 118, 140)),
    );
    // Tacche ogni 45°.
    for i in 0..8 {
        let a = (i as f32) * std::f32::consts::FRAC_PI_4;
        let (s, co) = a.sin_cos();
        let len = if i % 2 == 0 { 10.0 } else { 6.0 };
        painter.line_segment(
            [
                c + egui::vec2(s, -co) * (r - len),
                c + egui::vec2(s, -co) * r,
            ],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(170, 180, 205)),
        );
    }
    // Aghi: nord (oro) e sud (grigio).
    painter.add(egui::Shape::convex_polygon(
        vec![
            c + egui::vec2(0.0, -r + 6.0),
            c + egui::vec2(-7.0, 0.0),
            c + egui::vec2(7.0, 0.0),
        ],
        gold,
        egui::Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![
            c + egui::vec2(0.0, r - 6.0),
            c + egui::vec2(-7.0, 0.0),
            c + egui::vec2(7.0, 0.0),
        ],
        egui::Color32::from_rgb(120, 128, 150),
        egui::Stroke::NONE,
    ));
    for (label, dx, dy, col) in [
        ("N", 0.0, -1.0, gold),
        ("E", 1.0, 0.0, crate::ui_style::TEXT),
        ("S", 0.0, 1.0, crate::ui_style::TEXT),
        ("W", -1.0, 0.0, crate::ui_style::TEXT),
    ] {
        painter.text(
            c + egui::vec2(dx, dy) * (r + 28.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(20.0),
            col,
        );
    }

    // Segnalino del giocatore (freccia nella direzione dello sguardo) e casetta.
    if let Some((cam, cam_t)) = camera {
        if let Ok(p) = cam.world_to_viewport(cam_t, player_pos) {
            let pos = egui::pos2(p.x, p.y);
            let b = heading_deg.to_radians();
            let dir = egui::vec2(b.sin(), -b.cos());
            let perp = egui::vec2(-dir.y, dir.x);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos + dir * 13.0,
                    pos - dir * 8.0 + perp * 8.0,
                    pos - dir * 4.0,
                    pos - dir * 8.0 - perp * 8.0,
                ],
                egui::Color32::from_rgb(255, 90, 70),
                egui::Stroke::new(1.5, egui::Color32::WHITE),
            ));
            painter.text(
                pos + egui::vec2(0.0, 22.0),
                egui::Align2::CENTER_CENTER,
                "Tu",
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            );
        }
        if let Some((name, at)) = village
            && let Ok(p) = cam.world_to_viewport(cam_t, at + Vec3::Y * 6.0)
        {
            crate::ui_style::plate(&painter, egui::pos2(p.x, p.y), name, 28.0, gold);
        }
        if let Some(h) = hut_pos
            && let Ok(p) = cam.world_to_viewport(cam_t, h + Vec3::Y * 6.0)
        {
            crate::ui_style::plate(
                &painter,
                egui::pos2(p.x, p.y - 26.0),
                "Capanna del Viandante",
                18.0,
                gold,
            );
        }
    }

    // Legenda comandi.
    painter.text(
        egui::pos2(size.x / 2.0, size.y - 22.0),
        egui::Align2::CENTER_CENTER,
        format!(
            "M / Esc = chiudi   •   Rotella o +/- = zoom ({zoom:.0} m)   •   Frecce/WASD o trascina = sposta   •   C = su di te   •   F4 = debug piazzamenti"
        ),
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(220, 226, 240),
    );
}
