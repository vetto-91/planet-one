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

/// Semi-larghezza del vano del cancello e dei piloni.
const GATE_HALF: f32 = 2.2;
const PIER_HALF: f32 = 0.55;
/// Bottoni del cancello: distanza dalla linea del muro verso il villaggio e verso fuori (sulle torri).
const BUTTON_IN: f32 = 0.95;
const BUTTON_OUT: f32 = 3.45;

/// Una strada: larghezza (m) e polilinea.
#[derive(Clone, Debug)]
pub struct Road {
    pub width: f32,
    pub pts: Vec<Vec2>,
    /// Sentiero di terra battuta invece che selciato.
    pub dirt: bool,
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
                        let dirt = head.split_whitespace().next() == Some("sentiero");
                        s.roads.push(Road { width, pts, dirt });
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
    /// Percorso chiuso delle ronde sulle mura (x, quota del piano, z), riempito da `sync_village`.
    pub patrol: Vec<Vec3>,
    /// Porta della casa delle guardie e direzione dello sguardo verso l'esterno.
    pub barracks: Option<(Vec2, Vec2)>,
}

impl VillageInfo {
    /// Il punto cade nella fascia occupata da mura e torri (verso l'esterno, fino a ~5 m)?
    pub fn in_wall_band(&self, p: Vec2) -> bool {
        self.poly.len() >= 3 && !in_polygon(&self.poly, p) && {
            let d = (0..self.poly.len())
                .map(|i| dist_to_segment(p, self.poly[i], self.poly[(i + 1) % self.poly.len()]))
                .fold(f32::MAX, f32::min);
            d < 5.2
        }
    }

    pub fn build(items: &[Placement]) -> Self {
        let spec = load_spec();
        let poly = wall_polygon(items);
        let gates = project_gates(&poly, &spec.gates);
        let center = if poly.is_empty() {
            Vec2::ZERO
        } else {
            poly.iter().fold(Vec2::ZERO, |a, b| a + *b) / poly.len() as f32
        };
        // Casa delle guardie: la porta è sul fronte (+Z locale), 0,8 m oltre la facciata.
        let barracks = items.iter().find(|p| p.kind == "caserma").and_then(|pl| {
            let p = *pl.points.first()?;
            let yaw = pl.yaw(0);
            let dir = Vec2::new(yaw.sin(), yaw.cos());
            let half = crate::buildings::model("caserma", 1).half;
            Some((p + dir * (half.y + 0.8), dir))
        });
        Self {
            spec,
            poly,
            gates,
            center,
            patrol: Vec::new(),
            barracks,
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

/// Maniglia (stato aperto/chiuso) di un cancello.
#[derive(Component)]
pub struct GateHandle;

/// Bottone su un pilone: commuta lo stato del cancello `0`.
#[derive(Component)]
pub struct GateButton(pub Entity);

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
    pbr: Res<crate::pbr::PbrMaterials>,
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

    // --- Anello delle mura: quote del camminamento lungo tutto il perimetro ---
    let lp = Perimeter::new(&info.poly);
    let samples = (lp.total.ceil() as usize).max(4);
    let ground_ring: Vec<f32> = (0..samples)
        .map(|i| {
            let s = i as f32 * lp.total / samples as f32;
            let (p, _, out) = lp.at(s);
            ground(p + out * WALL_CENTER_U)
        })
        .collect();
    // Cancelli: ascissa, quota del ponte e zona occupata da vano e torri.
    let gates_s: Vec<(f32, f32)> = info
        .gates
        .iter()
        .map(|g| (lp.cum[g.edge] + g.along, ground(g.pos) + WALK_ABOVE_GROUND))
        .collect();
    let pins: Vec<(usize, usize, f32)> = gates_s
        .iter()
        .map(|(s, y)| {
            let from =
                ((s - GATE_ZONE - 1.0).rem_euclid(lp.total) * samples as f32 / lp.total) as usize;
            let len = ((2.0 * (GATE_ZONE + 1.0)) * samples as f32 / lp.total).ceil() as usize;
            (from, len, *y)
        })
        .collect();
    let heights = walk_heights(&ground_ring, &pins);
    let height_at = |s: f32| {
        let i = (s.rem_euclid(lp.total) * samples as f32 / lp.total) as usize % samples;
        heights[i]
    };

    // Torri d'angolo: sui vertici lontani dai cancelli.
    let mut towers: Vec<TowerPlan> = Vec::new();
    let mut blocked: Vec<(f32, f32)> = gates_s
        .iter()
        .map(|(s, _)| (s - GATE_ZONE, s + GATE_ZONE))
        .collect();
    for v in 0..n {
        if info
            .gates
            .iter()
            .any(|g| g.pos.distance(info.poly[v]) < TOWER_CLEAR)
        {
            continue;
        }
        let s = lp.cum[v];
        let prev_out = lp.at((s - 0.5).rem_euclid(lp.total)).2;
        let next_out = lp.at((s + 0.5).rem_euclid(lp.total)).2;
        let out = (prev_out + next_out).normalize_or_zero();
        let t = Vec2::new(-out.y, out.x);
        towers.push(TowerPlan {
            center: info.poly[v] + out * WALL_CENTER_U,
            t,
            n: out,
            half_t: TOWER_HALF,
            half_n: TOWER_HALF,
            floor: height_at(s),
            s,
            neighbours: vec![
                lp.deck_point(s - TOWER_DOOR_AIM),
                lp.deck_point(s + TOWER_DOOR_AIM),
            ],
        });
        blocked.push((s - TOWER_ZONE, s + TOWER_ZONE));
    }
    // Torri dei cancelli, una per lato del vano, con il ponte in mezzo.
    for (gate, (gs, floor)) in info.gates.iter().zip(&gates_s) {
        let out = -gate.inward;
        for side in [-1.0f32, 1.0] {
            // `gate.tangent` e l'ascissa crescente coincidono: il lato verso il muro è `side`.
            towers.push(TowerPlan {
                center: gate.pos
                    + gate.tangent * (side * (GATE_HALF + GATE_TOWER_HALF_T))
                    + out * WALL_CENTER_U,
                t: gate.tangent,
                n: out,
                half_t: GATE_TOWER_HALF_T,
                half_n: TOWER_HALF,
                floor: *floor,
                s: gs + side * (GATE_HALF + GATE_TOWER_HALF_T),
                neighbours: vec![
                    lp.deck_point(gs + side * (GATE_ZONE + 0.5)),
                    gate.pos + out * WALL_CENTER_U,
                ],
            });
        }
    }

    // Tratti di muro pieno: l'anello senza cancelli e torri.
    let free = free_intervals(lp.total, &blocked);
    // Scale: una dopo ogni cancello, più il tratto più lungo a est e a ovest.
    let mut stair_starts: Vec<f32> = gates_s
        .iter()
        .filter_map(|(s, _)| {
            let start = s + GATE_ZONE;
            free.iter()
                .find(|(a, b)| {
                    (a - start.rem_euclid(lp.total)).abs() < 0.1 && b - a >= MIN_STAIR_RUN
                })
                .map(|(a, _)| *a)
        })
        .collect();
    for sign in [1.0f32, -1.0] {
        let best = free
            .iter()
            .filter(|(a, b)| {
                b - a >= MIN_STAIR_RUN
                    && !stair_starts.iter().any(|s| (s - a).abs() < 0.1)
                    && (lp.at((a + b) / 2.0).0.x - info.center.x) * sign > 8.0
            })
            .max_by(|x, y| (x.1 - x.0).total_cmp(&(y.1 - y.0)));
        if let Some((a, _)) = best {
            stair_starts.push(*a);
        }
    }
    let mut stair_zones: Vec<(f32, f32)> = Vec::new();

    let mut patrol: Vec<(f32, Vec3)> = Vec::new();
    let mut walk_colliders: Vec<(Vec3, Vec3, f32)> = Vec::new();

    // Scale nella corsia interna (dentro lo spessore del muro).
    for start in &stair_starts {
        // Il primo gradino non deve cadere dentro la base di una torre: si parte oltre.
        let mut a = start + 0.1;
        let in_tower = |s: f32| {
            [-0.45f32, 0.3, 1.05].iter().any(|u| {
                let q = lp.at(s).0 + lp.at(s).2 * *u;
                towers.iter().any(|w| {
                    (q - w.center).dot(w.t).abs() <= w.half_t + 0.3
                        && (q - w.center).dot(w.n).abs() <= w.half_n + 0.3
                })
            })
        };
        let mut guard = 0;
        while in_tower(a) && guard < 40 {
            a += 0.1;
            guard += 1;
        }
        let inner = |s: f32| {
            let (p, t, out) = lp.at(s);
            (p + out * INNER_LANE_U, t)
        };
        let g0 = ground(inner(a).0);
        let top_guess = height_at(a + 6.0);
        let steps = ((top_guess - g0) / STEP_RISE).ceil().max(1.0) as usize;
        let len = steps as f32 * STEP_RUN;
        let rise = (height_at(a + len) - g0) / steps as f32;
        for i in 0..steps {
            let (c, t) = inner(a + (i as f32 + 0.5) * STEP_RUN);
            let top = g0 + (i as f32 + 1.0) * rise;
            let lo = ground(c) - 1.2;
            stone.obox(
                c,
                lo,
                top,
                STEP_RUN / 2.0 + 0.01,
                LANE_HALF_INNER,
                t,
                tint(STONE_DARK, rnd()),
            );
            walk_colliders.push((
                Vec3::new(c.x, (lo + top) / 2.0, c.y),
                Vec3::new(STEP_RUN + 0.02, top - lo, LANE_HALF_INNER * 2.0),
                (-t.y).atan2(t.x),
            ));
        }
        info!(
            "[VILLAGGIO] scala da ({:.1},{:.1}) a ({:.1},{:.1}), {steps} gradini, sale a {:.1}",
            inner(a).0.x,
            inner(a).0.y,
            inner(a + len).0.x,
            inner(a + len).0.y,
            g0 + steps as f32 * rise
        );
        stair_zones.push((a, a + len));
    }

    // --- Tratti di muro: corpo pieno, corsie, parapetto e cordolo ---
    // Le estremità vicino alle torri si prolungano fino a entrare nella torre, così
    // non restano cunei vuoti fra la testa del muro e la faccia della torre.
    let tower_boxes: Vec<(Vec2, Vec2, Vec2, f32, f32)> = towers
        .iter()
        .map(|w| (w.center, w.t, w.n, w.half_t, w.half_n))
        .collect();
    let reach = |origin: Vec2, t: Vec2, out: Vec2, dir: f32, u: f32| -> f32 {
        for k in 0..=60 {
            let d = k as f32 * 0.05;
            let q = origin + t * (dir * d) + out * u;
            if tower_boxes.iter().any(|(c, tt, nn, ht, hn)| {
                (q - *c).dot(*tt).abs() <= ht - 0.05 && (q - *c).dot(*nn).abs() <= hn - 0.05
            }) {
                return d;
            }
        }
        0.0
    };
    for (fa, fb) in &free {
        for (sa, sb) in lp.pieces(*fa, *fb) {
            let sm = (sa + sb) / 2.0;
            let (p, t, out) = lp.at(sm);
            let first = (sa - fa).abs() < 1e-3;
            let last = (sb - fb).abs() < 1e-3;
            let p_a = p + t * (sa - sm);
            let p_b = p + t * (sb - sm);
            // Prolungamento (a monte, a valle) per una linea a distanza `u` dal muro.
            let ext = |u: f32| -> (f32, f32) {
                (
                    if first {
                        reach(p_a, t, out, -1.0, u)
                    } else {
                        0.0
                    },
                    if last {
                        reach(p_b, t, out, 1.0, u)
                    } else {
                        0.0
                    },
                )
            };
            let h = height_at(sm);
            let half = (sb - sa) / 2.0 + 0.02;
            let hs = [lp.at(sa).0, p, lp.at(sb - 0.01).0].map(|q| ground(q + out * WALL_CENTER_U));
            let lo = hs.iter().cloned().fold(f32::MAX, f32::min) - 1.2;
            let yaw = (-t.y).atan2(t.x);
            let on_stair = stair_zones
                .iter()
                .any(|(a, b)| sb > *a - 0.05 && sa < *b + 0.05);
            let (body_u, body_half) = if on_stair {
                (OUTER_BODY_U, OUTER_BODY_HALF)
            } else {
                (WALL_CENTER_U, WALL_HALF)
            };
            let (e0, e1) = {
                let (a0, a1) = ext(body_u - body_half);
                let (b0, b1) = ext(body_u + body_half);
                (a0.max(b0), a1.max(b1))
            };
            let body_half_len = half + (e0 + e1) / 2.0;
            let body = p + out * body_u + t * ((e1 - e0) / 2.0);
            stone.obox(body, lo, h, body_half_len, body_half, t, tint(STONE, rnd()));
            walk_colliders.push((
                Vec3::new(body.x, (lo + h) / 2.0, body.y),
                Vec3::new(body_half_len * 2.0, h - lo, body_half * 2.0),
                yaw,
            ));
            // Parapetto esterno con merli.
            let (pe0, pe1) = ext(PARAPET_U);
            let par_half = half + (pe0 + pe1) / 2.0;
            let par = p + out * PARAPET_U + t * ((pe1 - pe0) / 2.0);
            stone.obox(
                par,
                h - 0.05,
                h + PARAPET_H,
                par_half,
                PARAPET_HALF,
                t,
                tint(STONE, rnd()),
            );
            colliders.push((
                Vec3::new(par.x, h + PARAPET_H / 2.0, par.y),
                Vec3::new(par_half * 2.0, PARAPET_H, PARAPET_HALF * 2.0),
                yaw,
            ));
            let merlon = p + t * (half * 0.5) + out * PARAPET_U;
            stone.obox(
                merlon,
                h + PARAPET_H - 0.05,
                h + PARAPET_H + 0.4,
                half * 0.28,
                PARAPET_HALF + 0.02,
                t,
                tint(STONE_DARK, rnd()),
            );
            // Cordolo verso il villaggio (non sopra le scale).
            if !on_stair {
                let (ke0, ke1) = ext(KERB_U);
                let kerb_half = half + (ke0 + ke1) / 2.0;
                let kerb = p + out * KERB_U + t * ((ke1 - ke0) / 2.0);
                stone.obox(
                    kerb,
                    h - 0.05,
                    h + KERB_H,
                    kerb_half,
                    KERB_HALF,
                    t,
                    tint(STONE_DARK, rnd()),
                );
                colliders.push((
                    Vec3::new(kerb.x, h + KERB_H / 2.0, kerb.y),
                    Vec3::new(kerb_half * 2.0, KERB_H, KERB_HALF * 2.0),
                    yaw,
                ));
            }
            patrol.push((sm, Vec3::new(p.x + out.x * LANE_U, h, p.y + out.y * LANE_U)));
        }
    }
    // In cima alle scale il tratto residuo della corsia interna viene richiuso.
    for (a, b) in &stair_zones {
        let Some((_, piece_end)) = free
            .iter()
            .flat_map(|(fa, fb)| lp.pieces(*fa, *fb))
            .find(|(sa, sb)| *b >= *sa && *b < *sb)
        else {
            continue;
        };
        let gap = piece_end - b;
        if gap > 0.05 {
            let sm = b + gap / 2.0;
            let (p, t, out) = lp.at(sm);
            let c = p + out * INNER_LANE_U;
            let h = height_at(sm);
            let lo = ground(c) - 1.2;
            stone.obox(
                c,
                lo,
                h,
                gap / 2.0 + 0.02,
                LANE_HALF_INNER,
                t,
                tint(STONE, rnd()),
            );
            walk_colliders.push((
                Vec3::new(c.x, (lo + h) / 2.0, c.y),
                Vec3::new(gap + 0.04, h - lo, LANE_HALF_INNER * 2.0),
                (-t.y).atan2(t.x),
            ));
        }
        let _ = a;
    }

    // --- Torri cave, attraversabili al livello del camminamento ---
    for tower in &towers {
        let c = tower.center;
        let (t, nrm) = (tower.t, tower.n);
        let yaw = (-t.y).atan2(t.x);
        let floor = tower.floor;
        let lo = [
            c + t * tower.half_t + nrm * tower.half_n,
            c + t * tower.half_t - nrm * tower.half_n,
            c - t * tower.half_t + nrm * tower.half_n,
            c - t * tower.half_t - nrm * tower.half_n,
        ]
        .map(&ground)
        .into_iter()
        .fold(f32::MAX, f32::min)
            - 1.4;
        stone.obox(
            c,
            lo,
            floor,
            tower.half_t,
            tower.half_n,
            t,
            tint(STONE, rnd()),
        );
        walk_colliders.push((
            Vec3::new(c.x, (lo + floor) / 2.0, c.y),
            Vec3::new(tower.half_t * 2.0, floor - lo, tower.half_n * 2.0),
            yaw,
        ));
        let top = floor + TOWER_ROOM_H;
        for face in tower.faces() {
            for (u0, u1) in &face.solid {
                let mid = (u0 + u1) / 2.0;
                let half = (u1 - u0) / 2.0;
                let centre = face.origin + face.along * mid;
                stone.obox(
                    centre,
                    floor,
                    top,
                    half,
                    TOWER_WALL / 2.0,
                    face.along,
                    tint(STONE, rnd()),
                );
                colliders.push((
                    Vec3::new(centre.x, (floor + top) / 2.0, centre.y),
                    Vec3::new(half * 2.0, top - floor, TOWER_WALL),
                    (-face.along.y).atan2(face.along.x),
                ));
            }
            for door in &face.doors {
                let centre = face.origin + face.along * *door;
                stone.obox(
                    centre,
                    floor + TOWER_DOOR_H,
                    top,
                    TOWER_DOOR_HALF,
                    TOWER_WALL / 2.0,
                    face.along,
                    tint(STONE_DARK, rnd()),
                );
            }
        }
        stone.obox(
            c,
            top,
            top + 0.3,
            tower.half_t + 0.15,
            tower.half_n + 0.15,
            t,
            tint(STONE_DARK, rnd()),
        );
        colliders.push((
            Vec3::new(c.x, top + 0.15, c.y),
            Vec3::new(tower.half_t * 2.0 + 0.3, 0.3, tower.half_n * 2.0 + 0.3),
            yaw,
        ));
        roofs.pyramid_oriented(
            Vec3::new(c.x, top + 0.3, c.y),
            t,
            nrm,
            tower.half_t + 0.25,
            tower.half_n + 0.25,
            2.6,
            ROOFS[0],
        );
        patrol.push((tower.s, Vec3::new(c.x, floor, c.y)));
    }

    // Ponte sopra ogni cancello, tra le due torri.
    for (gate, (gs, floor)) in info.gates.iter().zip(&gates_s) {
        let t = gate.tangent;
        let out = -gate.inward;
        let yaw = (-t.y).atan2(t.x);
        let centre = gate.pos + out * WALL_CENTER_U;
        stone.obox(
            centre,
            floor - BRIDGE_THICK,
            *floor,
            GATE_HALF + 0.02,
            WALL_HALF,
            t,
            tint(STONE, rnd()),
        );
        walk_colliders.push((
            Vec3::new(centre.x, floor - BRIDGE_THICK / 2.0, centre.y),
            Vec3::new(2.0 * GATE_HALF + 0.04, BRIDGE_THICK, WALL_HALF * 2.0),
            yaw,
        ));
        let par = gate.pos + out * PARAPET_U;
        stone.obox(
            par,
            floor - 0.05,
            floor + PARAPET_H,
            GATE_HALF,
            PARAPET_HALF,
            t,
            tint(STONE, rnd()),
        );
        colliders.push((
            Vec3::new(par.x, floor + PARAPET_H / 2.0, par.y),
            Vec3::new(2.0 * GATE_HALF, PARAPET_H, PARAPET_HALF * 2.0),
            yaw,
        ));
        let kerb = gate.pos + out * KERB_U;
        stone.obox(
            kerb,
            floor - 0.05,
            floor + KERB_H,
            GATE_HALF,
            KERB_HALF,
            t,
            tint(STONE_DARK, rnd()),
        );
        colliders.push((
            Vec3::new(kerb.x, floor + KERB_H / 2.0, kerb.y),
            Vec3::new(2.0 * GATE_HALF, KERB_H, KERB_HALF * 2.0),
            yaw,
        ));
        patrol.push((
            *gs,
            Vec3::new(
                gate.pos.x + out.x * LANE_U,
                *floor,
                gate.pos.y + out.y * LANE_U,
            ),
        ));
    }
    patrol.sort_by(|a, b| a.0.total_cmp(&b.0));
    let patrol: Vec<Vec3> = patrol.into_iter().map(|(_, p)| p).collect();
    info!(
        "[VILLAGGIO] anello di ronda: {:.0} m, {} torri, {} scale, quote {:.2}..{:.2}",
        lp.total,
        towers.len(),
        stair_zones.len(),
        heights.iter().cloned().fold(f32::MAX, f32::min),
        heights.iter().cloned().fold(f32::MIN, f32::max),
    );

    // --- Cancelli: battenti e maniglia (vano e ponte stanno nelle torri) ---
    let wood = pbr.get(crate::pbr::Mat::Planks);
    let mut gate_entities = Vec::new();
    for gate in &info.gates {
        let g = gate.pos;
        let gy = ground(g);
        let t = gate.tangent;
        let yaw = (-t.y).atan2(t.x);
        let frame = Quat::from_rotation_y(yaw);
        // Battenti: due ante piene (2,2 m ciascuna) che si chiudono al centro. Lo stato sta su
        // una maniglia invisibile; i bottoni sui piloni (dentro e fuori) lo commutano.
        let open_now = clock.awake();
        let inward3 = Vec3::new(gate.inward.x, 0.0, gate.inward.y);
        let frame_z = frame * Vec3::Z;
        let sign_in = if frame_z.dot(inward3) > 0.0 {
            -1.0
        } else {
            1.0
        };
        let state = commands
            .spawn((
                VillageWall,
                SceneNode,
                GateHandle,
                PropKind::Gate,
                crate::hut::PropId(format!("cancello:{}", gate.name)),
                PropState { on: open_now },
                Transform::from_xyz(g.x, gy + 1.6, g.y),
            ))
            .id();
        gate_entities.push(state);
        for s in [-1.0f32, 1.0] {
            // s = -1: anta sinistra (cardine a g - t*GATE_HALF, si estende verso +t).
            let hinge_pos = g + t * (s * GATE_HALF);
            let hy = ground(hinge_pos).min(gy);
            let dir = -s;
            let angle = if s < 0.0 {
                sign_in * 1.55
            } else {
                -sign_in * 1.55
            };
            let leaf = crate::pbr::with_tangents(Cuboid::new(GATE_HALF, 3.1, 0.14));
            commands
                .spawn((
                    VillageWall,
                    SceneNode,
                    RigidBody::Static,
                    Hinge {
                        state,
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
                        Transform::from_xyz(dir * GATE_HALF / 2.0, 1.55, 0.0),
                    ));
                    d.spawn((
                        DoorCollider,
                        Collider::cuboid(GATE_HALF, 3.1, 0.3),
                        BlocksCamera,
                        Transform::from_xyz(dir * GATE_HALF / 2.0, 1.55, 0.0),
                    ));
                });
        }
        // Bottoni: sulle facce dei piloni verso il villaggio e verso l'esterno (uno per lato).
        let button = meshes.add(Cuboid::new(0.2, 0.34, 0.12));
        for pier in [-1.0f32, 1.0] {
            for (side, normal) in [(1.0f32, inward3), (-1.0, -inward3)] {
                let along_wall = gate.inward * if side > 0.0 { BUTTON_IN } else { -BUTTON_OUT };
                let c = g + t * (pier * (GATE_HALF + PIER_HALF)) + along_wall;
                let pos = Vec3::new(c.x, gy + 1.5, c.y);
                commands.spawn((
                    VillageWall,
                    SceneNode,
                    GateButton(state),
                    PropKind::Gate,
                    PropState { on: open_now },
                    Interactable {
                        prompt: "[E] Apri/chiudi cancello".to_string(),
                        range: 4.0,
                        aim: Some(0.35),
                        // `use_from` è locale all'entità (che ha la rotazione del muro).
                        use_from: Some(frame.inverse() * normal),
                    },
                    Mesh3d(button.clone()),
                    MeshMaterial3d(wood.clone()),
                    Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)),
                ));
            }
        }
    }

    // --- Entità del muro: una mesh di pietra, una di tetti, collider a pezzi ---
    let snow_mat = crate::seasons::snow_material(&mut materials);
    let snows = [stone.snow_mesh(), roofs.snow_mesh()];
    commands
        .spawn((
            VillageWall,
            SceneNode,
            RigidBody::Static,
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|c| {
            for (m, mesh) in stone.finish_parts() {
                c.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(pbr.get(m))));
            }
            for snow in snows {
                c.spawn((
                    crate::seasons::SeasonSnowRoof,
                    Mesh3d(meshes.add(snow)),
                    MeshMaterial3d(snow_mat.clone()),
                    bevy::light::NotShadowCaster,
                    Visibility::Hidden,
                ));
            }
            for (m, mesh) in roofs.finish_parts() {
                c.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(pbr.get(m))));
            }
            for (center, size, yaw) in colliders {
                c.spawn((
                    Collider::cuboid(size.x, size.y, size.z),
                    Transform::from_translation(center).with_rotation(Quat::from_rotation_y(yaw)),
                    BlocksCamera,
                ));
            }
            // Camminamenti e gradini: sono "terreno" per il controllo di appoggio del giocatore.
            for (center, size, yaw) in walk_colliders {
                c.spawn((
                    Collider::cuboid(size.x, size.y, size.z),
                    Transform::from_translation(center).with_rotation(Quat::from_rotation_y(yaw)),
                    BlocksCamera,
                    crate::scene::Ground,
                ));
            }
        });
    info.patrol = patrol;
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
    let open = clock.awake();
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
    // Sul camminamento non si entra né si esce dal villaggio.
    if info.in_wall_band(pos) {
        return;
    }
    let inside = in_polygon(&info.poly, pos);
    if *was_inside == Some(false) && inside {
        quest.title = Some((info.spec.name.clone(), 4.0));
    }
    *was_inside = Some(inside);
}

// ---------------------------------------------------------------------------------------------
// Camminamenti sulle mura (ADR 0013)
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// Mura percorribili (ADR 0018). Coordinate laterali `u`: distanza dalla linea del poligono verso
// l'esterno. Il lato interno del muro resta dov'era (u = -0,45), lo spessore cresce verso fuori.
// ---------------------------------------------------------------------------------------------

/// Spessore totale e centro del muro.
const WALL_HALF: f32 = 1.7;
const WALL_CENTER_U: f32 = 1.25;
/// Corsia interna (dove stanno le scale) e parte esterna del corpo sulle scale.
const INNER_LANE_U: f32 = 0.3;
const LANE_HALF_INNER: f32 = 0.75;
const OUTER_BODY_U: f32 = 2.0;
const OUTER_BODY_HALF: f32 = 0.95;
/// Centro della corsia esterna, percorsa dalle ronde.
const LANE_U: f32 = 1.9;
/// Parapetto esterno e cordolo interno.
const PARAPET_U: f32 = 2.85;
const PARAPET_HALF: f32 = 0.1;
const PARAPET_H: f32 = 1.1;
const KERB_U: f32 = -0.35;
const KERB_HALF: f32 = 0.1;
const KERB_H: f32 = 0.5;
/// Piano del camminamento sopra il terreno lisciato.
const WALK_ABOVE_GROUND: f32 = 3.6;
/// Dislivello massimo per metro di camminamento (a tratti di ~2 m resta sotto il passo).
const MAX_WALK_SLOPE: f32 = 0.09;
/// Scale.
const STEP_RISE: f32 = 0.3;
const STEP_RUN: f32 = 0.38;
const MIN_STAIR_RUN: f32 = 8.0;
/// Torri.
const TOWER_CLEAR: f32 = 6.8;
const TOWER_HALF: f32 = 2.2;
/// Metà del tratto di anello occupato da una torre: il muro entra un po' sotto la torre, così non restano buchi.
const TOWER_ZONE: f32 = 1.5;
/// Distanza del punto verso cui si apre il varco di una torre.
const TOWER_DOOR_AIM: f32 = 3.0;
const GATE_TOWER_HALF_T: f32 = 1.4;
const GATE_ZONE: f32 = GATE_HALF + 2.0 * GATE_TOWER_HALF_T;
const TOWER_ROOM_H: f32 = 2.7;
const TOWER_WALL: f32 = 0.35;
const TOWER_DOOR_HALF: f32 = 1.5;
const TOWER_DOOR_H: f32 = 2.3;
/// Spessore del ponte sopra il vano del cancello.
const BRIDGE_THICK: f32 = 0.5;
/// Lunghezza massima dei tratti di muro.
const PIECE_LEN: f32 = 2.2;

/// Quote del camminamento su un anello di campioni a passo costante: terreno lisciato più l'altezza
/// del muro, fisse sopra i cancelli (`pins`: da, quanti, quota) e con pendenza limitata.
pub fn walk_heights(ground: &[f32], pins: &[(usize, usize, f32)]) -> Vec<f32> {
    let n = ground.len();
    let mut h = ground.to_vec();
    for _ in 0..3 {
        let src = h.clone();
        for (i, v) in h.iter_mut().enumerate() {
            *v = (-6..=6)
                .map(|k: i32| src[(i as i32 + k).rem_euclid(n as i32) as usize])
                .sum::<f32>()
                / 13.0;
        }
    }
    for v in &mut h {
        *v += WALK_ABOVE_GROUND;
    }
    let pin_at = |i: usize| {
        pins.iter()
            .find(|(from, len, _)| (i + n - from) % n < *len)
            .map(|p| p.2)
    };
    // Rilassamento: i punti fissati restano, gli altri si avvicinano ai vicini entro la pendenza.
    for _ in 0..(n * 2).max(32) {
        let mut changed = false;
        for i in 0..n {
            if let Some(y) = pin_at(i) {
                h[i] = y;
                continue;
            }
            let lo = h[(i + n - 1) % n].max(h[(i + 1) % n]) - MAX_WALK_SLOPE;
            let hi = h[(i + n - 1) % n].min(h[(i + 1) % n]) + MAX_WALK_SLOPE;
            let new = if lo <= hi {
                h[i].clamp(lo, hi)
            } else {
                (lo + hi) / 2.0
            };
            if (new - h[i]).abs() > 1e-4 {
                h[i] = new;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    h
}

/// Parti dell'anello rimaste libere togliendo le zone occupate (anche a cavallo dello zero).
pub fn free_intervals(total: f32, blocked: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut parts: Vec<(f32, f32)> = Vec::new();
    for &(a, b) in blocked {
        let a0 = a.rem_euclid(total);
        let len = b - a;
        if a0 + len > total {
            parts.push((a0, total));
            parts.push((0.0, a0 + len - total));
        } else {
            parts.push((a0, a0 + len));
        }
    }
    parts.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out = Vec::new();
    let mut cur = 0.0f32;
    for (a, b) in parts {
        if a - cur > 0.05 {
            out.push((cur, a));
        }
        cur = cur.max(b);
    }
    if total - cur > 0.05 {
        out.push((cur, total));
    }
    out
}

/// Il muro come linea chiusa, misurata lungo il perimetro.
pub struct Perimeter<'a> {
    poly: &'a [Vec2],
    /// Ascissa dell'inizio di ogni lato.
    pub cum: Vec<f32>,
    pub total: f32,
}

impl<'a> Perimeter<'a> {
    pub fn new(poly: &'a [Vec2]) -> Self {
        let mut cum = Vec::with_capacity(poly.len());
        let mut total = 0.0;
        for i in 0..poly.len() {
            cum.push(total);
            total += poly[i].distance(poly[(i + 1) % poly.len()]);
        }
        Self { poly, cum, total }
    }

    fn edge_of(&self, s: f32) -> usize {
        self.cum.partition_point(|c| *c <= s).saturating_sub(1)
    }

    /// (punto sul muro, tangente, normale verso l'esterno) all'ascissa `s`.
    pub fn at(&self, s: f32) -> (Vec2, Vec2, Vec2) {
        let s = s.rem_euclid(self.total);
        let e = self.edge_of(s);
        let (a, b) = (self.poly[e], self.poly[(e + 1) % self.poly.len()]);
        let t = (b - a).normalize_or_zero();
        let mut out = Vec2::new(-t.y, t.x);
        if in_polygon(self.poly, (a + b) / 2.0 + out * 0.5) {
            out = -out;
        }
        (a + t * (s - self.cum[e]), t, out)
    }

    /// Centro del camminamento all'ascissa `s` (dove si aprono i varchi delle torri).
    pub fn deck_point(&self, s: f32) -> Vec2 {
        let (p, _, out) = self.at(s);
        p + out * WALL_CENTER_U
    }

    /// Spezza un intervallo in tratti corti che non scavalcano i vertici.
    pub fn pieces(&self, a: f32, b: f32) -> Vec<(f32, f32)> {
        let mut cuts = vec![a];
        cuts.extend(
            self.cum
                .iter()
                .copied()
                .filter(|c| *c > a + 0.01 && *c < b - 0.01),
        );
        cuts.push(b);
        let mut out = Vec::new();
        for w in cuts.windows(2) {
            let k = ((w[1] - w[0]) / PIECE_LEN).ceil().max(1.0) as usize;
            for j in 0..k {
                let d = (w[1] - w[0]) / k as f32;
                out.push((w[0] + d * j as f32, w[0] + d * (j + 1) as f32));
            }
        }
        out
    }
}

/// Faccia di una torre: dove è pieno e dove si passa.
struct TowerFace {
    origin: Vec2,
    along: Vec2,
    solid: Vec<(f32, f32)>,
    doors: Vec<f32>,
}

/// Torre cava: base piena fino al piano, stanza con un varco verso ogni tratto di camminamento.
struct TowerPlan {
    center: Vec2,
    t: Vec2,
    n: Vec2,
    half_t: f32,
    half_n: f32,
    floor: f32,
    s: f32,
    neighbours: Vec<Vec2>,
}

impl TowerPlan {
    fn faces(&self) -> Vec<TowerFace> {
        // (direzione verso fuori, direzione lungo la faccia, semi-lunghezza, semi-profondità)
        let spec = [
            (self.t, self.n, self.half_n, self.half_t),
            (-self.t, self.n, self.half_n, self.half_t),
            (self.n, self.t, self.half_t, self.half_n),
            (-self.n, self.t, self.half_t, self.half_n),
        ];
        spec.iter()
            .map(|&(normal, along, half_len, depth)| {
                let mut doors: Vec<f32> = self
                    .neighbours
                    .iter()
                    .filter_map(|target| {
                        door_on_face(*target - self.center, normal, along, half_len, depth, self)
                    })
                    .collect();
                doors.sort_by(f32::total_cmp);
                doors.dedup_by(|a, b| (*a - *b).abs() < TOWER_DOOR_HALF);
                TowerFace {
                    origin: self.center + normal * (depth - TOWER_WALL / 2.0),
                    along,
                    solid: solid_spans(half_len, &doors),
                    doors,
                }
            })
            .collect()
    }
}

/// Coordinata lungo la faccia del varco, se il raggio verso `d` esce da questa faccia.
fn door_on_face(
    d: Vec2,
    normal: Vec2,
    along: Vec2,
    half_len: f32,
    depth: f32,
    tower: &TowerPlan,
) -> Option<f32> {
    let (dt, dn) = (d.dot(tower.t), d.dot(tower.n));
    let t_dominant = dt.abs() / tower.half_t >= dn.abs() / tower.half_n;
    let this_is_t = normal.dot(tower.t).abs() > 0.5;
    if t_dominant != this_is_t || d.dot(normal) <= 0.0 {
        return None;
    }
    let coord = d.dot(along) / d.dot(normal) * depth;
    Some(coord.clamp(
        -(half_len - TOWER_DOOR_HALF - 0.1),
        half_len - TOWER_DOOR_HALF - 0.1,
    ))
}

/// Parti piene di una faccia lunga `2 * half`, lasciando i varchi.
fn solid_spans(half: f32, doors: &[f32]) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    let mut cur = -half;
    for d in doors {
        if d - TOWER_DOOR_HALF - cur > 0.02 {
            out.push((cur, d - TOWER_DOOR_HALF));
        }
        cur = d + TOWER_DOOR_HALF;
    }
    if half - cur > 0.02 {
        out.push((cur, half));
    }
    out
}

/// Punto del percorso chiuso `path` a distanza `s` dall'inizio, con la direzione di marcia.
pub fn point_on_loop(path: &[Vec3], s: f32) -> (Vec3, Vec2) {
    let n = path.len();
    let len: f32 = (0..n).map(|i| path[i].distance(path[(i + 1) % n])).sum();
    let mut left = s.rem_euclid(len.max(1e-3));
    for i in 0..n {
        let (a, b) = (path[i], path[(i + 1) % n]);
        let l = a.distance(b);
        if left <= l || i + 1 == n {
            let k = (left / l.max(1e-4)).clamp(0.0, 1.0);
            let flat = Vec2::new(b.x - a.x, b.z - a.z).normalize_or_zero();
            return (a.lerp(b, k), flat);
        }
        left -= l;
    }
    (path[0], Vec2::X)
}

/// Lunghezza del percorso chiuso.
pub fn loop_length(path: &[Vec3]) -> f32 {
    let n = path.len();
    (0..n).map(|i| path[i].distance(path[(i + 1) % n])).sum()
}

/// Diagnostica (`PLANET_ONE_SCAN_WALLS=1`): dopo qualche secondo tira raggi dalla campagna contro
/// le mura, a più altezze, e scrive nel log i tratti dove passano senza colpire nulla (fuori dai cancelli).
fn scan_walls(
    time: Res<Time>,
    info: Res<VillageInfo>,
    field: Option<Res<crate::terrain::TerrainField>>,
    spatial: avian3d::prelude::SpatialQuery,
    mut done: Local<bool>,
) {
    let Some(field) = field else {
        return;
    };
    if *done
        || time.elapsed_secs() < 25.0
        || info.poly.len() < 3
        || std::env::var_os("PLANET_ONE_SCAN_WALLS").is_none()
    {
        return;
    }
    *done = true;
    let mut holes = 0;
    for i in 0..info.poly.len() {
        let (a, b) = (info.poly[i], info.poly[(i + 1) % info.poly.len()]);
        let len = a.distance(b);
        let dir = (b - a) / len;
        let mut out = Vec2::new(dir.y, -dir.x);
        if in_polygon(&info.poly, (a + b) / 2.0 + out * 0.5) {
            out = -out;
        }
        let steps = (len / 0.15) as usize;
        for k in 0..=steps {
            let p = a + dir * (k as f32 * 0.15);
            if info.gates.iter().any(|g| g.pos.distance(p) < GATE_ZONE) {
                continue;
            }
            for h in [0.3f32, 1.0, 2.0, 3.0, 3.8, 4.4] {
                let from = p + out * 9.0;
                let Ok(d) = Dir3::new(Vec3::new(-out.x, 0.0, -out.y)) else {
                    continue;
                };
                // La quota è assoluta rispetto al terreno al piede del muro: si parte da y = h + 1.
                let origin = Vec3::new(from.x, field.height_at(p.x, p.y) + h, from.y);
                let hit = spatial.cast_ray(
                    origin,
                    d,
                    13.0,
                    true,
                    &avian3d::prelude::SpatialQueryFilter::default(),
                );
                if hit.is_none() {
                    holes += 1;
                    info!(
                        "[SCAN] buco al muro {i} in ({:.1},{:.1}) quota {:.1} (verso {:.0},{:.0})",
                        p.x,
                        p.y,
                        h + 1.0,
                        -out.x,
                        -out.y
                    );
                }
            }
        }
    }
    // Dall'alto: lungo la fascia delle mura il piano di camminata deve essere continuo. Si cercano
    // i vuoti corti fra due tratti pieni, alla stessa distanza dal bordo.
    let mut top = 0;
    for i in 0..info.poly.len() {
        let (a, b) = (info.poly[i], info.poly[(i + 1) % info.poly.len()]);
        let len = a.distance(b);
        let dir = (b - a) / len;
        let mut out = Vec2::new(dir.y, -dir.x);
        if in_polygon(&info.poly, (a + b) / 2.0 + out * 0.5) {
            out = -out;
        }
        let n = (len / 0.3) as usize + 1;
        for u in [0.4f32, 1.0, 1.6, 2.2, 2.8] {
            let deck: Vec<bool> = (0..n)
                .map(|k| {
                    let p = a + dir * (k as f32 * 0.3);
                    let q = p + out * (u - 1.4);
                    let origin = Vec3::new(q.x, field.height_at(p.x, p.y) + 12.0, q.y);
                    spatial
                        .cast_ray(
                            origin,
                            Dir3::NEG_Y,
                            14.0,
                            true,
                            &avian3d::prelude::SpatialQueryFilter::default(),
                        )
                        .is_some_and(|h| origin.y - h.distance > field.height_at(q.x, q.y) + 2.5)
                })
                .collect();
            let mut k = 1;
            while k < n {
                if deck[k - 1] && !deck[k] {
                    let start = k;
                    while k < n && !deck[k] {
                        k += 1;
                    }
                    // Un vuoto corto fra due tratti pieni (non la fine del muro né i cancelli).
                    if k < n && k - start <= 15 {
                        let p = a + dir * (start as f32 * 0.3) + out * (u - 1.4);
                        top += 1;
                        info!(
                            "[SCAN] vuoto di {:.1} m sulla fascia del muro {i} in ({:.1},{:.1}) lato {u:.1}",
                            (k - start) as f32 * 0.3,
                            p.x,
                            p.y
                        );
                    }
                }
                k += 1;
            }
        }
    }
    info!("[SCAN] dall'alto: {top} vuoti");
    info!("[SCAN] finito: {holes} raggi passati senza colpire");
}

pub fn plugin(app: &mut App) {
    app.init_resource::<VillageInfo>()
        .add_systems(Startup, init_village)
        .add_systems(
            Update,
            (sync_village, gate_schedule, village_banner, scan_walls),
        );
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

    #[test]
    fn walk_heights_keep_pins_and_limit_the_slope() {
        let mut ground = vec![0.0f32; 200];
        for (i, g) in ground.iter_mut().enumerate() {
            *g = (i as f32 * 0.09).sin() * 2.2;
        }
        let h = walk_heights(&ground, &[(10, 12, 3.6)]);
        assert!(h[10..22].iter().all(|y| (y - 3.6).abs() < 1e-3));
        let n = h.len();
        for i in 0..n {
            let d = (h[(i + 1) % n] - h[i]).abs();
            assert!(d <= MAX_WALK_SLOPE + 0.02, "pendenza {d} a {i}");
        }
        // A tratti di 2,2 m il dislivello resta sotto i gradini delle scale.
        const { assert!(MAX_WALK_SLOPE * PIECE_LEN < STEP_RISE) };
    }

    #[test]
    fn free_intervals_handle_wraparound() {
        let f = free_intervals(100.0, &[(95.0, 105.0), (40.0, 50.0)]);
        assert_eq!(f, vec![(5.0, 40.0), (50.0, 95.0)]);
    }

    #[test]
    fn perimeter_points_are_outward_and_pieces_short() {
        let poly = square();
        let lp = Perimeter::new(&poly);
        assert_eq!(lp.total, 80.0);
        let (p, _, out) = lp.at(10.0);
        assert!(in_polygon(&poly, p - out * 0.5) && !in_polygon(&poly, p + out * 0.5));
        for (a, b) in lp.pieces(15.0, 45.0) {
            assert!(b - a <= PIECE_LEN + 1e-3);
        }
        // I tratti non scavalcano i vertici.
        assert!(
            lp.pieces(15.0, 45.0)
                .iter()
                .all(|(a, b)| *b <= 20.0 + 1e-3 || *a >= 20.0 - 1e-3)
        );
    }

    #[test]
    fn towers_have_an_opening_toward_each_neighbour() {
        let tower = TowerPlan {
            center: Vec2::ZERO,
            t: Vec2::X,
            n: Vec2::Y,
            half_t: TOWER_HALF,
            half_n: TOWER_HALF,
            floor: 0.0,
            s: 0.0,
            neighbours: vec![Vec2::new(-5.0, 0.3), Vec2::new(0.4, 5.0)],
        };
        let doors: usize = tower.faces().iter().map(|f| f.doors.len()).sum();
        assert_eq!(doors, 2);
        for face in tower.faces() {
            for d in &face.doors {
                assert!(
                    face.solid
                        .iter()
                        .all(|(a, b)| *b <= d - TOWER_DOOR_HALF + 1e-3
                            || *a >= d + TOWER_DOOR_HALF - 1e-3)
                );
            }
        }
    }

    #[test]
    fn point_on_loop_wraps_around() {
        let path = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(10.0, 1.0, 0.0),
            Vec3::new(10.0, 1.0, 10.0),
            Vec3::new(0.0, 1.0, 10.0),
        ];
        assert_eq!(loop_length(&path), 40.0);
        let (p, dir) = point_on_loop(&path, 15.0);
        assert!(p.distance(Vec3::new(10.0, 1.0, 5.0)) < 1e-3);
        assert!(dir.y > 0.9);
        assert!(point_on_loop(&path, 55.0).0.distance(p) < 1e-3);
    }
}
