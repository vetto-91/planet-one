//! Modalità debug per indicare dove piazzare oggetti (alberi, rocce, …).
//!
//! F4 attiva/disattiva il pannello; il cursore resta libero (in 3D si guarda
//! tenendo premuto il tasto destro). Si indica il punto col **clic sinistro**,
//! dalla mappa (M) o in 3D (il punto è quello del terreno sotto il cursore; P/X
//! restano come scorciatoie col mirino). Strumenti:
//!
//! - **Piazza**: singolo (con allineamento a griglia o a linea con passo uguale),
//!   gruppo attorno a un punto, rettangolo (4 angoli qualsiasi), cerchio (anteprima
//!   del raggio dal centro) e poligono (N punti, poi OK).
//! - **Gomma**: elimina l'oggetto più vicino; dall'elenco si elimina un intero piazzamento.
//! - **Sposta**: clic su un oggetto e poi sul nuovo punto; resta nel suo piazzamento.
//!
//! Ogni modifica riscrive `debug/placements.txt`, leggibile a mano e riletto
//! all'avvio (è anche la fonte degli oggetti veri: vedi `world_props`).

use crate::AppState;
use crate::daynight::GameClock;
use crate::props::SnapToGround;
use crate::scene::Ground;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use std::fmt::Write as _;

/// File con i piazzamenti (relativo alla cartella del progetto / di lavoro).
pub const FILE: &str = "debug/placements.txt";
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
    /// Quattro angoli qualsiasi.
    Rect,
    Circle,
    /// N punti chiusi con OK.
    Poly,
}

impl PlaceMode {
    fn label(self) -> &'static str {
        match self {
            PlaceMode::Single => "singolo",
            PlaceMode::Group => "gruppo-raggio",
            PlaceMode::Rect => "area-rettangolo",
            PlaceMode::Circle => "area-cerchio",
            PlaceMode::Poly => "area-poligono",
        }
    }
    fn ui_label(self) -> &'static str {
        match self {
            PlaceMode::Single => "Singolo",
            PlaceMode::Group => "Gruppo",
            PlaceMode::Rect => "Rettangolo",
            PlaceMode::Circle => "Cerchio",
            PlaceMode::Poly => "Poligono",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "singolo" => PlaceMode::Single,
            "gruppo-raggio" => PlaceMode::Group,
            "area-rettangolo" => PlaceMode::Rect,
            "area-cerchio" => PlaceMode::Circle,
            "area-poligono" => PlaceMode::Poly,
            _ => return None,
        })
    }
}

/// Strumento attivo.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Place,
    Erase,
    Move,
}

/// Allineamento del piazzamento singolo.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Free,
    Grid,
    Line,
}

/// Una richiesta di piazzamento (uno o più oggetti).
#[derive(Clone)]
pub struct Placement {
    pub id: u32,
    pub kind: String,
    pub mode: PlaceMode,
    /// Punto di riferimento: centro (gruppo/cerchio) o punto singolo.
    pub a: Vec2,
    /// Punto sul bordo (cerchio).
    pub b: Vec2,
    pub radius: f32,
    /// Angoli dell'area (rettangolo/poligono).
    pub poly: Vec<Vec2>,
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
    tool: Tool,
    kind_idx: usize,
    custom_kind: String,
    mode: PlaceMode,
    count: u32,
    radius: f32,
    /// Distribuzione regolare (passo uguale) invece che casuale.
    regular: bool,
    align: Align,
    grid_step: f32,
    line_step: f32,
    /// Linea di allineamento: punto di partenza e direzione (dal secondo punto).
    line_anchor: Option<Vec2>,
    line_dir: Option<Vec2>,
    note: String,
    /// Punti già indicati di un'area (centro, angoli…) in attesa del completamento.
    pending: Vec<Vec2>,
    /// Oggetto preso dallo strumento Sposta: (indice piazzamento, indice punto).
    grabbed: Option<(usize, usize)>,
    /// Seme della distribuzione corrente (uguale per anteprima e risultato).
    cur_seed: u64,
    /// I segnaposto 3D vanno ricreati.
    markers_dirty: bool,
    /// Il pulsante OK del poligono è stato premuto.
    finish_requested: bool,
    /// Aumenta a ogni modifica dei piazzamenti (gli oggetti veri si rigenerano).
    revision: u32,
}

fn new_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.subsec_nanos() as u64 ^ (d.as_secs() << 7))
        % 1_000_000
        + 1
}

impl Default for DebugPlace {
    fn default() -> Self {
        let mut d = Self {
            on: false,
            items: Vec::new(),
            next_id: 1,
            tool: Tool::Place,
            kind_idx: 0,
            custom_kind: String::new(),
            mode: PlaceMode::Single,
            count: 10,
            radius: 5.0,
            regular: true,
            align: Align::Free,
            grid_step: 1.0,
            line_step: 1.5,
            line_anchor: None,
            line_dir: None,
            note: String::new(),
            pending: Vec::new(),
            grabbed: None,
            cur_seed: new_seed(),
            markers_dirty: true,
            finish_requested: false,
            revision: 1,
        };
        d.items = read_file();
        d.next_id = d.items.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        info!("[DEBUG] piazzamenti letti da {FILE}: {}", d.items.len());
        d
    }
}

impl DebugPlace {
    pub fn revision(&self) -> u32 {
        self.revision
    }

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
        self.revision += 1;
        self.grabbed = None;
    }

    fn reset_pending(&mut self) {
        self.pending.clear();
        self.grabbed = None;
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

/// Punto dentro il poligono (regola pari-dispari).
fn in_poly(poly: &[Vec2], p: Vec2) -> bool {
    let mut inside = false;
    let mut j = poly.len().wrapping_sub(1);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Riempie una regione (riquadro + test di appartenenza) con `count` punti:
/// regolari (griglia a passo uguale, centrata) oppure casuali.
fn fill_region(
    lo: Vec2,
    hi: Vec2,
    inside: &dyn Fn(Vec2) -> bool,
    count: u32,
    regular: bool,
    seed: u64,
) -> Vec<Vec2> {
    let count = count.max(1) as usize;
    let size = hi - lo;
    if size.x < 0.01 || size.y < 0.01 {
        return vec![(lo + hi) / 2.0];
    }
    if !regular {
        let mut st = seed | 1;
        let mut out = Vec::with_capacity(count);
        let mut tries = 0;
        while out.len() < count && tries < count * 400 {
            tries += 1;
            let p = Vec2::new(
                lo.x + xorshift(&mut st) * size.x,
                lo.y + xorshift(&mut st) * size.y,
            );
            if inside(p) {
                out.push(p);
            }
        }
        return out;
    }
    let grid = |s: f32| -> Vec<Vec2> {
        let mut v = Vec::new();
        let mut z = lo.y + s / 2.0;
        while z < hi.y {
            let mut x = lo.x + s / 2.0;
            while x < hi.x {
                let p = Vec2::new(x, z);
                if inside(p) {
                    v.push(p);
                }
                x += s;
            }
            z += s;
        }
        v
    };
    let mut s_lo = (size.x * size.y / (count as f32 * 30.0)).sqrt().max(0.05);
    let mut s_hi = size.x.max(size.y);
    if grid(s_lo).len() < count {
        return grid(s_lo);
    }
    for _ in 0..24 {
        let mid = (s_lo + s_hi) / 2.0;
        if grid(mid).len() >= count {
            s_lo = mid;
        } else {
            s_hi = mid;
        }
    }
    let g = grid(s_lo);
    if g.len() <= count {
        return g;
    }
    // Più punti del richiesto: ne tengo `count` ben distribuiti.
    (0..count).map(|i| g[i * g.len() / count]).collect()
}

fn poly_bounds(poly: &[Vec2]) -> (Vec2, Vec2) {
    poly.iter().fold(
        (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    )
}

/// Punti di un piazzamento (anteprima e risultato usano la stessa funzione).
#[allow(clippy::too_many_arguments)]
fn gen_points(
    mode: PlaceMode,
    a: Vec2,
    b: Vec2,
    poly: &[Vec2],
    radius: f32,
    count: u32,
    regular: bool,
    seed: u64,
) -> Vec<Vec2> {
    let pts = match mode {
        PlaceMode::Single => vec![a],
        PlaceMode::Group | PlaceMode::Circle => {
            let r = if mode == PlaceMode::Circle {
                a.distance(b)
            } else {
                radius
            };
            fill_region(
                a - Vec2::splat(r),
                a + Vec2::splat(r),
                &|p| p.distance(a) <= r,
                count,
                regular,
                seed,
            )
        }
        PlaceMode::Rect | PlaceMode::Poly => {
            if poly.len() < 3 {
                return Vec::new();
            }
            let (lo, hi) = poly_bounds(poly);
            fill_region(lo, hi, &|p| in_poly(poly, p), count, regular, seed)
        }
    };
    pts.into_iter().map(clamp_terrain).collect()
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
            PlaceMode::Rect | PlaceMode::Poly => {
                let c: Vec<String> = p
                    .poly
                    .iter()
                    .map(|q| format!("({:.2},{:.2})", q.x, q.y))
                    .collect();
                let _ = writeln!(out, "angoli: {}", c.join(" "));
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

/// Elenco di coordinate `(x,z) (x,z) …`.
fn parse_points(v: &str) -> Vec<Vec2> {
    v.split(')')
        .filter_map(|t| {
            let t = t.trim().trim_start_matches('(');
            let (x, z) = t.split_once(',')?;
            Some(Vec2::new(x.trim().parse().ok()?, z.trim().parse().ok()?))
        })
        .collect()
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
                poly: Vec::new(),
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
                // Vecchio formato: rettangolo allineato agli assi.
                p.a = Vec2::new(nums[0], nums[2]);
                p.b = Vec2::new(nums[1], nums[3]);
                p.poly = vec![
                    Vec2::new(nums[0], nums[2]),
                    Vec2::new(nums[1], nums[2]),
                    Vec2::new(nums[1], nums[3]),
                    Vec2::new(nums[0], nums[3]),
                ];
            }
        } else if let Some(v) = line.strip_prefix("angoli:") {
            p.poly = parse_points(v);
            p.a = p.poly.first().copied().unwrap_or_default();
        } else if let Some(v) = line.strip_prefix("quantità:") {
            p.seed = num_after(v, "seed:").unwrap_or(1.0) as u64;
        } else if let Some(v) = line.strip_prefix("punti:") {
            p.points = parse_points(v);
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
    for p in dbg
        .items
        .iter()
        .filter(|p| !crate::world_props::has_model(&p.kind))
    {
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

/// Allinea il punto indicato (solo piazzamento singolo).
fn snap(dbg: &DebugPlace, w: Vec2) -> Vec2 {
    if dbg.tool != Tool::Place || dbg.mode != PlaceMode::Single {
        return w;
    }
    match dbg.align {
        Align::Free => w,
        Align::Grid => {
            let s = dbg.grid_step.max(0.1);
            (w / s).round() * s
        }
        Align::Line => {
            let s = dbg.line_step.max(0.1);
            match (dbg.line_anchor, dbg.line_dir) {
                (Some(a), Some(d)) => a + d * s * ((w - a).dot(d) / s).round(),
                (Some(a), None) => {
                    let v = w - a;
                    let len = v.length();
                    if len < 0.01 {
                        w
                    } else {
                        a + v / len * (len / s).round().max(1.0) * s
                    }
                }
                _ => w,
            }
        }
    }
}

const ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 160, 60);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(255, 235, 120);

/// Sistema (EguiPrimaryContextPass): tasto F4, pannello, strumenti e anteprime.
pub fn debug_ui(mut dbg: ResMut<DebugPlace>, mut cx: DebugCtx) {
    if cx.keys.just_pressed(KeyCode::F4)
        && matches!(
            cx.state.get(),
            AppState::Playing | AppState::Map | AppState::Paused
        )
    {
        dbg.on = !dbg.on;
        dbg.reset_pending();
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
    let hour = cx.clock.label();

    // --- Pannello ---
    egui::Window::new("Debug piazzamenti")
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -50.0))
        .resizable(false)
        .show(&ctx, |ui| panel(ui, &mut dbg, in_map, &hour));

    if std::mem::take(&mut dbg.finish_requested) {
        finish_poly(&mut dbg, &hour);
    }

    // --- Punto sotto il cursore ---
    let over_ui = ctx.is_pointer_over_egui();
    let looking = !in_map && ctx.input(|i| i.pointer.secondary_down());
    let hover: Option<Vec2> = if over_ui || looking {
        None
    } else {
        camera.and_then(|cam| {
            let pos = ctx.input(|i| i.pointer.latest_pos())?;
            if in_map {
                cursor_ground(cam, pos)
            } else {
                let ray = cam
                    .0
                    .viewport_to_world(cam.1, Vec2::new(pos.x, pos.y))
                    .ok()?;
                cx.spatial
                    .cast_ray_predicate(
                        ray.origin,
                        ray.direction,
                        200.0,
                        true,
                        &SpatialQueryFilter::default(),
                        &|e| cx.grounds.contains(e),
                    )
                    .map(|h| {
                        let p = ray.origin + *ray.direction * h.distance;
                        Vec2::new(p.x, p.z)
                    })
            }
        })
    };
    let snapped = hover.map(|w| snap(&dbg, w));
    let reach = if in_map { 2.5 } else { 1.5 };

    if let Some(w) = snapped {
        if ctx.input(|i| i.pointer.primary_clicked()) {
            click(&mut dbg, w, reach, &hour);
        }
        if in_map && ctx.input(|i| i.pointer.secondary_clicked()) {
            if !dbg.pending.is_empty() || dbg.grabbed.is_some() {
                dbg.reset_pending();
            } else {
                remove_nearest(&mut dbg, w, reach);
            }
        }
    }
    if !ctx.egui_wants_keyboard_input() {
        if cx.keys.just_pressed(KeyCode::Enter) || cx.keys.just_pressed(KeyCode::NumpadEnter) {
            finish_poly(&mut dbg, &hour);
        }
        if cx.keys.just_pressed(KeyCode::Backspace) {
            dbg.pending.pop();
        }
        // Scorciatoie 3D col mirino (P = aggiungi, X = elimina).
        if !in_map && let Some((_, t)) = camera {
            let target = cx
                .spatial
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
                });
            if let Some(w) = target {
                if cx.keys.just_pressed(KeyCode::KeyP) {
                    let w = snap(&dbg, w);
                    click(&mut dbg, w, reach, &hour);
                }
                if cx.keys.just_pressed(KeyCode::KeyX) {
                    remove_nearest(&mut dbg, w, 3.0);
                }
            }
        }
    }

    // --- Disegno: etichette, contorni, anteprime ---
    let Some((cam, cam_t)) = camera else {
        return;
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("debug_labels"),
    ));
    let proj = |p: Vec2| {
        cam.world_to_viewport(cam_t, Vec3::new(p.x, 0.5, p.y))
            .ok()
            .map(|s| egui::pos2(s.x, s.y))
    };
    let line = |pts: &[Vec2], closed: bool, stroke: egui::Stroke| {
        let mut v: Vec<egui::Pos2> = pts.iter().filter_map(|p| proj(*p)).collect();
        if closed && let Some(f) = v.first().copied() {
            v.push(f);
        }
        if v.len() >= 2 {
            painter.add(egui::Shape::line(v, stroke));
        }
    };
    let circle_pts = |c: Vec2, r: f32| -> Vec<Vec2> {
        (0..48)
            .map(|i| {
                let a = i as f32 / 48.0 * std::f32::consts::TAU;
                c + Vec2::new(a.cos(), a.sin()) * r
            })
            .collect()
    };
    let stroke = egui::Stroke::new(1.5, YELLOW);
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
                YELLOW,
            );
        }
        match pl.mode {
            PlaceMode::Rect | PlaceMode::Poly => line(&pl.poly, true, stroke),
            PlaceMode::Circle => line(&circle_pts(pl.a, pl.a.distance(pl.b)), true, stroke),
            PlaceMode::Group => line(&circle_pts(pl.a, pl.radius), true, stroke),
            PlaceMode::Single => {}
        }
    }
    if let Some((i, j)) = dbg.grabbed
        && let Some(q) = dbg.items.get(i).and_then(|p| p.points.get(j))
        && let Some(s) = proj(*q)
    {
        painter.circle_stroke(s, 12.0, egui::Stroke::new(3.0, ORANGE));
    }
    if dbg.tool != Tool::Place {
        // Gomma/Sposta: evidenzia l'oggetto più vicino al cursore.
        if let Some(w) = hover
            && let Some((i, j)) = nearest(&dbg, w, reach)
            && let Some(s) = proj(dbg.items[i].points[j])
        {
            painter.circle_stroke(s, 10.0, egui::Stroke::new(2.0, egui::Color32::LIGHT_RED));
        }
        return;
    }
    // Anteprime dello strumento Piazza.
    let pv = egui::Stroke::new(2.0, ORANGE);
    let dots = |pts: &[Vec2]| {
        for p in pts {
            if let Some(s) = proj(*p) {
                painter.circle_filled(s, 3.0, ORANGE);
            }
        }
    };
    for p in &dbg.pending {
        if let Some(s) = proj(*p) {
            painter.circle_filled(s, 6.0, egui::Color32::from_rgb(255, 120, 60));
        }
    }
    let preview_pts = |mode, a, b, poly: &[Vec2]| {
        gen_points(
            mode,
            a,
            b,
            poly,
            dbg.radius,
            dbg.count,
            dbg.regular,
            dbg.cur_seed,
        )
    };
    match (dbg.mode, snapped) {
        (PlaceMode::Single, Some(w)) => {
            if let Some(s) = proj(w) {
                painter.circle_stroke(s, 8.0, pv);
            }
            if dbg.align == Align::Line
                && let Some(a) = dbg.line_anchor
            {
                line(&[a, w], false, pv);
            }
        }
        (PlaceMode::Group, Some(w)) => {
            line(&circle_pts(w, dbg.radius), true, pv);
            dots(&preview_pts(PlaceMode::Group, w, w, &[]));
        }
        (PlaceMode::Circle, w) => {
            if let Some(&c) = dbg.pending.first() {
                // Raggio e cerchio in anteprima dal primo punto.
                let edge = w.unwrap_or(c + Vec2::X);
                let r = c.distance(edge);
                line(&circle_pts(c, r), true, pv);
                line(&[c, edge], false, pv);
                dots(&preview_pts(PlaceMode::Circle, c, edge, &[]));
                if let Some(s) = proj(edge) {
                    painter.text(
                        egui::pos2(s.x + 12.0, s.y - 12.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("raggio {r:.1} m"),
                        egui::FontId::proportional(15.0),
                        ORANGE,
                    );
                }
            }
        }
        (m @ (PlaceMode::Rect | PlaceMode::Poly), w) if !dbg.pending.is_empty() => {
            let mut shape = dbg.pending.clone();
            if let Some(w) = w {
                shape.push(w);
            }
            line(&shape, shape.len() >= 3, pv);
            if shape.len() >= 3 {
                dots(&preview_pts(m, shape[0], shape[0], &shape));
            }
        }
        _ => {}
    }
}

/// Contenuto del pannello.
fn panel(ui: &mut egui::Ui, dbg: &mut DebugPlace, in_map: bool, hour: &str) {
    // Pannello compatto: caratteri più piccoli del resto dell'interfaccia.
    ui.style_mut().override_font_id = Some(egui::FontId::proportional(14.0));
    ui.horizontal(|ui| {
        for (t, label) in [
            (Tool::Place, "Piazza"),
            (Tool::Erase, "Gomma"),
            (Tool::Move, "Sposta"),
        ] {
            if ui.selectable_label(dbg.tool == t, label).clicked() {
                dbg.tool = t;
                dbg.reset_pending();
            }
        }
    });
    match dbg.tool {
        Tool::Erase => {
            ui.label("Clic su un oggetto per eliminarlo (resta il resto del piazzamento).");
        }
        Tool::Move => {
            ui.label(if dbg.grabbed.is_some() {
                "Ora clic sul nuovo punto (clic destro in mappa = annulla)."
            } else {
                "Clic su un oggetto per prenderlo, poi clic dove metterlo."
            });
        }
        Tool::Place => place_panel(ui, dbg),
    }
    ui.label(if in_map {
        "Mappa: clic sinistro = scegli, clic destro = annulla / elimina, tasto centrale = sposta la vista"
    } else {
        "3D: clic sinistro = scegli sul terreno, tasto destro tenuto = guarda, P / X = col mirino"
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
}

fn place_panel(ui: &mut egui::Ui, dbg: &mut DebugPlace) {
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
            PlaceMode::Poly,
        ] {
            if ui.selectable_label(dbg.mode == m, m.ui_label()).clicked() {
                dbg.mode = m;
                dbg.reset_pending();
            }
        }
    });
    match dbg.mode {
        PlaceMode::Single => {
            ui.horizontal(|ui| {
                for (a, label) in [
                    (Align::Free, "Libero"),
                    (Align::Grid, "Griglia"),
                    (Align::Line, "Linea"),
                ] {
                    if ui.selectable_label(dbg.align == a, label).clicked() {
                        dbg.align = a;
                        dbg.line_anchor = None;
                        dbg.line_dir = None;
                    }
                }
            });
            match dbg.align {
                Align::Grid => {
                    ui.add(
                        egui::DragValue::new(&mut dbg.grid_step)
                            .range(0.2..=20.0)
                            .speed(0.05)
                            .prefix("passo ")
                            .suffix(" m"),
                    );
                }
                Align::Line => {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut dbg.line_step)
                                .range(0.2..=20.0)
                                .speed(0.05)
                                .prefix("distanza ")
                                .suffix(" m"),
                        );
                        if ui.button("Nuova linea").clicked() {
                            dbg.line_anchor = None;
                            dbg.line_dir = None;
                        }
                    });
                    ui.label(match (dbg.line_anchor, dbg.line_dir) {
                        (None, _) => "Linea: indica il primo punto",
                        (Some(_), None) => "Il secondo punto fissa la direzione",
                        _ => "Gli altri punti seguono la linea a distanza uguale",
                    });
                }
                Align::Free => {}
            }
        }
        PlaceMode::Group => {
            ui.add(
                egui::DragValue::new(&mut dbg.radius)
                    .range(1.0..=40.0)
                    .prefix("raggio ")
                    .suffix(" m"),
            );
        }
        _ => {}
    }
    if dbg.mode != PlaceMode::Single {
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut dbg.count)
                    .range(1..=500)
                    .prefix("quantità "),
            );
            ui.checkbox(&mut dbg.regular, "regolare");
        });
    }
    ui.add(egui::TextEdit::singleline(&mut dbg.note).hint_text("nota (facoltativa)"));
    let hint = match (dbg.mode, dbg.pending.len()) {
        (PlaceMode::Circle, 0) => "Indica il centro, poi un punto sul bordo".to_string(),
        (PlaceMode::Circle, _) => "Indica un punto sul bordo (il cerchio è in anteprima)".into(),
        (PlaceMode::Rect, n) => format!("Rettangolo: angolo {} di 4", n + 1),
        (PlaceMode::Poly, 0) => "Poligono: indica i punti, poi OK".to_string(),
        (PlaceMode::Poly, n) => format!("Poligono: {n} punti (servono almeno 3), poi OK"),
        (PlaceMode::Group, _) => "Indica il centro del gruppo".to_string(),
        _ => "Indica il punto".to_string(),
    };
    ui.label(hint);
    if matches!(
        dbg.mode,
        PlaceMode::Poly | PlaceMode::Rect | PlaceMode::Circle
    ) && !dbg.pending.is_empty()
    {
        ui.horizontal(|ui| {
            if dbg.mode == PlaceMode::Poly
                && dbg.pending.len() >= 3
                && ui.button("OK (Invio)").clicked()
            {
                dbg.finish_requested = true;
            }
            if ui.button("Togli ultimo punto").clicked() {
                dbg.pending.pop();
            }
            if ui.button("Annulla area").clicked() {
                dbg.pending.clear();
            }
        });
    }
}

/// Oggetto più vicino a `w` entro `max_d` metri: (piazzamento, punto).
fn nearest(dbg: &DebugPlace, w: Vec2, max_d: f32) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, f32)> = None;
    for (i, p) in dbg.items.iter().enumerate() {
        for (j, q) in p.points.iter().enumerate() {
            let d = q.distance(w);
            if d <= max_d && best.is_none_or(|(_, _, bd)| d < bd) {
                best = Some((i, j, d));
            }
        }
    }
    best.map(|(i, j, _)| (i, j))
}

/// Clic sinistro con lo strumento attivo.
fn click(dbg: &mut DebugPlace, w: Vec2, reach: f32, hour: &str) {
    match dbg.tool {
        Tool::Erase => remove_nearest(dbg, w, reach),
        Tool::Move => match dbg.grabbed {
            None => dbg.grabbed = nearest(dbg, w, reach.max(2.0)),
            Some((i, j)) => {
                if let Some(q) = dbg.items.get_mut(i).and_then(|p| p.points.get_mut(j)) {
                    *q = clamp_terrain(w);
                    let id = dbg.items[i].id;
                    info!(
                        "[DEBUG] spostato un oggetto di P-{id:04} a ({:.2},{:.2})",
                        w.x, w.y
                    );
                }
                dbg.changed();
            }
        },
        Tool::Place => match dbg.mode {
            PlaceMode::Single => {
                if dbg
                    .items
                    .iter()
                    .any(|p| p.points.iter().any(|q| q.distance(w) < 0.05))
                {
                    return;
                }
                if dbg.align == Align::Line {
                    match (dbg.line_anchor, dbg.line_dir) {
                        (None, _) => dbg.line_anchor = Some(w),
                        (Some(a), None) => dbg.line_dir = (w - a).try_normalize(),
                        _ => {}
                    }
                }
                add_placement(dbg, PlaceMode::Single, w, w, Vec::new(), hour);
            }
            PlaceMode::Group => add_placement(dbg, PlaceMode::Group, w, w, Vec::new(), hour),
            PlaceMode::Circle => {
                if let Some(&c) = dbg.pending.first() {
                    if c.distance(w) > 0.3 {
                        add_placement(dbg, PlaceMode::Circle, c, w, Vec::new(), hour);
                    }
                } else {
                    dbg.pending.push(w);
                }
            }
            PlaceMode::Rect => {
                dbg.pending.push(w);
                if dbg.pending.len() == 4 {
                    let poly = std::mem::take(&mut dbg.pending);
                    add_placement(dbg, PlaceMode::Rect, poly[0], poly[0], poly, hour);
                }
            }
            PlaceMode::Poly => dbg.pending.push(w),
        },
    }
}

/// Chiude il poligono in costruzione (tasto OK / Invio).
fn finish_poly(dbg: &mut DebugPlace, hour: &str) {
    if dbg.tool == Tool::Place && dbg.mode == PlaceMode::Poly && dbg.pending.len() >= 3 {
        let poly = std::mem::take(&mut dbg.pending);
        add_placement(dbg, PlaceMode::Poly, poly[0], poly[0], poly, hour);
    }
}

/// Aggiunge un piazzamento e genera i suoi oggetti.
fn add_placement(
    dbg: &mut DebugPlace,
    mode: PlaceMode,
    a: Vec2,
    b: Vec2,
    poly: Vec<Vec2>,
    hour: &str,
) {
    let seed = dbg.cur_seed;
    let points = gen_points(mode, a, b, &poly, dbg.radius, dbg.count, dbg.regular, seed);
    if points.is_empty() {
        return;
    }
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
        poly,
        seed,
        note,
        when,
        points,
    });
    dbg.cur_seed = new_seed();
    dbg.pending.clear();
    dbg.changed();
}

/// Elimina il punto più vicino a `w` entro `max_d` metri (un singolo oggetto).
fn remove_nearest(dbg: &mut DebugPlace, w: Vec2, max_d: f32) {
    if let Some((i, j)) = nearest(dbg, w, max_d) {
        let id = dbg.items[i].id;
        dbg.items[i].points.remove(j);
        if dbg.items[i].points.is_empty() {
            dbg.items.remove(i);
        }
        info!("[DEBUG] rimosso un oggetto di P-{id:04}");
        dbg.changed();
    }
}
