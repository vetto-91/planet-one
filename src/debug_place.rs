//! Modalità debug per indicare dove piazzare oggetti (alberi, rocce, …).
//!
//! F4 attiva/disattiva il pannello. Si piazza **dalla mappa** (M: clic = aggiungi,
//! clic destro = elimina il punto più vicino) oppure **in 3D** col mirino (P =
//! aggiungi, X = elimina il più vicino). Modalità: singolo, gruppo attorno a un
//! punto, area rettangolare, area cerchio (per le aree: due punti). Ogni modifica
//! riscrive `debug/placements.txt`, leggibile a mano (anche da chi continuerà il
//! lavoro) e riletto all'avvio. Sono solo segnaposto: nessun asset reale.

use crate::AppState;
use crate::daynight::GameClock;
use crate::props::SnapToGround;
use crate::scene::Ground;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use std::fmt::Write as _;

/// File con i piazzamenti (relativo alla cartella del progetto / di lavoro).
const FILE: &str = "debug/placements.txt";
/// Tipi predefiniti.
const KINDS: [&str; 9] = [
    "albero",
    "roccia",
    "cespuglio",
    "fiori",
    "casa",
    "muro",
    "lampione",
    "panchina",
    "altro",
];
/// Limite del terreno (±m) per i punti generati.
const TERRAIN_HALF: f32 = 49.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlaceMode {
    Single,
    Group,
    Rect,
    Circle,
}

impl PlaceMode {
    fn label(self) -> &'static str {
        match self {
            PlaceMode::Single => "singolo",
            PlaceMode::Group => "gruppo-raggio",
            PlaceMode::Rect => "area-rettangolo",
            PlaceMode::Circle => "area-cerchio",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "singolo" => PlaceMode::Single,
            "gruppo-raggio" => PlaceMode::Group,
            "area-rettangolo" => PlaceMode::Rect,
            "area-cerchio" => PlaceMode::Circle,
            _ => return None,
        })
    }
}

/// Una richiesta di piazzamento (uno o più oggetti).
#[derive(Clone)]
pub struct Placement {
    pub id: u32,
    pub kind: String,
    pub mode: PlaceMode,
    /// Punto di riferimento: centro (gruppo/cerchio) o primo angolo (rettangolo).
    pub a: Vec2,
    /// Secondo angolo (rettangolo).
    pub b: Vec2,
    pub radius: f32,
    pub seed: u64,
    pub note: String,
    pub when: String,
    /// Coordinate effettive (x, z) di ogni oggetto: è la fonte di verità.
    pub points: Vec<Vec2>,
}

#[derive(Resource)]
pub struct DebugPlace {
    pub on: bool,
    pub items: Vec<Placement>,
    next_id: u32,
    // Impostazioni dello strumento.
    kind_idx: usize,
    custom_kind: String,
    mode: PlaceMode,
    count: u32,
    radius: f32,
    note: String,
    /// Primo punto di un'area in attesa del secondo.
    pending: Option<Vec2>,
    /// I segnaposto 3D vanno ricreati.
    markers_dirty: bool,
}

impl Default for DebugPlace {
    fn default() -> Self {
        let mut d = Self {
            on: false,
            items: Vec::new(),
            next_id: 1,
            kind_idx: 0,
            custom_kind: String::new(),
            mode: PlaceMode::Single,
            count: 10,
            radius: 5.0,
            note: String::new(),
            pending: None,
            markers_dirty: true,
        };
        d.items = read_file();
        d.next_id = d.items.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        info!("[DEBUG] piazzamenti letti da {FILE}: {}", d.items.len());
        d
    }
}

impl DebugPlace {
    fn kind(&self) -> String {
        if KINDS[self.kind_idx] == "altro" && !self.custom_kind.trim().is_empty() {
            self.custom_kind.trim().to_string()
        } else {
            KINDS[self.kind_idx].to_string()
        }
    }

    /// Salva e fa ricreare i segnaposto.
    fn changed(&mut self) {
        write_file(&self.items);
        self.markers_dirty = true;
    }
}

// ---------------------------------------------------------------- generazione

fn xorshift(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 11) as f32 / (1u64 << 53) as f32
}

fn clamp_terrain(p: Vec2) -> Vec2 {
    p.clamp(Vec2::splat(-TERRAIN_HALF), Vec2::splat(TERRAIN_HALF))
}

fn gen_points(mode: PlaceMode, a: Vec2, b: Vec2, radius: f32, count: u32, seed: u64) -> Vec<Vec2> {
    let mut st = seed | 1;
    match mode {
        PlaceMode::Single => vec![clamp_terrain(a)],
        PlaceMode::Group | PlaceMode::Circle => {
            let r = if mode == PlaceMode::Circle {
                a.distance(b)
            } else {
                radius
            };
            (0..count)
                .map(|_| {
                    let ang = xorshift(&mut st) * std::f32::consts::TAU;
                    let d = xorshift(&mut st).sqrt() * r;
                    clamp_terrain(a + Vec2::new(ang.cos(), ang.sin()) * d)
                })
                .collect()
        }
        PlaceMode::Rect => {
            let (lo, hi) = (a.min(b), a.max(b));
            (0..count)
                .map(|_| {
                    clamp_terrain(Vec2::new(
                        lo.x + xorshift(&mut st) * (hi.x - lo.x),
                        lo.y + xorshift(&mut st) * (hi.y - lo.y),
                    ))
                })
                .collect()
        }
    }
}

// --------------------------------------------------------------------- file

fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Giorni dal 1970 -> data civile (algoritmo di Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60
    )
}

fn file_path() -> std::path::PathBuf {
    let rel = std::path::PathBuf::from(FILE);
    if rel.parent().is_some_and(|p| p.exists()) || std::path::Path::new("Cargo.toml").exists() {
        rel
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FILE)
    }
}

const HEADER: &str = "\
# Piazzamenti richiesti in modalità debug (F4)
#
# Come leggere questo file
# - Coordinate in metri sul piano del terreno: x = Est (+) / Ovest (-), z = Sud (+) / Nord (-)
#   (il nord è -Z, come la bussola del gioco). Il terreno va da -50 a +50 su entrambi gli assi.
# - Lo spawn del giocatore è (0, 0). La capanna del Viandante è centrata a circa (6.5, 0.7).
# - Ogni blocco [P-nnnn] è una richiesta: 'tipo' è l'oggetto da piazzare, 'modalità' come è stato
#   indicato (singolo, gruppo-raggio, area-rettangolo, area-cerchio), 'punti' le coordinate
#   effettive (x,z) di ogni oggetto (fonte di verità), 'nota' un commento dell'utente.
# - File riscritto dal gioco a ogni modifica e riletto all'avvio: non rinominare le chiavi.
";

fn write_file(items: &[Placement]) {
    let mut out = String::from(HEADER);
    for p in items {
        let _ = writeln!(
            out,
            "\n[P-{:04}] tipo: {} | modalità: {}",
            p.id,
            p.kind,
            p.mode.label()
        );
        let _ = writeln!(out, "quando: {}", p.when);
        match p.mode {
            PlaceMode::Single => {
                let _ = writeln!(out, "punto: x {:.2}, z {:.2}", p.a.x, p.a.y);
            }
            PlaceMode::Group => {
                let _ = writeln!(
                    out,
                    "centro: x {:.2}, z {:.2}   raggio: {:.1}",
                    p.a.x, p.a.y, p.radius
                );
            }
            PlaceMode::Rect => {
                let _ = writeln!(
                    out,
                    "area: x {:.2}..{:.2}, z {:.2}..{:.2}",
                    p.a.x.min(p.b.x),
                    p.a.x.max(p.b.x),
                    p.a.y.min(p.b.y),
                    p.a.y.max(p.b.y)
                );
            }
            PlaceMode::Circle => {
                let _ = writeln!(
                    out,
                    "centro: x {:.2}, z {:.2}   raggio: {:.1}",
                    p.a.x,
                    p.a.y,
                    p.a.distance(p.b)
                );
            }
        }
        let _ = writeln!(out, "quantità: {}   seed: {}", p.points.len(), p.seed);
        if !p.note.is_empty() {
            let _ = writeln!(out, "nota: {}", p.note);
        }
        let pts: Vec<String> = p
            .points
            .iter()
            .map(|q| format!("({:.2},{:.2})", q.x, q.y))
            .collect();
        let _ = writeln!(out, "punti: {}", pts.join(" "));
    }
    let path = file_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match std::fs::write(&path, out) {
        Ok(()) => info!(
            "[DEBUG] salvato {} ({} richieste)",
            path.display(),
            items.len()
        ),
        Err(e) => warn!("[DEBUG] impossibile salvare {}: {e}", path.display()),
    }
}

/// Numero dopo `key` (es. "x 12.3" -> 12.3).
fn num_after(s: &str, key: &str) -> Option<f32> {
    let i = s.find(key)? + key.len();
    s[i..]
        .trim_start()
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .next()?
        .parse()
        .ok()
}

fn read_file() -> Vec<Placement> {
    let Ok(text) = std::fs::read_to_string(file_path()) else {
        return Vec::new();
    };
    let mut items: Vec<Placement> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("[P-")
            && let Some((id, tail)) = rest.split_once(']')
            && let Ok(id) = id.parse::<u32>()
        {
            let kind = tail
                .split("tipo:")
                .nth(1)
                .and_then(|t| t.split('|').next())
                .unwrap_or("altro")
                .trim()
                .to_string();
            let mode = tail
                .split("modalità:")
                .nth(1)
                .and_then(|t| PlaceMode::parse(t.trim()))
                .unwrap_or(PlaceMode::Single);
            items.push(Placement {
                id,
                kind,
                mode,
                a: Vec2::ZERO,
                b: Vec2::ZERO,
                radius: 0.0,
                seed: 1,
                note: String::new(),
                when: String::new(),
                points: Vec::new(),
            });
            continue;
        }
        let Some(p) = items.last_mut() else {
            continue;
        };
        if let Some(v) = line.strip_prefix("quando:") {
            p.when = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("nota:") {
            p.note = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("punto:") {
            p.a = Vec2::new(
                num_after(v, "x").unwrap_or(0.0),
                num_after(v, "z").unwrap_or(0.0),
            );
        } else if let Some(v) = line.strip_prefix("centro:") {
            p.a = Vec2::new(
                num_after(v, "x").unwrap_or(0.0),
                num_after(v, "z").unwrap_or(0.0),
            );
            p.radius = num_after(v, "raggio:").unwrap_or(0.0);
            p.b = p.a + Vec2::new(p.radius, 0.0);
        } else if let Some(v) = line.strip_prefix("area:") {
            // "x lo..hi, z lo..hi"
            let nums: Vec<f32> = v
                .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                .filter(|t| !t.is_empty() && *t != "." && *t != "-")
                .filter_map(|t| t.parse().ok())
                .collect();
            if nums.len() >= 4 {
                p.a = Vec2::new(nums[0], nums[2]);
                p.b = Vec2::new(nums[1], nums[3]);
            }
        } else if let Some(v) = line.strip_prefix("quantità:") {
            p.seed = num_after(v, "seed:").unwrap_or(1.0) as u64;
        } else if let Some(v) = line.strip_prefix("punti:") {
            p.points = v
                .split(')')
                .filter_map(|t| {
                    let t = t.trim().trim_start_matches('(');
                    let (x, z) = t.split_once(',')?;
                    Some(Vec2::new(x.trim().parse().ok()?, z.trim().parse().ok()?))
                })
                .collect();
        }
    }
    items
}

// ----------------------------------------------------------------- segnaposto

/// Segnaposto 3D di un oggetto di un piazzamento.
#[derive(Component)]
pub struct DebugMarker;

fn kind_color(kind: &str) -> Color {
    match kind {
        "albero" => Color::srgb(0.15, 0.55, 0.2),
        "roccia" => Color::srgb(0.55, 0.55, 0.58),
        "cespuglio" => Color::srgb(0.25, 0.65, 0.3),
        "fiori" => Color::srgb(0.9, 0.4, 0.7),
        "casa" => Color::srgb(0.85, 0.75, 0.55),
        _ => Color::srgb(0.95, 0.95, 0.95),
    }
}

/// Sistema: ricrea i segnaposto quando cambiano i piazzamenti; visibili solo con F4 attivo.
pub fn sync_markers(
    mut commands: Commands,
    mut dbg: ResMut<DebugPlace>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    markers: Query<Entity, With<DebugMarker>>,
    mut vis: Query<&mut Visibility, With<DebugMarker>>,
) {
    let want = if dbg.on {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut vis {
        if *v != want {
            *v = want;
        }
    }
    if !dbg.markers_dirty {
        return;
    }
    dbg.markers_dirty = false;
    for e in &markers {
        commands.entity(e).despawn();
    }
    let trunk = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.27, 0.15),
        ..default()
    });
    let trunk_mesh = meshes.add(Cylinder::new(0.15, 1.2));
    let cone = meshes.add(Cone::new(0.8, 2.0));
    let sphere = meshes.add(Sphere::new(0.5));
    let cube = meshes.add(Cuboid::new(2.0, 2.0, 2.0));
    let pillar = meshes.add(Cylinder::new(0.2, 2.0));
    for p in &dbg.items {
        let mat = materials.add(StandardMaterial {
            base_color: kind_color(&p.kind),
            unlit: false,
            ..default()
        });
        for q in &p.points {
            let mut e = commands.spawn((
                DebugMarker,
                crate::scene::SceneNode,
                SnapToGround {
                    offset: 0.0,
                    half: Vec2::ZERO,
                },
                Transform::from_xyz(q.x, 0.0, q.y),
                Visibility::Hidden,
            ));
            e.with_children(|c| match p.kind.as_str() {
                "albero" => {
                    c.spawn((
                        Mesh3d(trunk_mesh.clone()),
                        MeshMaterial3d(trunk.clone()),
                        Transform::from_xyz(0.0, 0.6, 0.0),
                    ));
                    c.spawn((
                        Mesh3d(cone.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_xyz(0.0, 2.2, 0.0),
                    ));
                }
                "roccia" | "cespuglio" | "fiori" => {
                    c.spawn((
                        Mesh3d(sphere.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_xyz(0.0, 0.4, 0.0),
                    ));
                }
                "casa" => {
                    c.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_xyz(0.0, 1.0, 0.0),
                    ));
                }
                _ => {
                    c.spawn((
                        Mesh3d(pillar.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_xyz(0.0, 1.0, 0.0),
                    ));
                }
            });
        }
    }
}

// ------------------------------------------------------------------------ UI

/// Punto del terreno sul piano y=0 sotto il cursore, con la camera ortografica della mappa.
fn cursor_ground(camera: (&Camera, &GlobalTransform), cursor: egui::Pos2) -> Option<Vec2> {
    let ray = camera
        .0
        .viewport_to_world(camera.1, Vec2::new(cursor.x, cursor.y))
        .ok()?;
    let t = -ray.origin.y / ray.direction.y;
    (t > 0.0).then(|| {
        let p = ray.origin + *ray.direction * t;
        Vec2::new(p.x, p.z)
    })
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct DebugCtx<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    state: Res<'w, State<AppState>>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<Camera3d>>,
    spatial: SpatialQuery<'w, 's>,
    grounds: Query<'w, 's, (), With<Ground>>,
    clock: Res<'w, GameClock>,
}

/// Sistema (EguiPrimaryContextPass): tasto F4, pannello, piazzamento ed eliminazione.
pub fn debug_ui(mut dbg: ResMut<DebugPlace>, mut cx: DebugCtx) {
    if cx.keys.just_pressed(KeyCode::F4)
        && matches!(
            cx.state.get(),
            AppState::Playing | AppState::Map | AppState::Paused
        )
    {
        dbg.on = !dbg.on;
        info!("[DEBUG] modalità piazzamenti: {}", dbg.on);
    }
    let state = cx.state.get().clone();
    if !dbg.on || !matches!(state, AppState::Playing | AppState::Map) {
        return;
    }
    let in_map = state == AppState::Map;
    let Ok(ctx) = cx.contexts.ctx_mut() else {
        return;
    };
    let ctx = ctx.clone();
    let camera = cx.cameras.single().ok();

    // --- Pannello ---
    let hour = cx.clock.label();
    egui::Window::new("Debug piazzamenti")
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -50.0))
        .resizable(false)
        .show(&ctx, |ui| {
            // Pannello compatto: caratteri più piccoli del resto dell'interfaccia.
            ui.style_mut().override_font_id = Some(egui::FontId::proportional(14.0));
            egui::ComboBox::from_label("Tipo")
                .selected_text(KINDS[dbg.kind_idx])
                .show_ui(ui, |ui| {
                    for (i, k) in KINDS.iter().enumerate() {
                        ui.selectable_value(&mut dbg.kind_idx, i, *k);
                    }
                });
            if KINDS[dbg.kind_idx] == "altro" {
                ui.text_edit_singleline(&mut dbg.custom_kind);
            }
            ui.horizontal_wrapped(|ui| {
                for m in [
                    PlaceMode::Single,
                    PlaceMode::Group,
                    PlaceMode::Rect,
                    PlaceMode::Circle,
                ] {
                    if ui.selectable_label(dbg.mode == m, m.label()).clicked() {
                        dbg.mode = m;
                        dbg.pending = None;
                    }
                }
            });
            if matches!(
                dbg.mode,
                PlaceMode::Group | PlaceMode::Rect | PlaceMode::Circle
            ) {
                ui.add(
                    egui::DragValue::new(&mut dbg.count)
                        .range(1..=500)
                        .prefix("quantità "),
                );
            }
            if dbg.mode == PlaceMode::Group {
                ui.add(
                    egui::DragValue::new(&mut dbg.radius)
                        .range(1.0..=40.0)
                        .prefix("raggio ")
                        .suffix(" m"),
                );
            }
            ui.add(egui::TextEdit::singleline(&mut dbg.note).hint_text("nota (facoltativa)"));
            let hint = match (dbg.mode, dbg.pending) {
                (PlaceMode::Rect | PlaceMode::Circle, Some(p)) => {
                    format!("Primo punto ({:.1}, {:.1}): indica il secondo", p.x, p.y)
                }
                (PlaceMode::Rect, None) => "Indica il primo angolo".to_string(),
                (PlaceMode::Circle, None) => "Indica il centro, poi un punto sul bordo".to_string(),
                _ => "Indica il punto".to_string(),
            };
            ui.label(hint);
            ui.label(if in_map {
                "Mappa: clic = aggiungi, clic destro = elimina il punto più vicino"
            } else {
                "3D: P = aggiungi dove guardi, X = elimina il più vicino"
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Annulla ultimo").clicked() {
                    dbg.items.pop();
                    dbg.changed();
                }
                if ui.button("Svuota").clicked() {
                    dbg.items.clear();
                    dbg.changed();
                }
            });
            let mut del: Option<u32> = None;
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    for p in &dbg.items {
                        ui.horizontal(|ui| {
                            ui.label(format!("P-{:04} {} ×{}", p.id, p.kind, p.points.len()));
                            if ui.small_button("Elimina").clicked() {
                                del = Some(p.id);
                            }
                        });
                    }
                });
            if let Some(id) = del {
                dbg.items.retain(|p| p.id != id);
                dbg.changed();
            }
            ui.label(format!("File: {FILE}  (ora di gioco {hour})"));
        });

    // --- Punto indicato (clic sulla mappa o P/X in 3D) ---
    let mut add_at: Option<Vec2> = None;
    let mut remove_near: Option<(Vec2, f32)> = None;
    if in_map {
        if let Some(cam) = camera
            && !ctx.is_pointer_over_egui()
            && let Some(pos) = ctx.input(|i| i.pointer.latest_pos())
            && let Some(w) = cursor_ground(cam, pos)
        {
            if ctx.input(|i| i.pointer.primary_clicked()) {
                add_at = Some(w);
            }
            if ctx.input(|i| i.pointer.secondary_clicked()) {
                if dbg.pending.is_some() {
                    dbg.pending = None;
                } else {
                    remove_near = Some((w, 2.5));
                }
            }
        }
    } else if !ctx.egui_wants_keyboard_input() {
        // Punto guardato col mirino.
        let target = camera.and_then(|(_, t)| {
            cx.spatial
                .cast_ray_predicate(
                    t.translation(),
                    t.forward(),
                    90.0,
                    true,
                    &SpatialQueryFilter::default(),
                    &|e| cx.grounds.contains(e),
                )
                .map(|h| {
                    let p = t.translation() + *t.forward() * h.distance;
                    Vec2::new(p.x, p.z)
                })
        });
        if let Some(w) = target {
            if cx.keys.just_pressed(KeyCode::KeyP) {
                add_at = Some(w);
            }
            if cx.keys.just_pressed(KeyCode::KeyX) {
                remove_near = Some((w, 3.0));
            }
        }
    }
    if let Some(w) = add_at {
        place(&mut dbg, w, &hour);
    }
    if let Some((w, r)) = remove_near {
        remove_nearest(&mut dbg, w, r);
    }

    // --- Etichette e contorni (mappa: tutti; 3D: vicini) ---
    if let Some((cam, cam_t)) = camera {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("debug_labels"),
        ));
        let proj = |p: Vec2| cam.world_to_viewport(cam_t, Vec3::new(p.x, 0.5, p.y)).ok();
        for pl in &dbg.items {
            let Some(first) = pl.points.first() else {
                continue;
            };
            let near = in_map
                || cam_t
                    .translation()
                    .distance(Vec3::new(first.x, 0.0, first.y))
                    < 40.0;
            if !near {
                continue;
            }
            if let Some(s) =
                proj(pl.points.iter().fold(Vec2::ZERO, |a, b| a + *b) / pl.points.len() as f32)
            {
                painter.text(
                    egui::pos2(s.x, s.y - 14.0),
                    egui::Align2::CENTER_BOTTOM,
                    format!("P-{:04} {} ×{}", pl.id, pl.kind, pl.points.len()),
                    egui::FontId::proportional(14.0),
                    egui::Color32::from_rgb(255, 235, 120),
                );
            }
            let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 235, 120));
            match pl.mode {
                PlaceMode::Rect => {
                    let (lo, hi) = (pl.a.min(pl.b), pl.a.max(pl.b));
                    let c = [lo, Vec2::new(hi.x, lo.y), hi, Vec2::new(lo.x, hi.y), lo];
                    for w in c.windows(2) {
                        if let (Some(p), Some(q)) = (proj(w[0]), proj(w[1])) {
                            painter
                                .line_segment([egui::pos2(p.x, p.y), egui::pos2(q.x, q.y)], stroke);
                        }
                    }
                }
                PlaceMode::Circle | PlaceMode::Group => {
                    let r = if pl.mode == PlaceMode::Circle {
                        pl.a.distance(pl.b)
                    } else {
                        pl.radius
                    };
                    let pts: Vec<egui::Pos2> = (0..=48)
                        .filter_map(|i| {
                            let a = i as f32 / 48.0 * std::f32::consts::TAU;
                            proj(pl.a + Vec2::new(a.cos(), a.sin()) * r)
                                .map(|s| egui::pos2(s.x, s.y))
                        })
                        .collect();
                    painter.add(egui::Shape::line(pts, stroke));
                }
                PlaceMode::Single => {}
            }
        }
        if let Some(p) = dbg.pending.and_then(proj) {
            painter.circle_filled(
                egui::pos2(p.x, p.y),
                6.0,
                egui::Color32::from_rgb(255, 120, 60),
            );
        }
    }
}

/// Aggiunge un piazzamento (o imposta il primo punto di un'area).
fn place(dbg: &mut DebugPlace, w: Vec2, hour: &str) {
    let mode = dbg.mode;
    let (a, b) = match (mode, dbg.pending) {
        (PlaceMode::Rect | PlaceMode::Circle, None) => {
            dbg.pending = Some(w);
            return;
        }
        (PlaceMode::Rect | PlaceMode::Circle, Some(first)) => (first, w),
        _ => (w, w + Vec2::new(dbg.radius, 0.0)),
    };
    dbg.pending = None;
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.subsec_nanos() as u64 ^ (d.as_secs() << 7))
        % 1_000_000;
    let points = gen_points(mode, a, b, dbg.radius, dbg.count, seed);
    let id = dbg.next_id;
    dbg.next_id += 1;
    let note = dbg.note.trim().to_string();
    let when = format!("{} (ora di gioco {hour})", utc_now());
    let kind = dbg.kind();
    info!(
        "[DEBUG] P-{id:04} {kind} {} ×{}",
        mode.label(),
        points.len()
    );
    dbg.items.push(Placement {
        id,
        kind,
        mode,
        a,
        b,
        radius: dbg.radius,
        seed,
        note,
        when,
        points,
    });
    dbg.changed();
}

/// Elimina il punto più vicino a `w` entro `max_d` metri (un singolo oggetto).
fn remove_nearest(dbg: &mut DebugPlace, w: Vec2, max_d: f32) {
    let mut best: Option<(usize, usize, f32)> = None;
    for (i, p) in dbg.items.iter().enumerate() {
        for (j, q) in p.points.iter().enumerate() {
            let d = q.distance(w);
            if d <= max_d && best.is_none_or(|(_, _, bd)| d < bd) {
                best = Some((i, j, d));
            }
        }
    }
    if let Some((i, j, _)) = best {
        let id = dbg.items[i].id;
        dbg.items[i].points.remove(j);
        if dbg.items[i].points.is_empty() {
            dbg.items.remove(i);
        }
        info!("[DEBUG] rimosso un oggetto di P-{id:04}");
        dbg.changed();
    }
}
