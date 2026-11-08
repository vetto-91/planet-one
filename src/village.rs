//! Villaggio murato: mura, torrette, cancelli e dati (nome, strade, piazza).
//!
//! Le mura sono i piazzamenti `muro` di `debug/placements.txt` (vertici in ordine di id,
//! poligono chiuso): spostandoli in F4 il muro si rigenera. Nome, cancelli e strade stanno in
//! `assets/world/villaggio.txt` (modificabile a mano). Gli edifici sono piazzamenti normali
//! (vedi `buildings.rs`). Le strade a selciato sono disegnate dal terreno (`terrain.rs`).

use crate::buildings::{B, Col, ROOFS, STONE, STONE_DARK};
use crate::camera::BlocksCamera;
use crate::daynight::GameClock;
use crate::debug_place::{DebugPlace, Placement};
use crate::entities::Interactable;
use crate::hut::{DoorCollider, Hinge, PropKind, PropState};
use crate::scene::SceneNode;
use crate::terrain::TerrainField;
use avian3d::prelude::*;
use bevy::prelude::*;

const SPEC: &str = include_str!("../assets/world/villaggio.txt");

/// Altezza del muro sul terreno e spessore (m).
const WALL_H: f32 = 3.2;
const WALL_T: f32 = 0.9;
/// Semi-larghezza del vano del cancello e dei piloni.
const GATE_HALF: f32 = 2.2;
const PIER_HALF: f32 = 0.55;
/// Ora di apertura e chiusura dei cancelli (battenti).
const OPEN_HOUR: f32 = 6.0;
const CLOSE_HOUR: f32 = 21.0;

/// Una strada: larghezza (m) e polilinea.
#[derive(Clone, Debug)]
pub struct Road {
    pub width: f32,
    pub pts: Vec<Vec2>,
}

/// Dati letti da `villaggio.txt`.
#[derive(Clone, Debug, Default)]
pub struct VillageSpec {
    pub name: String,
    pub plaza: Vec2,
    pub gates: Vec<(String, Vec2)>,
    pub roads: Vec<Road>,
}

fn parse_spec(text: &str) -> VillageSpec {
    let mut s = VillageSpec {
        name: "Villaggio".into(),
        ..Default::default()
    };
    let nums = |v: &str| -> Vec<f32> {
        v.split(|c: char| c.is_whitespace() || c == ',')
            .filter_map(|t| t.parse().ok())
            .collect()
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let v = v.trim();
        match k.trim() {
            "nome" => s.name = v.to_string(),
            "piazza" => {
                let n = nums(v);
                if n.len() >= 2 {
                    s.plaza = Vec2::new(n[0], n[1]);
                }
            }
            "cancello" => {
                if let Some((name, rest)) = v.split_once(' ') {
                    let n = nums(rest);
                    if n.len() >= 2 {
                        s.gates.push((name.to_string(), Vec2::new(n[0], n[1])));
                    }
                }
            }
            "strada" => {
                if let Some((head, pts)) = v.split_once('|') {
                    let width = head
                        .split_whitespace()
                        .nth(1)
                        .and_then(|w| w.parse().ok())
                        .unwrap_or(2.0);
                    // "x z, x z, ..." -> coppie
                    let pts: Vec<Vec2> = pts
                        .split(',')
                        .filter_map(|p| {
                            let n = nums(p);
                            (n.len() >= 2).then(|| Vec2::new(n[0], n[1]))
                        })
                        .collect();
                    if pts.len() >= 2 {
                        s.roads.push(Road { width, pts });
                    }
                }
            }
            _ => {}
        }
    }
    s
}

/// Legge `assets/world/villaggio.txt` (disco, poi copia incorporata).
pub fn load_spec() -> VillageSpec {
    let rel = "assets/world/villaggio.txt";
    let candidates = [
        std::path::PathBuf::from(rel),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel),
    ];
    match candidates
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
    {
        Some(t) => parse_spec(&t),
        None => parse_spec(SPEC),
    }
}

/// Vertici del muro: i piazzamenti `muro` in ordine di id.
pub fn wall_polygon(items: &[Placement]) -> Vec<Vec2> {
    let mut walls: Vec<&Placement> = items.iter().filter(|p| p.kind == "muro").collect();
    walls.sort_by_key(|p| p.id);
    walls
        .iter()
        .flat_map(|p| p.points.iter().copied())
        .collect()
}

/// Punto dentro il poligono (regola pari-dispari).
pub fn in_polygon(poly: &[Vec2], p: Vec2) -> bool {
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

/// Distanza (m) del punto dal poligono: negativa dentro.
pub fn signed_distance(poly: &[Vec2], p: Vec2) -> f32 {
    let mut d = f32::MAX;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        d = d.min(dist_to_segment(p, a, b));
    }
    if in_polygon(poly, p) { -d } else { d }
}

pub fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let l2 = ab.length_squared();
    let t = if l2 < 1e-6 {
        0.0
    } else {
        ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
    };
    p.distance(a + ab * t)
}

/// Cancello proiettato sul muro: posizione, tangente (lungo il muro) e lato verso l'interno.
#[derive(Clone, Debug)]
pub struct Gate {
    #[allow(dead_code)]
    pub name: String,
    pub pos: Vec2,
    pub tangent: Vec2,
    pub inward: Vec2,
    pub edge: usize,
    /// Distanza (m) dall'inizio del tratto di muro.
    pub along: f32,
}

/// Proietta i punti indicativi sul lato di muro più vicino.
pub fn project_gates(poly: &[Vec2], hints: &[(String, Vec2)]) -> Vec<Gate> {
    if poly.len() < 3 {
        return Vec::new();
    }
    let center = poly.iter().fold(Vec2::ZERO, |a, b| a + *b) / poly.len() as f32;
    hints
        .iter()
        .map(|(name, hint)| {
            let mut best: Option<(f32, Gate)> = None;
            for i in 0..poly.len() {
                let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
                let ab = b - a;
                let l = ab.length();
                let t = ((*hint - a).dot(ab) / (l * l)).clamp(0.0, 1.0);
                let pos = a + ab * t;
                let d = pos.distance(*hint);
                if best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                    let tangent = ab / l;
                    let mut inward = Vec2::new(-tangent.y, tangent.x);
                    if inward.dot(center - pos) < 0.0 {
                        inward = -inward;
                    }
                    best = Some((
                        d,
                        Gate {
                            name: name.clone(),
                            pos,
                            tangent,
                            inward,
                            edge: i,
                            along: t * l,
                        },
                    ));
                }
            }
            best.map(|(_, g)| g).unwrap_or(Gate {
                name: name.clone(),
                pos: *hint,
                tangent: Vec2::X,
                inward: Vec2::Y,
                edge: 0,
                along: 0.0,
            })
        })
        .collect()
}

/// Dati del villaggio per il resto del gioco (mappa, terreno, banner).
#[derive(Resource, Default, Clone)]
pub struct VillageInfo {
    pub spec: VillageSpec,
    pub poly: Vec<Vec2>,
    pub gates: Vec<Gate>,
    pub center: Vec2,
}

impl VillageInfo {
    pub fn build(items: &[Placement]) -> Self {
        let spec = load_spec();
        let poly = wall_polygon(items);
        let gates = project_gates(&poly, &spec.gates);
        let center = if poly.is_empty() {
            Vec2::ZERO
        } else {
            poly.iter().fold(Vec2::ZERO, |a, b| a + *b) / poly.len() as f32
        };
        Self {
            spec,
            poly,
            gates,
            center,
        }
    }
}

/// Startup: prepara `VillageInfo` dai piazzamenti (serve prima del terreno).
pub fn init_village(mut commands: Commands, dbg: Res<DebugPlace>) {
    let info = VillageInfo::build(&dbg.items);
    info!(
        "[VILLAGGIO] {}: {} vertici di muro, {} cancelli, {} strade",
        info.spec.name,
        info.poly.len(),
        info.gates.len(),
        info.spec.roads.len()
    );
    commands.insert_resource(info);
}

/// Marker delle entità del muro e dei cancelli (rigenerati insieme).
#[derive(Component)]
pub struct VillageWall;

/// Maniglia di un cancello (stato aperto/chiuso).
#[derive(Component)]
pub struct GateHandle;

fn tint(c: Col, k: f32) -> Col {
    [c[0] * k, c[1] * k, c[2] * k, c[3]]
}

/// Sistema: (ri)costruisce mura, torrette e cancelli quando cambiano i piazzamenti.
#[allow(clippy::too_many_arguments)]
pub fn sync_village(
    mut commands: Commands,
    dbg: Res<DebugPlace>,
    field: Option<Res<TerrainField>>,
    clock: Res<GameClock>,
    mut info: ResMut<VillageInfo>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    old: Query<Entity, With<VillageWall>>,
    mut built: Local<(Option<u32>, usize)>,
) {
    let Some(field) = field else {
        return;
    };
    let stale = built.0 != Some(dbg.revision());
    let wiped = old.is_empty() && built.1 > 0;
    if !stale && !wiped {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    *info = VillageInfo::build(&dbg.items);
    built.0 = Some(dbg.revision());
    built.1 = 0;
    let n = info.poly.len();
    if n < 3 {
        return;
    }
    let ground = |p: Vec2| field.height_at(p.x, p.y);
    let mut stone = B::default();
    let mut roofs = B::default();
    let mut colliders: Vec<(Vec3, Vec3, f32)> = Vec::new();
    let mut seed = 7u32;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        0.92 + (seed % 1000) as f32 / 1000.0 * 0.16
    };

    // --- Tratti di muro (a pezzi da ~2 m che seguono il terreno), con i vani dei cancelli ---
    for i in 0..n {
        let (a, b) = (info.poly[i], info.poly[(i + 1) % n]);
        let len = a.distance(b);
        let t = (b - a) / len;
        let gaps: Vec<(f32, f32)> = info
            .gates
            .iter()
            .filter(|g| g.edge == i)
            .map(|g| {
                (
                    g.along - GATE_HALF - 2.0 * PIER_HALF,
                    g.along + GATE_HALF + 2.0 * PIER_HALF,
                )
            })
            .collect();
        // Intervalli pieni.
        let mut cuts = [0.0f32];
        let mut spans: Vec<(f32, f32)> = Vec::new();
        let mut gaps_sorted = gaps.clone();
        gaps_sorted.sort_by(|x, y| x.0.total_cmp(&y.0));
        for (g0, g1) in gaps_sorted {
            spans.push((cuts[0], g0.max(0.0)));
            cuts[0] = g1.min(len);
        }
        spans.push((cuts[0], len));
        for (s0, s1) in spans {
            if s1 - s0 < 0.05 {
                continue;
            }
            let k = ((s1 - s0) / 2.2).ceil().max(1.0) as usize;
            for j in 0..k {
                let (u0, u1) = (
                    s0 + (s1 - s0) * j as f32 / k as f32,
                    s0 + (s1 - s0) * (j + 1) as f32 / k as f32,
                );
                let (pa, pb) = (a + t * u0, a + t * u1);
                let pm = (pa + pb) / 2.0;
                let hs = [ground(pa), ground(pm), ground(pb)];
                let lo = hs.iter().cloned().fold(f32::MAX, f32::min) - 1.2;
                let hi = hs.iter().cloned().fold(f32::MIN, f32::max) + WALL_H;
                let half = (u1 - u0) / 2.0 + 0.02;
                stone.obox(pm, lo, hi, half, WALL_T / 2.0, t, tint(STONE, rnd()));
                stone.obox(
                    pm,
                    hi - 0.05,
                    hi + 0.25,
                    half,
                    WALL_T / 2.0 + 0.15,
                    t,
                    tint(STONE_DARK, rnd()),
                );
                let yaw = (-t.y).atan2(t.x);
                colliders.push((
                    Vec3::new(pm.x, (lo + hi) / 2.0, pm.y),
                    Vec3::new(half * 2.0, hi - lo, WALL_T),
                    yaw,
                ));
            }
        }
    }

    // --- Torrette ai vertici (non vicino ai cancelli, che hanno i loro piloni) ---
    for i in 0..n {
        let v = info.poly[i];
        if info.gates.iter().any(|g| g.pos.distance(v) < 6.8) {
            continue;
        }
        let g0 = ground(v);
        stone.obox(v, g0 - 1.4, g0 + 4.9, 1.4, 1.4, Vec2::X, tint(STONE, rnd()));
        stone.obox(
            v,
            g0 + 4.7,
            g0 + 5.0,
            1.65,
            1.65,
            Vec2::X,
            tint(STONE_DARK, rnd()),
        );
        roofs.pyramid(Vec3::new(v.x, g0 + 5.0, v.y), 1.75, 2.6, ROOFS[0]);
        colliders.push((Vec3::new(v.x, g0 + 1.8, v.y), Vec3::new(2.8, 6.5, 2.8), 0.0));
    }

    // --- Cancelli: piloni con torrette, architrave, battenti e maniglia ---
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.22, 0.12),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mut gate_entities = Vec::new();
    for gate in &info.gates {
        let g = gate.pos;
        let gy = ground(g);
        let t = gate.tangent;
        let yaw = (-t.y).atan2(t.x);
        let frame = Quat::from_rotation_y(yaw);
        for s in [-1.0f32, 1.0] {
            let c = g + t * (s * (GATE_HALF + PIER_HALF));
            let cy = ground(c);
            stone.obox(
                c,
                cy.min(gy) - 1.4,
                gy + 4.4,
                PIER_HALF + 0.05,
                0.85,
                t,
                tint(STONE, rnd()),
            );
            stone.obox(
                c,
                gy + 4.3,
                gy + 4.6,
                PIER_HALF + 0.2,
                1.0,
                t,
                tint(STONE_DARK, rnd()),
            );
            roofs.pyramid(Vec3::new(c.x, gy + 4.6, c.y), 0.95, 1.7, ROOFS[0]);
            colliders.push((
                Vec3::new(c.x, gy + 1.5, c.y),
                Vec3::new(2.0 * PIER_HALF + 0.1, 6.5, 1.7),
                yaw,
            ));
        }
        // Architrave sopra il vano.
        stone.obox(
            g,
            gy + 3.4,
            gy + 4.4,
            GATE_HALF + 0.1,
            0.8,
            t,
            tint(STONE, rnd()),
        );
        stone.obox(
            g,
            gy + 4.3,
            gy + 4.6,
            GATE_HALF + 0.3,
            0.95,
            t,
            tint(STONE_DARK, rnd()),
        );
        colliders.push((
            Vec3::new(g.x, gy + 3.9, g.y),
            Vec3::new(2.0 * GATE_HALF + 0.2, 1.2, 1.6),
            yaw,
        ));
        // Battenti: due ante grandi e una porticina centrale, ognuna con la sua maniglia (stato).
        let open_now = (OPEN_HOUR..CLOSE_HOUR).contains(&clock.hour);
        let inward3 = Vec3::new(gate.inward.x, 0.0, gate.inward.y);
        let frame_z = frame * Vec3::Z;
        let sign_in = if frame_z.dot(inward3) > 0.0 {
            -1.0
        } else {
            1.0
        };
        // (cardine lungo la tangente, larghezza, verso in cui si estende, raggio di mira)
        let leaves = [
            (-GATE_HALF, 1.7f32, 1.0f32, 0.9f32),
            (GATE_HALF, 1.7, -1.0, 0.9),
            (-0.5, 1.0, 1.0, 0.5),
        ];
        for (hx, w, dir, aim) in leaves {
            let hinge_pos = g + t * hx;
            let hy = ground(hinge_pos).min(gy);
            let mid = hinge_pos + t * (dir * w / 2.0);
            let handle = commands
                .spawn((
                    VillageWall,
                    SceneNode,
                    GateHandle,
                    PropKind::Gate,
                    PropState { on: open_now },
                    Interactable {
                        prompt: "[E] Chiudi cancello".to_string(),
                        range: 5.0,
                        aim: Some(aim),
                        use_from: None,
                    },
                    Transform::from_xyz(mid.x, gy + 1.6, mid.y),
                ))
                .id();
            gate_entities.push(handle);
            let angle = if dir > 0.0 {
                sign_in * 1.55
            } else {
                -sign_in * 1.55
            };
            let leaf = Cuboid::new(w, 3.1, 0.14);
            commands
                .spawn((
                    VillageWall,
                    SceneNode,
                    RigidBody::Static,
                    Hinge {
                        state: handle,
                        open_angle: angle,
                        angle: 0.0,
                        base: frame,
                    },
                    Transform::from_xyz(hinge_pos.x, hy, hinge_pos.y).with_rotation(frame),
                    Visibility::default(),
                ))
                .with_children(|d| {
                    d.spawn((
                        Mesh3d(meshes.add(leaf)),
                        MeshMaterial3d(wood.clone()),
                        Transform::from_xyz(dir * w / 2.0, 1.55, 0.0),
                    ));
                    d.spawn((
                        DoorCollider,
                        Collider::cuboid(w, 3.1, 0.3),
                        BlocksCamera,
                        Transform::from_xyz(dir * w / 2.0, 1.55, 0.0),
                    ));
                });
        }
    }

    // --- Entità del muro: una mesh di pietra, una di tetti, collider a pezzi ---
    let mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.95,
        ..default()
    });
    commands
        .spawn((
            VillageWall,
            SceneNode,
            RigidBody::Static,
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|c| {
            c.spawn((
                Mesh3d(meshes.add(stone.finish())),
                MeshMaterial3d(mat.clone()),
            ));
            c.spawn((
                Mesh3d(meshes.add(roofs.finish())),
                MeshMaterial3d(mat.clone()),
            ));
            for (center, size, yaw) in colliders {
                c.spawn((
                    Collider::cuboid(size.x, size.y, size.z),
                    Transform::from_translation(center).with_rotation(Quat::from_rotation_y(yaw)),
                    BlocksCamera,
                ));
            }
        });
    built.1 = 1;
    info!(
        "[VILLAGGIO] mura costruite: {} vertici, {} cancelli",
        n,
        gate_entities.len()
    );
}

/// Sistema: i cancelli si aprono all'alba e si chiudono la sera (il varco pedonale resta libero).
pub fn gate_schedule(
    clock: Res<GameClock>,
    mut gates: Query<&mut PropState, With<GateHandle>>,
    mut last: Local<Option<bool>>,
) {
    let open = (OPEN_HOUR..CLOSE_HOUR).contains(&clock.hour);
    if *last != Some(open) {
        *last = Some(open);
        for mut s in &mut gates {
            s.on = open;
        }
    }
}

/// Sistema: entrando nel villaggio compare il nome in alto (come il titolo delle quest).
pub fn village_banner(
    info: Res<VillageInfo>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    mut quest: ResMut<crate::quests::denaro_rubato::QuestState>,
    mut was_inside: Local<Option<bool>>,
) {
    let (Ok(p), true) = (players.single(), info.poly.len() >= 3) else {
        return;
    };
    let pos = Vec2::new(p.translation().x, p.translation().z);
    let inside = in_polygon(&info.poly, pos);
    if *was_inside == Some(false) && inside {
        quest.title = Some((info.spec.name.clone(), 4.0));
    }
    *was_inside = Some(inside);
}

pub fn plugin(app: &mut App) {
    app.init_resource::<VillageInfo>()
        .add_systems(Startup, init_village)
        .add_systems(Update, (sync_village, gate_schedule, village_banner));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Vec2> {
        vec![
            Vec2::new(-10.0, -10.0),
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
        ]
    }

    #[test]
    fn polygon_queries() {
        let p = square();
        assert!(in_polygon(&p, Vec2::ZERO));
        assert!(!in_polygon(&p, Vec2::new(11.0, 0.0)));
        assert!(signed_distance(&p, Vec2::ZERO) < -9.9);
        assert!((signed_distance(&p, Vec2::new(13.0, 0.0)) - 3.0).abs() < 1e-3);
    }

    #[test]
    fn gate_projects_on_nearest_edge() {
        let g = project_gates(&square(), &[("N".into(), Vec2::new(2.0, -7.0))]);
        assert_eq!(g[0].edge, 0);
        assert!((g[0].pos - Vec2::new(2.0, -10.0)).length() < 1e-3);
        // l'interno è verso +z
        assert!(g[0].inward.y > 0.9);
    }

    #[test]
    fn spec_parses() {
        let s = parse_spec(SPEC);
        assert_eq!(s.name, "Fontevivo");
        assert_eq!(s.gates.len(), 2);
        assert!(s.roads.len() >= 2);
    }
}
