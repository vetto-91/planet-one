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

/// Camminamento sulle mura: etichetta (nord/sud/est/ovest), centro della striscia e quota del piano.
#[derive(Clone, Debug)]
pub struct Walk {
    pub label: String,
    pub pts: Vec<Vec2>,
    pub y: f32,
}

/// Dati del villaggio per il resto del gioco (mappa, terreno, banner).
#[derive(Resource, Default, Clone)]
pub struct VillageInfo {
    pub spec: VillageSpec,
    pub poly: Vec<Vec2>,
    pub gates: Vec<Gate>,
    pub center: Vec2,
    /// Camminamenti (riempiti da `sync_village` quando costruisce le mura).
    pub walks: Vec<Walk>,
    /// Porta della casa delle guardie e direzione dello sguardo verso l'esterno.
    pub barracks: Option<(Vec2, Vec2)>,
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
            walks: Vec::new(),
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

    // --- Camminamenti: tratti scelti, quota del piano e gradini (la scala parte dal lato del cancello) ---
    struct WalkBuild {
        label: String,
        run: Run,
        y: f32,
        g0: f32,
        steps: usize,
        start: f32,
        end: f32,
    }
    let mut walk_builds: Vec<WalkBuild> = Vec::new();
    let clear = walk_clearance(&dbg.items);
    for (label, run) in pick_walkable(&info.poly, &info.gates, &ground, &clear) {
        let len = run.len();
        let strip = |s: f32| {
            let (p, _, inw) = run_point(&info.poly, &run, s);
            p + inw * (WALL_T / 2.0 + WALK_W / 2.0)
        };
        let hs: Vec<f32> = (0..=(len as usize))
            .map(|k| ground(strip(k as f32)))
            .collect();
        let gmax = hs.iter().cloned().fold(f32::MIN, f32::max);
        let gmean = hs.iter().sum::<f32>() / hs.len() as f32;
        // Piano a 2,4 m sopra il terreno medio, ma mai a meno di 1,3 m sopra il punto più alto.
        let y = (gmean + WALK_H).max(gmax + 1.3);
        let start = if run.gate_start.is_some() {
            0.1
        } else {
            TOWER_MARGIN
        };
        let end = if run.gate_end.is_some() {
            0.1
        } else {
            TOWER_MARGIN
        };
        let g0 = ground(strip(start));
        let steps = ((y - g0) / STEP_RISE).ceil().max(1.0) as usize;
        let walk_len = len - end - start - steps as f32 * STEP_RUN;
        info!(
            "[VILLAGGIO] camminamento {label}: tratto {len:.1} m, piano a {y:.2} m (terreno {g0:.2}..{gmax:.2}), {steps} gradini, piano utile {walk_len:.1} m"
        );
        if walk_len < 2.5 {
            continue;
        }
        walk_builds.push(WalkBuild {
            label,
            run,
            y,
            g0,
            steps,
            start,
            end,
        });
    }
    // Per ogni lato: intervalli (da, a, quota) dove il muro deve salire fino al piano.
    let raised: Vec<(usize, f32, f32, f32)> = walk_builds
        .iter()
        .flat_map(|w| {
            w.run
                .spans
                .iter()
                .map(|s| (s.0, s.1.min(s.2), s.1.max(s.2), w.y))
        })
        .collect();
    let mut walk_colliders: Vec<(Vec3, Vec3, f32)> = Vec::new();
    let mut walks: Vec<Walk> = Vec::new();

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
                let um = (u0 + u1) / 2.0;
                let mut hi = hs.iter().cloned().fold(f32::MIN, f32::max) + WALL_H;
                if let Some(r) = raised
                    .iter()
                    .find(|r| r.0 == i && um >= r.1 - 0.01 && um <= r.2 + 0.01)
                {
                    hi = hi.max(r.3 + 1.1);
                }
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
        if info.gates.iter().any(|g| g.pos.distance(v) < TOWER_CLEAR) {
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

    // --- Camminamenti e scale ---
    for w in &walk_builds {
        let len = w.run.len();
        let at = |s: f32| {
            let (p, t, inw) = run_point(&info.poly, &w.run, s);
            (p + inw * (WALL_T / 2.0 + WALK_W / 2.0), t)
        };
        let rise = (w.y - w.g0) / w.steps as f32;
        for i in 0..w.steps {
            let (c, t) = at(w.start + (i as f32 + 0.5) * STEP_RUN);
            let top = w.g0 + (i as f32 + 1.0) * rise;
            let lo = ground(c) - 1.0;
            stone.obox(
                c,
                lo,
                top,
                STEP_RUN / 2.0,
                WALK_W / 2.0,
                t,
                tint(STONE_DARK, rnd()),
            );
            walk_colliders.push((
                Vec3::new(c.x, (lo + top) / 2.0, c.y),
                Vec3::new(STEP_RUN, top - lo, WALK_W),
                (-t.y).atan2(t.x),
            ));
        }
        let s_from = w.start + w.steps as f32 * STEP_RUN;
        let s_to = len - w.end;
        let k = (((s_to - s_from) / 2.2).ceil() as usize).max(1);
        let mut pts = Vec::new();
        for j in 0..k {
            let (a, b) = (
                s_from + (s_to - s_from) * j as f32 / k as f32,
                s_from + (s_to - s_from) * (j + 1) as f32 / k as f32,
            );
            let (c, t) = at((a + b) / 2.0);
            let lo = ground(c) - 1.0;
            let half = (b - a) / 2.0 + 0.02;
            stone.obox(c, lo, w.y, half, WALK_W / 2.0, t, tint(STONE, rnd()));
            walk_colliders.push((
                Vec3::new(c.x, (lo + w.y) / 2.0, c.y),
                Vec3::new(half * 2.0, w.y - lo, WALK_W),
                (-t.y).atan2(t.x),
            ));
            pts.push(c);
        }
        walks.push(Walk {
            label: w.label.clone(),
            pts,
            y: w.y,
        });
    }

    // --- Cancelli: piloni con torrette, architrave, battenti e maniglia ---
    let wood = pbr.get(crate::pbr::Mat::Planks);
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
            let leaf = Cuboid::new(GATE_HALF, 3.1, 0.14);
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
                let c = g + t * (pier * (GATE_HALF + PIER_HALF)) + gate.inward * (side * 0.95);
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
    info.walks = walks;
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
    let inside = in_polygon(&info.poly, pos);
    if *was_inside == Some(false) && inside {
        quest.title = Some((info.spec.name.clone(), 4.0));
    }
    *was_inside = Some(inside);
}

// ---------------------------------------------------------------------------------------------
// Camminamenti sulle mura (ADR 0013)
// ---------------------------------------------------------------------------------------------

/// Distanza dal cancello oltre la quale un vertice del muro ha la sua torretta.
const TOWER_CLEAR: f32 = 6.8;
/// Quota del piano di camminamento sopra il punto più alto del terreno del tratto (m).
const WALK_H: f32 = 2.4;
/// Larghezza del camminamento (m), alzata e pedata dei gradini (m).
const WALK_W: f32 = 1.2;
const STEP_RISE: f32 = 0.3;
const STEP_RUN: f32 = 0.38;
/// Margine (m) lasciato libero davanti a una torretta piena.
const TOWER_MARGIN: f32 = 1.9;
/// Lunghezza minima (m) di un tratto per ospitare scala e camminamento.
const MIN_RUN: f32 = 8.0;

/// Funzione "qui è libero" per i camminamenti: fuori da edifici (rettangoli ruotati, la casetta compresa)
/// e da pali, lampioni e alberi (cerchi), con un margine di 0,4 m.
pub fn walk_clearance(items: &[Placement]) -> impl Fn(Vec2) -> bool {
    // (centro, semi-lati locali, yaw); la casetta del Viandante sta in (6,5, 0,7), 4×6 m.
    let mut rects: Vec<(Vec2, Vec2, f32)> = vec![(Vec2::new(6.5, 0.7), Vec2::new(2.0, 3.0), 0.0)];
    let mut circles: Vec<(Vec2, f32)> = Vec::new();
    for pl in items {
        for (j, &p) in pl.points.iter().enumerate() {
            if crate::buildings::is_building(&pl.kind) {
                let half = crate::buildings::model(&pl.kind, 1).half;
                rects.push((p, half, pl.yaw(j)));
            } else if matches!(
                pl.kind.as_str(),
                "lampione" | "cartello" | "albero" | "panchina" | "pozzo"
            ) {
                circles.push((p, crate::debug_place::footprint(&pl.kind)));
            }
        }
    }
    move |q: Vec2| {
        let m = 0.4;
        !rects.iter().any(|(c, half, yaw)| {
            let l = Quat::from_rotation_y(-yaw) * Vec3::new(q.x - c.x, 0.0, q.y - c.y);
            l.x.abs() <= half.x + m && l.z.abs() <= half.y + m
        }) && !circles.iter().any(|(c, r)| c.distance(q) <= r + 0.3)
    }
}

/// Tratto di muro compreso tra due ostacoli (torretta o pilone di cancello).
#[derive(Clone, Debug)]
pub struct Run {
    /// (lato, ascissa iniziale, ascissa finale) lungo il lato; l'ascissa può decrescere.
    pub spans: Vec<(usize, f32, f32)>,
    /// Cancello contro cui il tratto inizia / finisce, se c'è.
    pub gate_start: Option<usize>,
    pub gate_end: Option<usize>,
}

impl Run {
    pub fn len(&self) -> f32 {
        self.spans.iter().map(|s| (s.2 - s.1).abs()).sum()
    }

    fn reversed(&self) -> Run {
        Run {
            spans: self.spans.iter().rev().map(|s| (s.0, s.2, s.1)).collect(),
            gate_start: self.gate_end,
            gate_end: self.gate_start,
        }
    }
}

fn edge_len(poly: &[Vec2], i: usize) -> f32 {
    poly[i].distance(poly[(i + 1) % poly.len()])
}

/// Normale del lato `i` rivolta verso l'interno del villaggio.
fn edge_inward(poly: &[Vec2], i: usize) -> Vec2 {
    let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
    let t = (b - a).normalize_or_zero();
    let n = Vec2::new(-t.y, t.x);
    if in_polygon(poly, (a + b) / 2.0 + n * 0.5) {
        n
    } else {
        -n
    }
}

/// Suddivide il muro in tratti (catene di pezzi di muro) separati da torrette e cancelli.
pub fn wall_runs(poly: &[Vec2], gates: &[Gate]) -> Vec<Run> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    let tower = |v: usize| !gates.iter().any(|g| g.pos.distance(poly[v]) < TOWER_CLEAR);
    // Pezzi pieni di ogni lato (fuori dai vani dei cancelli).
    #[allow(clippy::type_complexity)]
    let mut pieces: Vec<(usize, f32, f32, Option<usize>, Option<usize>)> = Vec::new();
    for i in 0..n {
        let len = edge_len(poly, i);
        let mut gaps: Vec<(f32, f32, usize)> = gates
            .iter()
            .enumerate()
            .filter(|(_, g)| g.edge == i)
            .map(|(k, g)| {
                (
                    (g.along - GATE_HALF - 2.0 * PIER_HALF).max(0.0),
                    (g.along + GATE_HALF + 2.0 * PIER_HALF).min(len),
                    k,
                )
            })
            .collect();
        gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mut cur, mut before) = (0.0f32, None);
        for (g0, g1, k) in gaps {
            if g0 - cur > 0.05 {
                pieces.push((i, cur, g0, before, Some(k)));
            }
            cur = g1;
            before = Some(k);
        }
        if len - cur > 0.05 {
            pieces.push((i, cur, len, before, None));
        }
    }
    let mut runs: Vec<Run> = Vec::new();
    for (e, s0, s1, gb, ga) in pieces {
        let join = runs.last().is_some_and(|r| {
            let l = r.spans.last().unwrap();
            l.0 + 1 == e
                && (l.2 - edge_len(poly, l.0)).abs() < 0.05
                && s0 < 0.05
                && r.gate_end.is_none()
                && !tower(e)
        });
        if join {
            let r = runs.last_mut().unwrap();
            r.spans.push((e, s0, s1));
            r.gate_end = ga;
        } else {
            runs.push(Run {
                spans: vec![(e, s0, s1)],
                gate_start: gb,
                gate_end: ga,
            });
        }
    }
    // Raccordo ciclico: ultimo tratto → primo, se non c'è un ostacolo in mezzo.
    if runs.len() >= 2 {
        let (f, l) = (runs[0].clone(), runs[runs.len() - 1].clone());
        let (fs, ls) = (f.spans[0], l.spans[l.spans.len() - 1]);
        if fs.0 == 0
            && fs.1 < 0.05
            && ls.0 == n - 1
            && (ls.2 - edge_len(poly, ls.0)).abs() < 0.05
            && l.gate_end.is_none()
            && f.gate_start.is_none()
            && !tower(0)
        {
            let mut merged = l;
            merged.spans.extend(f.spans);
            merged.gate_end = f.gate_end;
            runs.pop();
            runs[0] = merged;
        }
    }
    runs
}

/// Sceglie i tratti da rendere percorribili: il fianco più lungo di ogni cancello (la scala "della porta")
/// più il tratto più lungo a est e quello più lungo a ovest. Ogni tratto parte dal lato della scala.
pub fn pick_walkable(
    poly: &[Vec2],
    gates: &[Gate],
    ground: &dyn Fn(Vec2) -> f32,
    clear: &dyn Fn(Vec2) -> bool,
) -> Vec<(String, Run)> {
    let runs = wall_runs(poly, gates);
    // Un tratto è usabile se è abbastanza lungo e la striscia di camminamento non attraversa edifici o pali.
    let usable = |r: &Run| {
        r.len() >= MIN_RUN
            && (0..=(r.len() * 2.0) as usize).all(|k| {
                let (p, _, inw) = run_point(poly, r, k as f32 * 0.5);
                [0.0f32, 0.5, 1.0, 1.5]
                    .iter()
                    .all(|d| clear(p + inw * (WALL_T / 2.0 + *d)))
            })
    };
    let mut used = vec![false; runs.len()];
    let mut out: Vec<(String, Run)> = Vec::new();
    for (k, g) in gates.iter().enumerate() {
        let best = (0..runs.len())
            .filter(|&i| runs[i].gate_start == Some(k) || runs[i].gate_end == Some(k))
            .filter(|&i| usable(&runs[i]))
            .max_by(|a, b| runs[*a].len().total_cmp(&runs[*b].len()));
        if let Some(i) = best {
            used[i] = true;
            let r = if runs[i].gate_start == Some(k) {
                runs[i].clone()
            } else {
                runs[i].reversed()
            };
            out.push((g.name.to_lowercase(), r));
        }
    }
    let center = poly.iter().fold(Vec2::ZERO, |a, b| a + *b) / poly.len().max(1) as f32;
    let score = |r: &Run| {
        let hs: Vec<f32> = (0..=(r.len() as usize))
            .map(|k| ground(run_point(poly, r, k as f32).0))
            .collect();
        let range = hs.iter().cloned().fold(f32::MIN, f32::max)
            - hs.iter().cloned().fold(f32::MAX, f32::min);
        r.len() - 12.0 * range
    };
    for (label, sign) in [("est", 1.0f32), ("ovest", -1.0)] {
        let best = (0..runs.len())
            .filter(|&i| !used[i] && usable(&runs[i]))
            .filter(|&i| {
                let (p, _, _) = run_point(poly, &runs[i], runs[i].len() / 2.0);
                (p.x - center.x) * sign > 8.0
            })
            // Preferisce i tratti lunghi e in piano (dove il muro non deve salire troppo).
            .max_by(|a, b| score(&runs[*a]).total_cmp(&score(&runs[*b])));
        if let Some(i) = best {
            used[i] = true;
            out.push((label.to_string(), runs[i].clone()));
        }
    }
    out
}

/// Punto del tratto a distanza `s` dall'inizio: (posizione sul muro, tangente, normale verso l'interno).
pub fn run_point(poly: &[Vec2], run: &Run, s: f32) -> (Vec2, Vec2, Vec2) {
    let mut acc = 0.0;
    for (k, &(e, s0, s1)) in run.spans.iter().enumerate() {
        let l = (s1 - s0).abs();
        if s <= acc + l || k + 1 == run.spans.len() {
            let u = (s - acc).clamp(0.0, l);
            let dir = if s1 >= s0 { 1.0 } else { -1.0 };
            let (a, b) = (poly[e], poly[(e + 1) % poly.len()]);
            let t = (b - a).normalize_or_zero();
            return (a + t * (s0 + dir * u), t * dir, edge_inward(poly, e));
        }
        acc += l;
    }
    (Vec2::ZERO, Vec2::X, Vec2::Y)
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

    fn big_square() -> Vec<Vec2> {
        vec![
            Vec2::new(-20.0, -20.0),
            Vec2::new(20.0, -20.0),
            Vec2::new(20.0, 20.0),
            Vec2::new(-20.0, 20.0),
        ]
    }

    #[test]
    fn walls_split_into_runs_between_gates_and_towers() {
        let poly = big_square();
        let gates = project_gates(
            &poly,
            &[
                ("Nord".into(), Vec2::new(0.0, -20.0)),
                ("Sud".into(), Vec2::new(0.0, 20.0)),
            ],
        );
        let runs = wall_runs(&poly, &gates);
        // Ogni cancello ha due fianchi; est e ovest sono tratti interi tra due torrette (angoli).
        assert!(runs.len() >= 4, "{}", runs.len());
        assert!(runs.iter().all(|r| r.len() > 0.0));
    }

    #[test]
    fn walkable_runs_cover_gates_east_and_west() {
        let poly = big_square();
        let gates = project_gates(
            &poly,
            &[
                ("Nord".into(), Vec2::new(0.0, -20.0)),
                ("Sud".into(), Vec2::new(0.0, 20.0)),
            ],
        );
        let walkable = pick_walkable(&poly, &gates, &|_| 0.0, &|_| true);
        let labels: Vec<&str> = walkable.iter().map(|(l, _)| l.as_str()).collect();
        for want in ["nord", "sud", "est", "ovest"] {
            assert!(labels.contains(&want), "manca {want}: {labels:?}");
        }
        // Il tratto di un cancello parte contro il cancello (lì c'è la scala).
        let nord = &walkable.iter().find(|(l, _)| l == "nord").unwrap().1;
        assert_eq!(nord.gate_start, Some(0));
        let (p, _, inw) = run_point(&poly, nord, 0.0);
        assert!(p.distance(gates[0].pos) > 2.0 && p.distance(gates[0].pos) < 6.0);
        assert!(inw.y > 0.5, "la normale guarda verso l'interno");
    }
}
