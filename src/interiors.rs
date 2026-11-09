//! Interni degli edifici (ADR 0019): piani, scale e arredi costruiti con lo stesso `B` degli esterni.
//!
//! Le coordinate sono locali all'edificio. Ogni stanza è un rettangolo con la quota del pavimento; i mobili
//! grandi hanno un collider, i piccoli no. I letti sono registrati: gli abitanti ci dormono di notte.

use crate::buildings::{B, Col, Interior};
use crate::containers::{ContainerKind, ContainerSpot};
use bevy::prelude::*;

const WOOD: Col = [0.42, 0.28, 0.16, 1.0];
const WOOD_DARK: Col = [0.28, 0.19, 0.11, 1.0];
const LINEN: Col = [0.82, 0.78, 0.68, 1.0];
const BLANKET: [Col; 4] = [
    [0.55, 0.16, 0.14, 1.0],
    [0.2, 0.32, 0.55, 1.0],
    [0.3, 0.45, 0.25, 1.0],
    [0.55, 0.42, 0.2, 1.0],
];
const STONE_FIRE: Col = [0.74, 0.72, 0.68, 1.0];
const IRON: Col = [0.24, 0.24, 0.26, 1.0];
const STRAW: Col = [0.85, 0.7, 0.3, 1.0];

/// Alzata massima dei gradini (m) e pedata.
pub const MAX_RISE: f32 = 0.28;
const RUN: f32 = 0.27;
/// Larghezza delle rampe.
const STAIR_WIDTH: f32 = 1.05;
/// Corsia libera (m) ai piedi di una rampa.
const LANE: f32 = 0.75;
/// Spessore dei solai.
const SLAB: f32 = 0.18;

/// Stanza: rettangolo libero e quota del pavimento.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Room {
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
    pub y: f32,
    /// Piano (0 = terra): i letti lo ricordano per il percorso degli abitanti.
    pub floor: usize,
    /// Parete di fondo e ascissa da cui la fascia lungo di essa è libera (a destra della rampa).
    pub back_z: f32,
    pub back_x: f32,
}

impl Room {
    fn x(&self, a: f32) -> f32 {
        self.x0 + (self.x1 - self.x0) * a
    }
    fn z(&self, a: f32) -> f32 {
        self.z0 + (self.z1 - self.z0) * a
    }
}

/// Dove sono finite le scale: serve a non metterci i mobili davanti.
#[derive(Clone, Debug)]
pub(crate) struct Layout {
    /// Fine della prima rampa: da qui la fascia di fondo del piano terra è libera.
    pub back_x: f32,
    /// Lato sinistro dell'ultima rampa, che sale al sottotetto lungo la parete destra.
    pub top_x0: f32,
    /// Per ogni piano, i punti da percorrere dal piano terra fino al pianerottolo.
    pub routes: Vec<Vec<Vec3>>,
}

/// Scatola con collider (mobile grande: ferma il giocatore, non la camera).
fn solid(b: &mut B, int: &mut Interior, c: Vec3, s: Vec3, col: Col) {
    b.cuboid_c(c, s, col);
    int.furniture.push((c, s));
}

/// Rettangolo di pavimento: visibile e calpestabile.
pub(crate) fn slab(b: &mut B, int: &mut Interior, x0: f32, x1: f32, z0: f32, z1: f32, top: f32) {
    if x1 - x0 < 0.02 || z1 - z0 < 0.02 {
        return;
    }
    let c = Vec3::new((x0 + x1) / 2.0, top - SLAB / 2.0, (z0 + z1) / 2.0);
    let s = Vec3::new(x1 - x0, SLAB, z1 - z0);
    b.cuboid_c(c, s, WOOD);
    int.ground.push((c, s));
}

/// Solaio con un foro rettangolare (vano scala).
pub(crate) fn slab_with_hole(
    b: &mut B,
    int: &mut Interior,
    (x0, x1, z0, z1): (f32, f32, f32, f32),
    (hx0, hx1, hz0, hz1): (f32, f32, f32, f32),
    top: f32,
) {
    slab(b, int, x0, hx0, z0, z1, top);
    slab(b, int, hx1, x1, z0, z1, top);
    slab(b, int, hx0, hx1, z0, hz0, top);
    slab(b, int, hx0, hx1, hz1, z1, top);
}

/// Gradino pieno da `lower` a `top`, calpestabile.
#[allow(clippy::too_many_arguments)]
fn step(b: &mut B, int: &mut Interior, xa: f32, xb: f32, za: f32, zb: f32, lower: f32, top: f32) {
    let c = Vec3::new((xa + xb) / 2.0, (lower + top) / 2.0, (za + zb) / 2.0);
    let s = Vec3::new(xb - xa + 0.005, top - lower, zb - za + 0.005);
    b.cuboid_c(c, s, WOOD_DARK);
    int.ground.push((c, s));
}

/// Solai e scale: `ys` sono le quote dei pavimenti dal basso verso l'alto, l'ultima è il sottotetto.
/// Le rampe tra i piani stanno lungo la parete di fondo, alternando il verso; l'ultima corre lungo la
/// parete destra e arriva nella zona alta del sottotetto (`knee` = semi-larghezza dove ci si sta in piedi).
pub(crate) fn floors_and_stairs(
    b: &mut B,
    int: &mut Interior,
    rect: (f32, f32, f32, f32),
    ys: &[f32],
    knee: f32,
) -> Layout {
    let (x0, x1, z0, z1) = rect;
    slab(b, int, x0, x1, z0, z1, ys[0]);
    let flights = ys.len() - 1;
    let mut layout = Layout {
        back_x: x0,
        top_x0: x1,
        routes: vec![Vec::new()],
    };
    let mut walk: Vec<Vec3> = Vec::new();
    for i in 0..flights {
        let (lower, upper) = (ys[i], ys[i + 1]);
        let n = ((upper - lower) / MAX_RISE).ceil() as usize;
        let rise = (upper - lower) / n as f32;
        let len = n as f32 * RUN;
        let hole = if i + 1 == flights {
            let (tx0, tx1) = (x1 - STAIR_WIDTH - 0.05, x1 - 0.05);
            let top = knee - 0.05;
            let start = top - len;
            for k in 0..n {
                let mut za = start + k as f32 * RUN;
                // Senza spazio ai piedi, il primo gradino diventa un pianerottolo fino al muro.
                if k == 0 && start - z0 < LANE {
                    za = z0;
                }
                let zb = start + (k + 1) as f32 * RUN;
                let t = lower + (k + 1) as f32 * rise;
                step(b, int, tx0, tx1, za, zb, lower, t);
            }
            layout.top_x0 = tx0;
            (tx0, tx1, start, top)
        } else {
            let forward = i % 2 == 0;
            let origin = if forward { x0 + LANE } else { x1 - LANE };
            let dir = if forward { 1.0 } else { -1.0 };
            for k in 0..n {
                let (p, q) = (
                    origin + dir * k as f32 * RUN,
                    origin + dir * (k + 1) as f32 * RUN,
                );
                let t = lower + (k + 1) as f32 * rise;
                step(b, int, p.min(q), p.max(q), z0, z0 + STAIR_WIDTH, lower, t);
            }
            let end = origin + dir * len;
            if i == 0 {
                layout.back_x = end;
            }
            // Percorso degli abitanti: davanti alla corsia, ai piedi della rampa, in cima, fuori dal vano.
            let lane_x = origin - dir * LANE / 2.0;
            let mid = z0 + STAIR_WIDTH / 2.0;
            let front = z0 + STAIR_WIDTH + 0.35;
            walk.extend([
                Vec3::new(lane_x, lower, front),
                Vec3::new(lane_x, lower, mid),
                Vec3::new(origin, lower, mid),
                Vec3::new(end, upper, mid),
                Vec3::new(end, upper, front),
            ]);
            layout.routes.push(walk.clone());
            (origin.min(end), origin.max(end), z0, z0 + STAIR_WIDTH)
        };
        slab_with_hole(b, int, rect, hole, upper);
    }
    layout
}

/// Scala a chiocciola quadrata lungo le pareti del rettangolo (campanile), da `y0` a `y1`.
/// Parte dall'angolo fronte-destra, verso il fondo; restituisce il foro da lasciare nel solaio d'arrivo.
pub(crate) fn square_helix(
    b: &mut B,
    int: &mut Interior,
    (x0, x1, z0, z1): (f32, f32, f32, f32),
    y0: f32,
    y1: f32,
    lane: f32,
) -> (f32, f32, f32, f32) {
    const THICK: f32 = 0.34;
    let n = ((y1 - y0) / MAX_RISE).ceil() as usize;
    let rise = (y1 - y0) / n as f32;
    // Celle di un giro: (xa, xb, za, zb).
    let mut lap: Vec<(f32, f32, f32, f32)> = Vec::new();
    let straight = |lap: &mut Vec<(f32, f32, f32, f32)>,
                    from: f32,
                    to: f32,
                    other: (f32, f32),
                    along_z: bool| {
        let cells = (((to - from).abs() / RUN).round() as usize).max(1);
        let d = (to - from) / cells as f32;
        for k in 0..cells {
            let (p, q) = (from + d * k as f32, from + d * (k + 1) as f32);
            let (a, c) = (p.min(q), p.max(q));
            lap.push(if along_z {
                (other.0, other.1, a, c)
            } else {
                (a, c, other.0, other.1)
            });
        }
    };
    let right = (x1 - lane, x1);
    let left = (x0, x0 + lane);
    let back = (z0, z0 + lane);
    let front = (z1 - lane, z1);
    lap.push((right.0, right.1, front.0, front.1));
    straight(&mut lap, front.0, back.1, right, true);
    lap.push((right.0, right.1, back.0, back.1));
    straight(&mut lap, right.0, left.1, back, false);
    lap.push((left.0, left.1, back.0, back.1));
    straight(&mut lap, back.1, front.0, left, true);
    lap.push((left.0, left.1, front.0, front.1));
    straight(&mut lap, left.1, right.0, front, false);
    let mut last = lap[0];
    let mut hole = last;
    for k in 0..n {
        let cell = lap[k % lap.len()];
        let top = y0 + (k + 1) as f32 * rise;
        let lower = (top - THICK).max(y0);
        step(b, int, cell.0, cell.1, cell.2, cell.3, lower, top);
        last = cell;
        if k + 2 >= n {
            // Le ultime due celle restano scoperte nel solaio.
            hole = if k + 2 == n {
                cell
            } else {
                (
                    hole.0.min(cell.0),
                    hole.1.max(cell.1),
                    hole.2.min(cell.2),
                    hole.3.max(cell.3),
                )
            };
        }
    }
    let _ = last;
    hole
}

fn table(b: &mut B, int: &mut Interior, x: f32, z: f32, y: f32, lx: f32, lz: f32) {
    let top = y + 0.76;
    solid(
        b,
        int,
        Vec3::new(x, top - 0.03, z),
        Vec3::new(lx, 0.06, lz),
        WOOD,
    );
    for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        b.cuboid_c(
            Vec3::new(
                x + sx * (lx / 2.0 - 0.07),
                y + 0.365,
                z + sz * (lz / 2.0 - 0.07),
            ),
            Vec3::new(0.09, 0.73, 0.09),
            WOOD_DARK,
        );
    }
}

fn bench(b: &mut B, int: &mut Interior, x: f32, z: f32, y: f32, lx: f32) {
    solid(
        b,
        int,
        Vec3::new(x, y + 0.23, z),
        Vec3::new(lx, 0.46, 0.3),
        WOOD_DARK,
    );
}

fn stool(b: &mut B, x: f32, z: f32, y: f32) {
    b.cuboid_c(
        Vec3::new(x, y + 0.23, z),
        Vec3::new(0.32, 0.46, 0.32),
        WOOD_DARK,
    );
}

fn container(int: &mut Interior, pos: Vec3, kind: ContainerKind) {
    int.containers.push(ContainerSpot { pos, kind });
}

fn chest(b: &mut B, int: &mut Interior, x: f32, z: f32, y: f32) {
    solid(
        b,
        int,
        Vec3::new(x, y + 0.26, z),
        Vec3::new(0.85, 0.52, 0.5),
        WOOD,
    );
    b.cuboid_c(
        Vec3::new(x, y + 0.42, z + 0.26),
        Vec3::new(0.14, 0.14, 0.04),
        IRON,
    );
    container(int, Vec3::new(x, y + 0.62, z), ContainerKind::Chest);
}

/// Armadio; `facing` (+1 o -1) è il verso Z verso cui guardano le ante.
fn wardrobe(b: &mut B, int: &mut Interior, x: f32, z: f32, y: f32, facing: f32) {
    solid(
        b,
        int,
        Vec3::new(x, y + 0.95, z),
        Vec3::new(1.0, 1.9, 0.5),
        WOOD_DARK,
    );
    b.cuboid_c(
        Vec3::new(x, y + 1.0, z + facing * 0.26),
        Vec3::new(0.02, 1.7, 0.02),
        IRON,
    );
    container(
        int,
        Vec3::new(x, y + 1.1, z + facing * 0.32),
        ContainerKind::Wardrobe,
    );
}

/// Scaffale; `facing` (+1 o -1) è il verso Z verso cui guardano i ripiani.
fn shelf(b: &mut B, int: &mut Interior, x: f32, z: f32, y: f32, lx: f32, facing: f32) {
    solid(
        b,
        int,
        Vec3::new(x, y + 0.9, z),
        Vec3::new(lx, 1.8, 0.34),
        WOOD,
    );
    for row in 0..4 {
        for k in 0..3 {
            let a = (row * 3 + k) as f32 * 1.7;
            b.cuboid_c(
                Vec3::new(
                    x - lx * 0.3 + lx * 0.3 * k as f32,
                    y + 0.34 + 0.42 * row as f32,
                    z + facing * 0.18,
                ),
                Vec3::new(0.2, 0.26, 0.06),
                BLANKET[(a as usize) % 4],
            );
        }
    }
    container(
        int,
        Vec3::new(x, y + 1.05, z + facing * 0.24),
        ContainerKind::Shelf,
    );
}

fn barrel(b: &mut B, int: &mut Interior, x: f32, z: f32, y: f32) {
    solid(
        b,
        int,
        Vec3::new(x, y + 0.42, z),
        Vec3::new(0.62, 0.84, 0.62),
        WOOD,
    );
    for hy in [0.18, 0.66] {
        b.cuboid_c(Vec3::new(x, y + hy, z), Vec3::new(0.66, 0.05, 0.66), IRON);
    }
    container(int, Vec3::new(x, y + 0.94, z), ContainerKind::Barrel);
}

fn crate_box(b: &mut B, int: &mut Interior, x: f32, z: f32, y: f32, size: f32) {
    solid(
        b,
        int,
        Vec3::new(x, y + size / 2.0, z),
        Vec3::splat(size),
        WOOD,
    );
    container(int, Vec3::new(x, y + size + 0.08, z), ContainerKind::Crate);
}

/// Letto con la testata verso `head` (±X o ±Z); registra il posto e il punto da cui ci si sdraia.
fn bed(b: &mut B, int: &mut Interior, r: &Room, x: f32, z: f32, head: Vec3, tint: usize) {
    let along_x = head.x.abs() > 0.5;
    let axis = |long: f32, short: f32| {
        if along_x {
            Vec3::new(long, 0.0, short)
        } else {
            Vec3::new(short, 0.0, long)
        }
    };
    let c = Vec3::new(x, r.y, z);
    solid(
        b,
        int,
        c + Vec3::Y * 0.2,
        axis(1.95, 0.95) + Vec3::Y * 0.4,
        WOOD_DARK,
    );
    b.force = Some(crate::pbr::Mat::Cloth);
    b.cuboid_c(c + Vec3::Y * 0.46, axis(1.87, 0.87) + Vec3::Y * 0.14, LINEN);
    b.cuboid_c(
        c - head * 0.34 + Vec3::Y * 0.56,
        axis(1.2, 0.97) + Vec3::Y * 0.08,
        BLANKET[tint % 4],
    );
    b.cuboid_c(
        c + head * 0.72 + Vec3::Y * 0.59,
        axis(0.38, 0.7) + Vec3::Y * 0.12,
        LINEN,
    );
    b.force = None;
    // Di fianco, dal lato della stanza, verso i piedi del letto.
    let side = Vec3::new(head.z.abs(), 0.0, head.x.abs());
    let to_room = Vec3::new(r.x(0.5) - x, 0.0, r.z(0.5) - z);
    let sign = if side.dot(to_room) >= 0.0 { 1.0 } else { -1.0 };
    int.beds.push(crate::buildings::Bed {
        pos: c + Vec3::Y * 0.53,
        head,
        floor: r.floor,
        stand: c + side * sign * 0.85 - head * 0.7,
    });
}

/// Focolare contro la parete di fondo, nella parte libera a destra della scala.
fn hearth_back(b: &mut B, int: &mut Interior, r: &Room) {
    let width = (r.x1 - r.back_x - 0.05).min(1.5);
    if width < 0.9 {
        return;
    }
    let x = r.x1 - width / 2.0;
    let (y, z) = (r.y, r.back_z + 0.28);
    solid(
        b,
        int,
        Vec3::new(x, y + 0.9, z),
        Vec3::new(width, 1.8, 0.55),
        STONE_FIRE,
    );
    b.cuboid_c(
        Vec3::new(x, y + 0.5, z + 0.28),
        Vec3::new(width - 0.5, 0.7, 0.04),
        [0.08, 0.07, 0.07, 1.0],
    );
    b.cuboid_c(
        Vec3::new(x, y + 0.24, z + 0.31),
        Vec3::new(width - 0.8, 0.18, 0.04),
        [0.95, 0.45, 0.12, 1.0],
    );
}

/// Piano di una casa: cucina a terra, camera ai piani. Il corridoio porta-scala resta libero.
fn home_floor(b: &mut B, int: &mut Interior, r: &Room, floor: usize, seed: u32) {
    let y = r.y;
    let tint = seed as usize;
    let mid = r.z(0.5);
    if floor == 0 {
        hearth_back(b, int, r);
        let x = r.x1 - 1.0;
        table(b, int, x, mid, y, 1.6, 0.85);
        bench(b, int, x, mid - 0.68, y, 1.4);
        bench(b, int, x, mid + 0.68, y, 1.4);
        shelf(b, int, r.x0 + 0.75, r.z1 - 0.2, y, 1.3, -1.0);
    } else {
        bed(b, int, r, r.x0 + 0.5, r.z1 - 1.0, Vec3::Z, tint);
        bed(b, int, r, r.x0 + 2.05, r.z1 - 0.5, Vec3::NEG_X, tint + 1);
        if r.x1 - (r.x0 + 3.05) >= 1.1 {
            wardrobe(b, int, r.x0 + 3.6, r.z1 - 0.28, y, -1.0);
        }
        if r.z1 - r.z0 >= 2.2 {
            chest(b, int, r.x0 + 2.0, r.z1 - 1.3, y);
        }
    }
}

/// Sottotetto: casse e botti in fondo, lontano dalla scala.
fn attic(b: &mut B, int: &mut Interior, r: &Room) {
    let y = r.y;
    crate_box(b, int, r.x0 + 0.4, r.z0 + 0.4, y, 0.7);
    barrel(b, int, r.x0 + 0.4, r.z1 - 0.4, y);
    if r.x1 - r.x0 > 1.5 {
        crate_box(b, int, r.x0 + 1.15, r.z1 - 0.32, y, 0.55);
    }
    if r.x1 - r.x0 > 3.0 {
        chest(b, int, r.x0 + 2.3, r.z0 + 0.3, y);
        barrel(b, int, r.x0 + 3.0, r.z1 - 0.4, y);
    }
}

/// Fila di brande contro la parete di fronte.
fn dormitory(b: &mut B, int: &mut Interior, r: &Room, seed: usize) {
    for (k, a) in [0.14, 0.42, 0.7, 0.92].into_iter().enumerate() {
        bed(b, int, r, r.x(a), r.z1 - 1.0, Vec3::Z, seed + k);
    }
}

/// Arreda un piano secondo il tipo di edificio.
pub(crate) fn furnish(
    b: &mut B,
    int: &mut Interior,
    kind: &str,
    r: &Room,
    floor: usize,
    seed: u32,
) {
    let y = r.y;
    match (kind, floor) {
        ("locanda", 0) => {
            // Bancone con sgabelli e tavoli ai lati del passaggio dalla porta.
            solid(
                b,
                int,
                Vec3::new(r.x(0.72), y + 0.55, r.z(0.22)),
                Vec3::new(3.2, 1.1, 0.6),
                WOOD_DARK,
            );
            for k in 0..3 {
                stool(b, r.x(0.58) + k as f32 * 0.7, r.z(0.22) + 0.6, y);
            }
            shelf(b, int, r.x(0.75), r.z0 + 0.2, y, 2.4, 1.0);
            barrel(b, int, r.x(0.95), r.z(0.5), y);
            for (a, c) in [(0.16, 0.4), (0.16, 0.82), (0.82, 0.78)] {
                table(b, int, r.x(a), r.z(c), y, 1.1, 0.85);
                stool(b, r.x(a) - 0.7, r.z(c), y);
                stool(b, r.x(a) + 0.7, r.z(c), y);
            }
        }
        ("locanda", _) => {
            dormitory(b, int, r, seed as usize);
            chest(b, int, r.x(0.55), r.z(0.2), y);
            wardrobe(b, int, r.x(0.8), r.z(0.2), y, 1.0);
        }
        ("caserma", 0) => {
            let x = r.x(0.72);
            table(b, int, x, r.z(0.6), y, 2.2, 0.9);
            bench(b, int, x, r.z(0.6) - 0.68, y, 2.0);
            bench(b, int, x, r.z(0.6) + 0.68, y, 2.0);
            for k in 0..4 {
                // Rastrelliera con lance.
                let x = r.x(0.85) + k as f32 * 0.2 - 0.3;
                b.cuboid_c(
                    Vec3::new(x, y + 1.0, r.z1 - 0.25),
                    Vec3::new(0.04, 2.0, 0.04),
                    WOOD,
                );
                b.cuboid_c(
                    Vec3::new(x, y + 2.04, r.z1 - 0.25),
                    Vec3::new(0.08, 0.18, 0.08),
                    IRON,
                );
            }
            hearth_back(b, int, r);
        }
        ("caserma", _) => {
            dormitory(b, int, r, 0);
            chest(b, int, r.x(0.5), r.z(0.3), y);
        }
        ("fabbro", 0) => {
            solid(
                b,
                int,
                Vec3::new(r.x(0.82), y + 0.5, r.z(0.28)),
                Vec3::new(1.4, 1.0, 1.0),
                STONE_FIRE,
            );
            b.cuboid_c(
                Vec3::new(r.x(0.82), y + 1.02, r.z(0.28)),
                Vec3::new(0.9, 0.06, 0.6),
                [0.95, 0.4, 0.1, 1.0],
            );
            // Incudine su ceppo.
            let (ax, az) = (r.x(0.66), r.z(0.62));
            solid(
                b,
                int,
                Vec3::new(ax, y + 0.36, az),
                Vec3::new(0.5, 0.72, 0.5),
                WOOD_DARK,
            );
            b.cuboid_c(Vec3::new(ax, y + 0.82, az), Vec3::new(0.8, 0.2, 0.3), IRON);
            table(b, int, r.x(0.2), r.z(0.82), y, 1.5, 0.7);
            barrel(b, int, r.x(0.9), r.z(0.75), y);
        }
        ("forno", 0) => {
            solid(
                b,
                int,
                Vec3::new(r.x(0.8), y + 0.8, r.z(0.28)),
                Vec3::new(1.5, 1.6, 1.2),
                STONE_FIRE,
            );
            b.cuboid_c(
                Vec3::new(r.x(0.8), y + 0.7, r.z(0.28) + 0.62),
                Vec3::new(0.6, 0.45, 0.05),
                [0.9, 0.42, 0.12, 1.0],
            );
            table(b, int, r.x(0.8), r.z(0.83), y, 1.4, 0.7);
            shelf(b, int, r.x(0.12), r.z1 - 0.25, y, 1.1, -1.0);
            chest(b, int, r.x0 + 0.45, r.z(0.62), y);
        }
        ("arcere", 0) => {
            for k in 0..3 {
                let x = r.x(0.15) + k as f32 * 0.9;
                // Rastrelliera con archi.
                b.cuboid_c(
                    Vec3::new(x, y + 1.0, r.z1 - 0.2),
                    Vec3::new(0.06, 1.6, 0.06),
                    [0.55, 0.38, 0.2, 1.0],
                );
                b.cuboid_c(
                    Vec3::new(x + 0.14, y + 1.0, r.z1 - 0.2),
                    Vec3::new(0.02, 1.5, 0.02),
                    LINEN,
                );
            }
            table(b, int, r.x(0.78), r.z(0.5), y, 1.5, 0.8);
            barrel(b, int, r.x(0.93), r.z(0.12), y);
        }
        ("stalla", 0) => {
            for k in 0..3 {
                let x = r.x(0.3 + k as f32 * 0.25);
                solid(
                    b,
                    int,
                    Vec3::new(x, y + 0.65, r.z(0.55)),
                    Vec3::new(0.1, 1.3, 1.8),
                    WOOD_DARK,
                );
                b.force = Some(crate::pbr::Mat::Thatch);
                b.cuboid_c(
                    Vec3::new(x + 0.7, y + 0.06, r.z(0.55)),
                    Vec3::new(1.2, 0.12, 1.8),
                    STRAW,
                );
                b.force = None;
            }
        }
        _ => home_floor(b, int, r, floor, seed),
    }
}

/// Sottotetto arredato.
pub(crate) fn furnish_attic(b: &mut B, int: &mut Interior, r: &Room) {
    attic(b, int, r);
}

/// Faro: alloggio del guardiano al piano terra, rampa al primo piano, ripostiglio e chiocciola fino in cima.
pub(crate) fn lighthouse_interior(b: &mut B, int: &mut Interior, y0: f32, y1: f32, top: f32) {
    let rect = (-1.7, 1.7, -1.7, 1.7);
    slab(b, int, rect.0, rect.1, rect.2, rect.3, y0);
    // Rampa lungo la parete destra: il primo gradino fa da pianerottolo fino al muro di fondo.
    let (tx0, tx1) = (0.9, 1.7);
    let n = 11;
    let rise = (y1 - y0) / n as f32;
    let start = -1.45;
    for k in 0..n {
        let za = if k == 0 {
            rect.2
        } else {
            start + k as f32 * RUN
        };
        let zb = start + (k + 1) as f32 * RUN;
        step(b, int, tx0, tx1, za, zb, y0, y0 + (k + 1) as f32 * rise);
    }
    slab_with_hole(b, int, rect, (tx0, tx1, start, start + n as f32 * RUN), y1);

    // Alloggio: i mobili lasciano libero il passaggio dalla porta alla rampa.
    let room = Room {
        x0: -1.6,
        x1: 0.85,
        z0: -1.6,
        z1: 1.6,
        y: y0,
        floor: 0,
        back_z: rect.2,
        back_x: 0.85,
    };
    bed(b, int, &room, -1.12, -0.6, Vec3::NEG_Z, 2);
    shelf(b, int, -1.1, 1.5, y0, 1.0, -1.0);
    table(b, int, 0.55, 0.4, y0, 0.6, 0.6);
    stool(b, 0.55, -0.25, y0);
    crate_box(b, int, 0.62, 1.3, y0, 0.5);

    // Ripostiglio al primo piano, sotto i giri alti della scala.
    chest(b, int, -1.05, 1.28, y1);
    barrel(b, int, -1.2, -1.2, y1);

    // Chiocciola nel pozzo centrale, fino alla stanza della lanterna.
    let shaft = (-0.95, 0.95, -0.95, 0.95);
    let hole = square_helix(b, int, shaft, y1, top, 0.7);
    slab_with_hole(b, int, (-0.97, 0.97, -0.97, 0.97), hole, top);
}

/// Navata della chiesa: panche e altare.
pub(crate) fn furnish_church(b: &mut B, int: &mut Interior, r: &Room) {
    let y = r.y;
    for row in 0..5 {
        let z = r.z(0.32) + row as f32 * 1.35;
        for side in [-1.0f32, 1.0] {
            solid(
                b,
                int,
                Vec3::new(side * 1.75, y + 0.23, z),
                Vec3::new(2.3, 0.46, 0.35),
                WOOD_DARK,
            );
            b.cuboid_c(
                Vec3::new(side * 1.75, y + 0.68, z + 0.16),
                Vec3::new(2.3, 0.5, 0.05),
                WOOD_DARK,
            );
        }
    }
    solid(
        b,
        int,
        Vec3::new(0.0, y + 0.5, r.z0 + 0.9),
        Vec3::new(2.0, 1.0, 0.9),
        STONE_FIRE,
    );
    b.cuboid_c(
        Vec3::new(0.0, y + 1.5, r.z0 + 0.5),
        Vec3::new(0.12, 1.1, 0.12),
        WOOD_DARK,
    );
    b.cuboid_c(
        Vec3::new(0.0, y + 1.72, r.z0 + 0.5),
        Vec3::new(0.7, 0.12, 0.12),
        WOOD_DARK,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top_of(int: &Interior) -> f32 {
        int.ground
            .iter()
            .map(|(c, s)| c.y + s.y / 2.0)
            .fold(f32::MIN, f32::max)
    }

    #[test]
    fn stairs_stay_under_the_step_limit_and_reach_the_top() {
        let mut b = B::default();
        let mut int = Interior::default();
        let ys = [0.42, 2.92, 5.42, 7.94];
        floors_and_stairs(&mut b, &mut int, (-3.28, 3.28, -2.78, 2.78), &ys, 1.84);
        assert!((top_of(&int) - 7.94).abs() < 1e-3);
        let steps: Vec<f32> = int
            .ground
            .iter()
            .filter(|(_, s)| s.y > SLAB + 0.01 || s.y < SLAB - 0.01)
            .map(|(_, s)| s.y)
            .collect();
        assert!(!steps.is_empty());
    }

    #[test]
    fn flights_do_not_overlap_on_the_back_wall() {
        let mut b = B::default();
        let mut int = Interior::default();
        let ys = [0.42, 2.92, 5.42, 7.94];
        let layout = floors_and_stairs(&mut b, &mut int, (-3.28, 3.28, -2.78, 2.78), &ys, 1.84);
        // La prima rampa finisce prima che inizi la seconda, che scende dalla destra.
        assert!(
            layout.back_x <= 3.28 - LANE - 2.43 + 1e-3,
            "{}",
            layout.back_x
        );
        assert!(layout.top_x0 > layout.back_x);
    }

    #[test]
    fn home_floors_register_beds() {
        let mut b = B::default();
        let mut int = Interior::default();
        let room = Room {
            x0: -3.0,
            x1: 3.0,
            z0: -1.0,
            z1: 2.5,
            y: 2.9,
            floor: 1,
            back_z: -2.3,
            back_x: 0.0,
        };
        furnish(&mut b, &mut int, "casa-media", &room, 1, 3);
        assert_eq!(int.beds.len(), 2);
        furnish(&mut b, &mut int, "caserma", &room, 2, 3);
        assert_eq!(int.beds.len(), 6);
    }

    #[test]
    fn helix_reaches_the_top_with_headroom_between_laps() {
        let mut b = B::default();
        let mut int = Interior::default();
        let _ = square_helix(&mut b, &mut int, (-1.45, 1.45, 3.65, 6.75), 0.42, 8.0, 0.8);
        assert!((top_of(&int) - 8.0).abs() < 1e-3);
    }
}
