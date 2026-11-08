//! Edifici del villaggio costruiti a runtime: case di vari tipi, locanda, fabbro, forno,
//! stalla, chiesa con campanile e pozzo.
//!
//! Ogni edificio è **una sola mesh con colori di vertice** (stile cel-shaded, un materiale
//! condiviso) più qualche collider a parallelepipedo. Sistema locale: origine a terra al centro,
//! +Y su, **+Z = davanti** (lato della porta). Le varianti (colori, finestre) dipendono dal seme.

use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Tipi di piazzamento che sono edifici.
pub fn is_building(kind: &str) -> bool {
    matches!(
        kind,
        "casa"
            | "casa-piccola"
            | "casa-media"
            | "casa-grande"
            | "locanda"
            | "caserma"
            | "fabbro"
            | "forno"
            | "stalla"
            | "chiesa"
            | "pozzo"
            | "arcere"
            | "bancarella-erbe"
            | "bancarella-frutta"
            | "bancarella-pesce"
            | "pagliaio"
            | "spaventapasseri"
            | "faro"
            | "molo"
            | "barca"
    )
}

/// Edificio costruito: mesh, collider (centro, dimensioni) e semi-lati dell'impronta.
pub struct Building {
    pub mesh: Mesh,
    /// Manto di neve sui tetti (mostrato solo d'inverno); vuoto se non ci sono tetti.
    pub snow: Mesh,
    pub colliders: Vec<(Vec3, Vec3)>,
    pub half: Vec2,
}

pub(crate) type Col = [f32; 4];

pub(crate) const STONE: Col = [0.74, 0.72, 0.68, 1.0];
pub(crate) const STONE_DARK: Col = [0.5, 0.5, 0.52, 1.0];
const TIMBER: Col = [0.3, 0.2, 0.12, 1.0];
const DOOR: Col = [0.38, 0.24, 0.13, 1.0];
const GLASS: Col = [0.16, 0.22, 0.32, 1.0];
const WALLS: [Col; 5] = [
    [0.93, 0.87, 0.72, 1.0],
    [0.88, 0.72, 0.45, 1.0],
    [0.95, 0.94, 0.9, 1.0],
    [0.9, 0.68, 0.6, 1.0],
    [0.72, 0.8, 0.66, 1.0],
];
pub(crate) const ROOFS: [Col; 3] = [
    [0.72, 0.33, 0.2, 1.0],
    [0.33, 0.36, 0.42, 1.0],
    [0.58, 0.46, 0.27, 1.0],
];
const SHUTTERS: [Col; 4] = [
    [0.2, 0.45, 0.3, 1.0],
    [0.25, 0.4, 0.6, 1.0],
    [0.6, 0.22, 0.2, 1.0],
    [0.3, 0.2, 0.12, 1.0],
];

/// Generatore pseudo-casuale deterministico.
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn pick<T: Copy>(&mut self, v: &[T]) -> T {
        v[self.next() as usize % v.len()]
    }
}

/// Costruttore di mesh con colori di vertice.
#[derive(Default)]
pub(crate) struct B {
    pos: Vec<[f32; 3]>,
    nor: Vec<[f32; 3]>,
    col: Vec<Col>,
    idx: Vec<u32>,
    /// Triangoli del manto di neve (sopra le falde dei tetti).
    snow: Vec<[Vec3; 3]>,
}

impl B {
    /// Aggiunge un triangolo di neve sollevato di 6 cm sopra una falda.
    fn snow_tri(&mut self, a: Vec3, b: Vec3, c: Vec3) {
        let up = Vec3::Y * 0.06;
        self.snow.push([a + up, b + up, c + up]);
    }

    /// Mesh del manto di neve (bianco, normali verso l'alto).
    pub(crate) fn snow_mesh(&self) -> Mesh {
        let mut pos: Vec<[f32; 3]> = Vec::new();
        let mut idx: Vec<u32> = Vec::new();
        for t in &self.snow {
            let base = pos.len() as u32;
            let n = (t[1] - t[0]).cross(t[2] - t[0]);
            let order = if n.y >= 0.0 { [0, 1, 2] } else { [0, 2, 1] };
            for i in order {
                pos.push(t[i].to_array());
            }
            idx.extend([base, base + 1, base + 2]);
        }
        let n = pos.len();
        let mut m = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; n]);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; n]);
        m.insert_indices(Indices::U32(idx));
        m
    }

    /// Quadrilatero; `hint` è la direzione "verso l'esterno" per orientare la faccia.
    pub(crate) fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, hint: Vec3, col: Col) {
        let mut n = (b - a).cross(d - a).normalize_or_zero();
        let pts = if n.dot(hint) < 0.0 {
            n = -n;
            [a, d, c, b]
        } else {
            [a, b, c, d]
        };
        let base = self.pos.len() as u32;
        for p in pts {
            self.pos.push(p.to_array());
            self.nor.push(n.to_array());
            self.col.push(col);
        }
        self.idx
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub(crate) fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, hint: Vec3, col: Col) {
        let mut n = (b - a).cross(c - a).normalize_or_zero();
        let pts = if n.dot(hint) < 0.0 {
            n = -n;
            [a, c, b]
        } else {
            [a, b, c]
        };
        let base = self.pos.len() as u32;
        for p in pts {
            self.pos.push(p.to_array());
            self.nor.push(n.to_array());
            self.col.push(col);
        }
        self.idx.extend([base, base + 1, base + 2]);
    }

    /// Parallelepipedo (sei facce).
    pub(crate) fn cuboid(&mut self, min: Vec3, max: Vec3, col: Col) {
        let (x0, y0, z0) = (min.x, min.y, min.z);
        let (x1, y1, z1) = (max.x, max.y, max.z);
        let v = |x, y, z| Vec3::new(x, y, z);
        self.quad(
            v(x0, y0, z1),
            v(x1, y0, z1),
            v(x1, y1, z1),
            v(x0, y1, z1),
            Vec3::Z,
            col,
        );
        self.quad(
            v(x0, y0, z0),
            v(x1, y0, z0),
            v(x1, y1, z0),
            v(x0, y1, z0),
            Vec3::NEG_Z,
            col,
        );
        self.quad(
            v(x1, y0, z0),
            v(x1, y0, z1),
            v(x1, y1, z1),
            v(x1, y1, z0),
            Vec3::X,
            col,
        );
        self.quad(
            v(x0, y0, z0),
            v(x0, y0, z1),
            v(x0, y1, z1),
            v(x0, y1, z0),
            Vec3::NEG_X,
            col,
        );
        self.quad(
            v(x0, y1, z0),
            v(x1, y1, z0),
            v(x1, y1, z1),
            v(x0, y1, z1),
            Vec3::Y,
            col,
        );
    }

    pub(crate) fn cuboid_c(&mut self, c: Vec3, s: Vec3, col: Col) {
        self.cuboid(c - s / 2.0, c + s / 2.0, col);
    }

    /// Tetto a due falde sul rettangolo [x0,x1]×[z0,z1] con base a `y0`; il colmo corre lungo
    /// X se `ridge_x`, altrimenti lungo Z. I timpani (triangoli) usano `wall`.
    #[allow(clippy::too_many_arguments)]
    fn gable(
        &mut self,
        (x0, x1, z0, z1): (f32, f32, f32, f32),
        y0: f32,
        rh: f32,
        over: f32,
        ridge_x: bool,
        roof: Col,
        wall: Col,
    ) {
        let yr = y0 + rh;
        // (u = asse del colmo, v = asse trasversale)
        let (u0, u1, v0, v1) = if ridge_x {
            (x0, x1, z0, z1)
        } else {
            (z0, z1, x0, x1)
        };
        let vc = (v0 + v1) / 2.0;
        let p = |u: f32, y: f32, v: f32| {
            if ridge_x {
                Vec3::new(u, y, v)
            } else {
                Vec3::new(v, y, u)
            }
        };
        let (a0, a1, b0, b1) = (u0 - over, u1 + over, v0 - over, v1 + over);
        let dir = |sv: f32| {
            if ridge_x {
                Vec3::new(0.0, 1.0, sv)
            } else {
                Vec3::new(sv, 1.0, 0.0)
            }
        };
        for (vs, sv) in [(b1, 1.0), (b0, -1.0)] {
            self.quad(
                p(a0, y0, vs),
                p(a1, y0, vs),
                p(a1, yr, vc),
                p(a0, yr, vc),
                dir(sv),
                roof,
            );
            self.snow_tri(p(a0, y0, vs), p(a1, y0, vs), p(a1, yr, vc));
            self.snow_tri(p(a0, y0, vs), p(a1, yr, vc), p(a0, yr, vc));
        }
        let end = |s: f32| {
            if ridge_x {
                Vec3::new(s, 0.0, 0.0)
            } else {
                Vec3::new(0.0, 0.0, s)
            }
        };
        self.tri(p(u0, y0, v0), p(u0, y0, v1), p(u0, yr, vc), end(-1.0), wall);
        self.tri(p(u1, y0, v0), p(u1, y0, v1), p(u1, yr, vc), end(1.0), wall);
    }

    /// Piramide a base quadrata.
    pub(crate) fn pyramid(&mut self, c: Vec3, half: f32, h: f32, col: Col) {
        let top = c + Vec3::Y * h;
        let pts = [
            c + Vec3::new(-half, 0.0, -half),
            c + Vec3::new(half, 0.0, -half),
            c + Vec3::new(half, 0.0, half),
            c + Vec3::new(-half, 0.0, half),
        ];
        for i in 0..4 {
            let (a, b) = (pts[i], pts[(i + 1) % 4]);
            let out = ((a + b) / 2.0 - c).normalize_or_zero() + Vec3::Y * 0.3;
            self.tri(a, b, top, out, col);
            self.snow_tri(a, b, top);
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Parallelepipedo orientato: `half` = semi-lati (lungo, alto, spesso), `t` = direzione
    /// orizzontale del lato lungo (versore, x e z); `y0..y1` = quote.
    pub(crate) fn obox(
        &mut self,
        c: Vec2,
        y0: f32,
        y1: f32,
        half_len: f32,
        half_th: f32,
        t: Vec2,
        col: Col,
    ) {
        let n = Vec2::new(-t.y, t.x);
        let corner = |a: f32, b: f32| c + t * a + n * b;
        let p = |q: Vec2, y: f32| Vec3::new(q.x, y, q.y);
        let (a, b, cc, d) = (
            corner(-half_len, -half_th),
            corner(half_len, -half_th),
            corner(half_len, half_th),
            corner(-half_len, half_th),
        );
        let hint = |q: Vec2| Vec3::new(q.x - c.x, 0.0, q.y - c.y);
        for (u, v) in [(a, b), (b, cc), (cc, d), (d, a)] {
            let mid = (u + v) / 2.0;
            self.quad(p(u, y0), p(v, y0), p(v, y1), p(u, y1), hint(mid), col);
        }
        self.quad(p(a, y1), p(b, y1), p(cc, y1), p(d, y1), Vec3::Y, col);
        self.snow_tri(p(a, y1), p(b, y1), p(cc, y1));
        self.snow_tri(p(a, y1), p(cc, y1), p(d, y1));
    }

    pub(crate) fn finish(self) -> Mesh {
        let mut m = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        let n = self.pos.len();
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.nor);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; n]);
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.col);
        m.insert_indices(Indices::U32(self.idx));
        m
    }
}

/// Finestra sul fronte (`z_face`, normale `n` = ±Z o ±X) con imposte.
fn window(b: &mut B, c: Vec3, n: Vec3, shutter: Col) {
    let t = Vec3::Y.cross(n).normalize(); // verso destra guardando la finestra
    let (w, h) = (0.62, 0.86);
    let o = n * 0.03;
    let (l, r) = (c - t * (w / 2.0), c + t * (w / 2.0));
    b.quad(
        l - Vec3::Y * (h / 2.0) + o,
        r - Vec3::Y * (h / 2.0) + o,
        r + Vec3::Y * (h / 2.0) + o,
        l + Vec3::Y * (h / 2.0) + o,
        n,
        GLASS,
    );
    // Davanzale e architrave di pietra.
    b.cuboid_c(
        c - Vec3::Y * (h / 2.0 + 0.04) + n * 0.06,
        t.abs() * (w + 0.3) + n.abs() * 0.16 + Vec3::Y * 0.08,
        STONE,
    );
    // Imposte ai lati.
    for s in [-1.0, 1.0] {
        let sc = c + t * (s * (w / 2.0 + 0.18)) + n * 0.05;
        b.cuboid_c(
            sc,
            t.abs() * 0.3 + n.abs() * 0.05 + Vec3::Y * (h + 0.04),
            shutter,
        );
    }
}

/// Porta sul fronte (+Z) con gradino.
fn door_front(b: &mut B, x: f32, z: f32, w: f32, h: f32) {
    b.cuboid(
        Vec3::new(x - w / 2.0 - 0.1, 0.0, z - 0.02),
        Vec3::new(x + w / 2.0 + 0.1, h + 0.1, z + 0.08),
        TIMBER,
    );
    b.cuboid(
        Vec3::new(x - w / 2.0, 0.0, z),
        Vec3::new(x + w / 2.0, h, z + 0.12),
        DOOR,
    );
    b.cuboid(
        Vec3::new(x - w / 2.0 - 0.2, 0.0, z),
        Vec3::new(x + w / 2.0 + 0.2, 0.18, z + 0.55),
        STONE,
    );
}

fn chimney(b: &mut B, x: f32, z: f32, y0: f32, y1: f32) {
    b.cuboid(
        Vec3::new(x - 0.35, y0, z - 0.35),
        Vec3::new(x + 0.35, y1, z + 0.35),
        STONE_DARK,
    );
    b.cuboid(
        Vec3::new(x - 0.45, y1, z - 0.45),
        Vec3::new(x + 0.45, y1 + 0.15, z + 0.45),
        STONE,
    );
}

/// Casa a base rettangolare `w`×`d`, muri alti `wh`, `floors` piani di finestre.
#[allow(clippy::too_many_arguments)]
fn house(
    b: &mut B,
    r: &mut Rng,
    w: f32,
    d: f32,
    wh: f32,
    floors: usize,
    timber: bool,
    chimneys: usize,
) {
    let wall = r.pick(&WALLS);
    let roof = r.pick(&ROOFS);
    let shutter = r.pick(&SHUTTERS);
    let (hw, hd) = (w / 2.0, d / 2.0);
    // Zoccolo di pietra che affonda nel terreno (compensa le pendenze).
    b.cuboid(
        Vec3::new(-hw - 0.12, -1.3, -hd - 0.12),
        Vec3::new(hw + 0.12, 0.4, hd + 0.12),
        STONE,
    );
    b.cuboid(Vec3::new(-hw, 0.4, -hd), Vec3::new(hw, wh, hd), wall);
    if timber {
        // Travi a vista: montanti d'angolo e fasce orizzontali.
        for (x, z) in [(-hw, -hd), (hw, -hd), (-hw, hd), (hw, hd)] {
            b.cuboid_c(
                Vec3::new(x, wh / 2.0 + 0.2, z),
                Vec3::new(0.2, wh - 0.4, 0.2),
                TIMBER,
            );
        }
        for y in [wh * 0.52, wh - 0.1] {
            b.cuboid_c(
                Vec3::new(0.0, y, hd + 0.01),
                Vec3::new(w + 0.1, 0.16, 0.12),
                TIMBER,
            );
            b.cuboid_c(
                Vec3::new(0.0, y, -hd - 0.01),
                Vec3::new(w + 0.1, 0.16, 0.12),
                TIMBER,
            );
        }
    }
    let ridge_x = w >= d;
    let rh = (if ridge_x { d } else { w }) * 0.55;
    b.gable((-hw, hw, -hd, hd), wh, rh, 0.4, ridge_x, roof, wall);
    // Porta e finestre sul fronte.
    let door_x = if floors > 1 { 0.0 } else { -hw * 0.35 };
    door_front(b, door_x, hd, 0.95, 1.95);
    let per_floor = ((w - 1.2) / 2.2).floor().max(1.0) as usize;
    for f in 0..floors {
        let y = 1.45 + f as f32 * 2.5;
        if y + 0.5 > wh {
            break;
        }
        for i in 0..per_floor {
            let x = -hw + 0.9 + (w - 1.8) * (i as f32 + 0.5) / per_floor as f32;
            if f == 0 && (x - door_x).abs() < 1.1 {
                continue;
            }
            window(b, Vec3::new(x, y, hd), Vec3::Z, shutter);
        }
    }
    // Finestre sul retro e sui fianchi.
    window(b, Vec3::new(0.0, 1.5, -hd), Vec3::NEG_Z, shutter);
    window(b, Vec3::new(hw, 1.5, 0.0), Vec3::X, shutter);
    window(b, Vec3::new(-hw, 1.5, 0.0), Vec3::NEG_X, shutter);
    // Camini.
    for i in 0..chimneys {
        let x = if ridge_x {
            hw * (if i == 0 { 0.55 } else { -0.55 })
        } else {
            0.0
        };
        let z = if ridge_x {
            0.0
        } else {
            hd * (if i == 0 { 0.55 } else { -0.55 })
        };
        chimney(b, x, z, wh + rh * 0.4, wh + rh + 0.9);
    }
}

fn body_colliders(w: f32, d: f32, wh: f32) -> Vec<(Vec3, Vec3)> {
    vec![
        (
            Vec3::new(0.0, wh / 2.0 - 0.3, 0.0),
            Vec3::new(w + 0.2, wh + 1.2, d + 0.2),
        ),
        (Vec3::new(0.0, wh + 0.9, 0.0), Vec3::new(w, 1.8, d)),
    ]
}

/// Chiesa: navata (larghezza 7, lunghezza 11), abside, campanile sul fronte e portale.
fn church(b: &mut B, r: &mut Rng) -> Vec<(Vec3, Vec3)> {
    let wall = [0.93, 0.9, 0.82, 1.0];
    let roof = ROOFS[1];
    let (hw, wh) = (3.5, 5.4);
    let (z_back, z_front) = (-7.5, 3.5); // navata
    b.cuboid(
        Vec3::new(-hw - 0.15, -1.4, z_back - 0.15),
        Vec3::new(hw + 0.15, 0.5, z_front + 0.15),
        STONE,
    );
    b.cuboid(
        Vec3::new(-hw, 0.5, z_back),
        Vec3::new(hw, wh, z_front),
        wall,
    );
    b.gable((-hw, hw, z_back, z_front), wh, 3.2, 0.5, false, roof, wall);
    // Abside poligonale (più bassa).
    b.cuboid(
        Vec3::new(-2.2, -1.4, z_back - 2.6),
        Vec3::new(2.2, 0.5, z_back + 0.2),
        STONE,
    );
    b.cuboid(
        Vec3::new(-2.0, 0.5, z_back - 2.4),
        Vec3::new(2.0, 4.0, z_back + 0.1),
        wall,
    );
    b.gable(
        (-2.0, 2.0, z_back - 2.4, z_back + 0.1),
        4.0,
        1.8,
        0.3,
        false,
        roof,
        wall,
    );
    // Campanile sul fronte.
    let tz = z_front + 1.7;
    b.cuboid(
        Vec3::new(-1.8, -1.5, tz - 1.9),
        Vec3::new(1.8, 0.5, tz + 1.9),
        STONE,
    );
    b.cuboid(
        Vec3::new(-1.7, 0.5, tz - 1.8),
        Vec3::new(1.7, 10.4, tz + 1.8),
        wall,
    );
    // Cella campanaria con aperture scure e cuspide.
    for s in [-1.0f32, 1.0] {
        b.cuboid_c(
            Vec3::new(0.0, 9.3, tz + s * 1.82),
            Vec3::new(1.1, 1.5, 0.08),
            GLASS,
        );
        b.cuboid_c(
            Vec3::new(s * 1.72, 9.3, tz),
            Vec3::new(0.08, 1.5, 1.1),
            GLASS,
        );
    }
    b.cuboid(
        Vec3::new(-1.85, 10.4, tz - 1.95),
        Vec3::new(1.85, 10.7, tz + 1.95),
        STONE,
    );
    b.pyramid(Vec3::new(0.0, 10.7, tz), 1.9, 5.2, roof);
    // Croce sulla cuspide.
    b.cuboid_c(Vec3::new(0.0, 16.4, tz), Vec3::new(0.12, 1.1, 0.12), TIMBER);
    b.cuboid_c(Vec3::new(0.0, 16.5, tz), Vec3::new(0.6, 0.12, 0.12), TIMBER);
    // Portale (sul fronte del campanile, +Z) e orologio.
    door_front(b, 0.0, tz + 1.8, 1.7, 2.8);
    b.cuboid_c(
        Vec3::new(0.0, 6.8, tz + 1.82),
        Vec3::new(1.0, 1.0, 0.08),
        [0.9, 0.85, 0.6, 1.0],
    );
    // Finestre alte e strette sui fianchi della navata.
    for i in 0..3 {
        let z = z_back + 2.0 + i as f32 * 3.0;
        for s in [-1.0f32, 1.0] {
            b.cuboid_c(
                Vec3::new(s * (hw + 0.02), 3.2, z),
                Vec3::new(0.06, 2.0, 0.8),
                GLASS,
            );
        }
    }
    let _ = r.next();
    vec![
        (
            Vec3::new(0.0, 2.6, (z_back + z_front) / 2.0),
            Vec3::new(7.4, 6.4, z_front - z_back + 0.4),
        ),
        (
            Vec3::new(0.0, 5.6, (z_back + z_front) / 2.0),
            Vec3::new(7.0, 3.0, z_front - z_back),
        ),
        (Vec3::new(0.0, 2.0, z_back - 1.2), Vec3::new(4.2, 4.2, 2.6)),
        (Vec3::new(0.0, 5.2, tz), Vec3::new(3.8, 11.4, 4.0)),
    ]
}

/// Insegna a bandiera accanto alla porta (bottega).
fn sign_arm(b: &mut B, col: Col) {
    b.cuboid_c(Vec3::new(1.9, 3.1, 3.0), Vec3::new(0.08, 0.08, 1.0), TIMBER);
    b.cuboid_c(Vec3::new(1.9, 2.65, 3.45), Vec3::new(0.08, 0.7, 0.9), col);
    b.cuboid_c(
        Vec3::new(1.9, 2.65, 3.5),
        Vec3::new(0.1, 0.1, 0.55),
        [0.95, 0.9, 0.7, 1.0],
    );
}

/// Bancarella di mercato: bancone, tendone a strisce, montanti e merce (`kind` dà il tipo).
fn stall(b: &mut B, kind: &str) -> Vec<(Vec3, Vec3)> {
    let (c1, goods): (Col, Col) = match kind {
        "bancarella-frutta" => ([0.78, 0.2, 0.18, 1.0], [0.85, 0.2, 0.2, 1.0]),
        "bancarella-pesce" => ([0.25, 0.45, 0.75, 1.0], [0.75, 0.8, 0.85, 1.0]),
        _ => ([0.25, 0.55, 0.3, 1.0], [0.4, 0.7, 0.3, 1.0]),
    };
    let c2: Col = [0.95, 0.93, 0.85, 1.0];
    // Zoccolo e bancone.
    b.cuboid(
        Vec3::new(-1.5, -1.0, -1.2),
        Vec3::new(1.5, 0.1, 1.2),
        STONE_DARK,
    );
    b.cuboid(Vec3::new(-1.4, 0.0, 0.0), Vec3::new(1.4, 0.95, 0.9), TIMBER);
    b.cuboid(
        Vec3::new(-1.45, 0.95, -0.05),
        Vec3::new(1.45, 1.02, 0.95),
        [0.55, 0.38, 0.22, 1.0],
    );
    // Montanti.
    for (x, z, h) in [
        (-1.4, 0.95, 2.2),
        (1.4, 0.95, 2.2),
        (-1.4, -1.1, 2.7),
        (1.4, -1.1, 2.7),
    ] {
        b.cuboid_c(Vec3::new(x, h / 2.0, z), Vec3::new(0.1, h, 0.1), TIMBER);
    }
    // Tendone inclinato a strisce alternate.
    let n = 7;
    for i in 0..n {
        let (x0, x1) = (
            -1.6 + 3.2 * i as f32 / n as f32,
            -1.6 + 3.2 * (i + 1) as f32 / n as f32,
        );
        let col = if i % 2 == 0 { c1 } else { c2 };
        b.quad(
            Vec3::new(x0, 2.7, -1.3),
            Vec3::new(x1, 2.7, -1.3),
            Vec3::new(x1, 2.2, 1.35),
            Vec3::new(x0, 2.2, 1.35),
            Vec3::Y,
            col,
        );
    }
    // Merce sul bancone e casse dietro.
    for i in 0..6 {
        let x = -1.1 + 0.44 * i as f32;
        b.cuboid_c(Vec3::new(x, 1.12, 0.45), Vec3::new(0.28, 0.2, 0.28), goods);
    }
    b.cuboid_c(
        Vec3::new(-0.9, 0.3, -0.8),
        Vec3::new(0.7, 0.6, 0.6),
        [0.55, 0.4, 0.25, 1.0],
    );
    b.cuboid_c(
        Vec3::new(0.9, 0.3, -0.8),
        Vec3::new(0.7, 0.6, 0.6),
        [0.5, 0.36, 0.22, 1.0],
    );
    vec![
        (Vec3::new(0.0, 0.5, 0.45), Vec3::new(2.9, 1.1, 0.95)),
        (Vec3::new(0.0, 0.5, -0.8), Vec3::new(2.9, 1.1, 0.7)),
    ]
}

/// Faro: zoccolo di pietra, torre a fasce bianche e rosse, balconata, lanterna e cupola.
fn lighthouse(b: &mut B) -> Vec<(Vec3, Vec3)> {
    let (white, red) = ([0.95, 0.93, 0.88, 1.0], [0.78, 0.18, 0.15, 1.0]);
    b.cuboid(Vec3::new(-2.6, -1.8, -2.6), Vec3::new(2.6, 0.6, 2.6), STONE);
    b.cuboid(
        Vec3::new(-2.2, 0.6, -2.2),
        Vec3::new(2.2, 1.0, 2.2),
        STONE_DARK,
    );
    let mut y = 1.0;
    for i in 0..5 {
        let hw = 1.95 - i as f32 * 0.14;
        b.cuboid(
            Vec3::new(-hw, y, -hw),
            Vec3::new(hw, y + 2.6, hw),
            if i % 2 == 0 { white } else { red },
        );
        y += 2.6;
    }
    // Porta e finestrelle.
    door_front(b, 0.0, 1.95, 0.95, 1.9);
    for (i, h) in [3.4f32, 6.0, 8.6, 11.2].iter().enumerate() {
        let n = if i % 2 == 0 { Vec3::X } else { Vec3::NEG_X };
        b.cuboid_c(
            Vec3::new(n.x * (1.78 - i as f32 * 0.1), *h, 0.0),
            Vec3::new(0.08, 0.6, 0.4),
            GLASS,
        );
    }
    // Balconata con ringhiera.
    b.cuboid(
        Vec3::new(-2.3, y, -2.3),
        Vec3::new(2.3, y + 0.25, 2.3),
        STONE_DARK,
    );
    for (x, z) in [
        (-2.2, -2.2),
        (2.2, -2.2),
        (-2.2, 2.2),
        (2.2, 2.2),
        (0.0, -2.2),
        (0.0, 2.2),
        (-2.2, 0.0),
        (2.2, 0.0),
    ] {
        b.cuboid_c(Vec3::new(x, y + 0.75, z), Vec3::new(0.1, 1.0, 0.1), TIMBER);
    }
    for (cx, cz, sx, sz) in [
        (0.0, -2.2, 4.5, 0.1),
        (0.0, 2.2, 4.5, 0.1),
        (-2.2, 0.0, 0.1, 4.5),
        (2.2, 0.0, 0.1, 4.5),
    ] {
        b.cuboid_c(Vec3::new(cx, y + 1.25, cz), Vec3::new(sx, 0.08, sz), TIMBER);
    }
    // Lanterna (vetri chiari), cupola e parafulmine.
    b.cuboid(
        Vec3::new(-0.95, y + 0.25, -0.95),
        Vec3::new(0.95, y + 2.05, 0.95),
        [1.0, 0.92, 0.62, 1.0],
    );
    for (x, z) in [(-0.95, -0.95), (0.95, -0.95), (-0.95, 0.95), (0.95, 0.95)] {
        b.cuboid_c(
            Vec3::new(x, y + 1.15, z),
            Vec3::new(0.12, 1.8, 0.12),
            TIMBER,
        );
    }
    b.cuboid(
        Vec3::new(-1.15, y + 2.05, -1.15),
        Vec3::new(1.15, y + 2.25, 1.15),
        STONE_DARK,
    );
    b.pyramid(Vec3::new(0.0, y + 2.25, 0.0), 1.2, 1.4, red);
    b.cuboid_c(
        Vec3::new(0.0, y + 4.1, 0.0),
        Vec3::new(0.06, 1.0, 0.06),
        TIMBER,
    );
    vec![
        (Vec3::new(0.0, 5.5, 0.0), Vec3::new(4.2, 12.0, 4.2)),
        (Vec3::new(0.0, -0.6, 0.0), Vec3::new(5.2, 2.4, 5.2)),
    ]
}

/// Molo di legno (si stende verso +Z per 14 m a partire da z = -2): tavole, pali, parapetti, casse.
/// L'origine è a livello del mare; il primo collider è il piano calpestabile.
/// Quota (m) del piano del molo rispetto al livello del mare.
pub const PIER_DECK_TOP: f32 = 0.55;

fn pier(b: &mut B) -> Vec<(Vec3, Vec3)> {
    let deck_y = PIER_DECK_TOP;
    // Tavole a strisce alternate.
    let n = 31;
    for i in 0..n {
        let z0 = -2.0 + 14.0 * i as f32 / n as f32;
        let z1 = -2.0 + 14.0 * (i + 1) as f32 / n as f32;
        let col: Col = if i % 2 == 0 {
            [0.55, 0.4, 0.25, 1.0]
        } else {
            [0.5, 0.36, 0.22, 1.0]
        };
        b.cuboid(
            Vec3::new(-1.2, deck_y - 0.12, z0 + 0.01),
            Vec3::new(1.2, deck_y, z1 - 0.01),
            col,
        );
    }
    // Travi longitudinali e pali.
    for x in [-1.0f32, 1.0] {
        b.cuboid(
            Vec3::new(x - 0.1, deck_y - 0.35, -2.0),
            Vec3::new(x + 0.1, deck_y - 0.12, 12.0),
            TIMBER,
        );
    }
    for i in 0..6 {
        let z = -1.0 + i as f32 * 2.6;
        for x in [-1.3f32, 1.3] {
            b.cuboid_c(Vec3::new(x, -1.2, z), Vec3::new(0.24, 4.6, 0.24), TIMBER);
            b.cuboid_c(
                Vec3::new(x, deck_y + 0.55, z),
                Vec3::new(0.12, 1.1, 0.12),
                TIMBER,
            );
        }
    }
    for x in [-1.3f32, 1.3] {
        b.cuboid_c(
            Vec3::new(x, deck_y + 0.95, 5.0),
            Vec3::new(0.08, 0.08, 14.0),
            TIMBER,
        );
        b.cuboid_c(
            Vec3::new(x, deck_y + 0.5, 5.0),
            Vec3::new(0.08, 0.08, 14.0),
            TIMBER,
        );
    }
    // Bitte di ormeggio, casse e un barile.
    for (x, z) in [(-1.1, 4.0), (1.1, 8.0), (-1.1, 11.4)] {
        b.cuboid_c(
            Vec3::new(x, deck_y + 0.18, z),
            Vec3::new(0.2, 0.36, 0.2),
            [0.2, 0.2, 0.22, 1.0],
        );
    }
    b.cuboid_c(
        Vec3::new(-0.7, deck_y + 0.3, 1.0),
        Vec3::new(0.7, 0.6, 0.7),
        [0.6, 0.45, 0.28, 1.0],
    );
    b.cuboid_c(
        Vec3::new(-0.6, deck_y + 0.9, 1.0),
        Vec3::new(0.5, 0.5, 0.5),
        [0.55, 0.4, 0.25, 1.0],
    );
    b.cuboid_c(
        Vec3::new(0.75, deck_y + 0.4, 2.4),
        Vec3::new(0.55, 0.8, 0.55),
        [0.45, 0.3, 0.18, 1.0],
    );
    vec![
        (
            Vec3::new(0.0, deck_y - 0.15, 5.0),
            Vec3::new(2.4, 0.3, 14.0),
        ),
        (
            Vec3::new(-1.3, deck_y + 0.6, 5.0),
            Vec3::new(0.14, 1.2, 14.0),
        ),
        (
            Vec3::new(1.3, deck_y + 0.6, 5.0),
            Vec3::new(0.14, 1.2, 14.0),
        ),
    ]
}

/// Barca a remi (si stende lungo Z, prua verso +Z).
fn boat(b: &mut B, r: &mut Rng) {
    let hull: Col = r.pick(&[
        [0.45, 0.3, 0.18, 1.0],
        [0.3, 0.42, 0.55, 1.0],
        [0.6, 0.25, 0.2, 1.0],
    ]);
    let wood: Col = [0.55, 0.4, 0.25, 1.0];
    b.cuboid(
        Vec3::new(-0.55, -0.25, -1.4),
        Vec3::new(0.55, 0.45, 1.0),
        hull,
    );
    // Prua a punta e poppa dritta.
    b.cuboid(
        Vec3::new(-0.38, -0.2, 1.0),
        Vec3::new(0.38, 0.45, 1.4),
        hull,
    );
    b.cuboid(
        Vec3::new(-0.18, -0.15, 1.4),
        Vec3::new(0.18, 0.5, 1.75),
        hull,
    );
    // Interno: fondo e sedili.
    b.cuboid(
        Vec3::new(-0.45, 0.4, -1.3),
        Vec3::new(0.45, 0.46, 0.95),
        [0.35, 0.25, 0.15, 1.0],
    );
    for z in [-0.6f32, 0.2] {
        b.cuboid_c(Vec3::new(0.0, 0.58, z), Vec3::new(1.05, 0.08, 0.3), wood);
    }
    // Remi.
    for x in [-0.8f32, 0.8] {
        b.cuboid_c(Vec3::new(x, 0.75, -0.2), Vec3::new(0.05, 0.05, 1.7), wood);
    }
}

/// Pozzo: anello di pietra, due montanti, piccolo tetto e secchio.
fn well(b: &mut B) -> Vec<(Vec3, Vec3)> {
    b.cuboid(Vec3::new(-1.2, -1.0, -1.2), Vec3::new(1.2, 0.9, 1.2), STONE);
    b.cuboid(
        Vec3::new(-0.8, 0.9, -0.8),
        Vec3::new(0.8, 0.95, 0.8),
        [0.1, 0.25, 0.4, 1.0],
    );
    for s in [-1.0f32, 1.0] {
        b.cuboid_c(
            Vec3::new(s * 1.0, 1.8, 0.0),
            Vec3::new(0.16, 1.9, 0.16),
            TIMBER,
        );
    }
    b.cuboid_c(
        Vec3::new(0.0, 1.85, 0.0),
        Vec3::new(0.12, 0.12, 2.0),
        TIMBER,
    );
    b.gable(
        (-1.3, 1.3, -1.0, 1.0),
        2.75,
        0.8,
        0.2,
        false,
        ROOFS[0],
        WALLS[0],
    );
    b.cuboid_c(Vec3::new(0.0, 1.35, 0.0), Vec3::new(0.3, 0.3, 0.3), TIMBER);
    vec![(Vec3::new(0.0, 0.5, 0.0), Vec3::new(2.4, 2.0, 2.4))]
}

/// Costruisce l'edificio `kind` con la variante data dal `seed`.
pub fn model(kind: &str, seed: u32) -> Building {
    let mut b = B::default();
    let mut r = Rng(seed | 1);
    for _ in 0..3 {
        r.next();
    }
    let (w, d, colliders) = match kind {
        "casa-piccola" => {
            house(&mut b, &mut r, 5.0, 4.0, 2.7, 1, false, 1);
            (5.0, 4.0, body_colliders(5.0, 4.0, 2.7))
        }
        "casa-grande" => {
            house(&mut b, &mut r, 7.0, 6.0, 5.4, 2, true, 2);
            (7.0, 6.0, body_colliders(7.0, 6.0, 5.4))
        }
        "locanda" => {
            house(&mut b, &mut r, 10.0, 6.5, 5.2, 2, true, 2);
            // Insegna a bandiera accanto alla porta.
            b.cuboid_c(Vec3::new(1.6, 3.4, 3.4), Vec3::new(0.08, 0.08, 0.9), TIMBER);
            b.cuboid_c(
                Vec3::new(1.6, 3.0, 3.8),
                Vec3::new(0.7, 0.55, 0.06),
                [0.55, 0.3, 0.15, 1.0],
            );
            (10.0, 6.5, body_colliders(10.0, 6.5, 5.2))
        }
        "caserma" => {
            // Casa delle guardie: due piani, travi a vista, stendardo rosso accanto alla porta.
            house(&mut b, &mut r, 8.0, 6.0, 5.2, 2, true, 1);
            b.cuboid_c(Vec3::new(1.9, 3.6, 3.4), Vec3::new(0.08, 0.08, 0.9), TIMBER);
            b.cuboid_c(
                Vec3::new(1.9, 3.1, 3.8),
                Vec3::new(0.7, 0.9, 0.06),
                [0.7, 0.14, 0.12, 1.0],
            );
            (8.0, 6.0, body_colliders(8.0, 6.0, 5.2))
        }
        "fabbro" => {
            house(&mut b, &mut r, 6.5, 5.0, 3.0, 1, false, 0);
            sign_arm(&mut b, [0.35, 0.36, 0.4, 1.0]);
            chimney(&mut b, 2.3, -1.2, 2.6, 5.6);
            b.cuboid(
                Vec3::new(3.25, 0.0, -1.5),
                Vec3::new(5.0, 2.3, 2.0),
                [0.45, 0.38, 0.32, 1.0],
            );
            let mut c = body_colliders(6.5, 5.0, 3.0);
            c.push((Vec3::new(4.1, 1.1, 0.25), Vec3::new(1.8, 2.4, 3.5)));
            (10.0, 5.0, c)
        }
        "forno" => {
            house(&mut b, &mut r, 5.5, 4.5, 2.8, 1, false, 0);
            chimney(&mut b, 1.4, 0.0, 2.8, 5.2);
            (5.5, 4.5, body_colliders(5.5, 4.5, 2.8))
        }
        "stalla" => {
            house(&mut b, &mut r, 9.0, 5.0, 2.4, 1, false, 0);
            b.cuboid(
                Vec3::new(-1.4, 0.0, 2.52),
                Vec3::new(1.4, 2.1, 2.64),
                TIMBER,
            );
            (9.0, 5.0, body_colliders(9.0, 5.0, 2.4))
        }
        "arcere" => {
            house(&mut b, &mut r, 6.2, 5.0, 3.5, 2, true, 1);
            sign_arm(&mut b, [0.25, 0.5, 0.3, 1.0]);
            (6.2, 5.0, body_colliders(6.2, 5.0, 3.5))
        }
        "bancarella-erbe" | "bancarella-frutta" | "bancarella-pesce" => {
            let c = stall(&mut b, kind);
            (3.0, 2.8, c)
        }
        "faro" => {
            let c = lighthouse(&mut b);
            (5.2, 5.2, c)
        }
        "molo" => {
            let c = pier(&mut b);
            (3.2, 14.0, c)
        }
        "barca" => {
            boat(&mut b, &mut r);
            (1.3, 3.2, Vec::new())
        }
        "pagliaio" => {
            b.cuboid(
                Vec3::new(-0.9, -0.2, -0.9),
                Vec3::new(0.9, 1.1, 0.9),
                [0.85, 0.7, 0.3, 1.0],
            );
            b.pyramid(Vec3::new(0.0, 1.1, 0.0), 1.15, 1.4, [0.8, 0.62, 0.22, 1.0]);
            (
                1.8,
                1.8,
                vec![(Vec3::new(0.0, 0.8, 0.0), Vec3::new(1.8, 1.8, 1.8))],
            )
        }
        "spaventapasseri" => {
            b.cuboid_c(Vec3::new(0.0, 0.9, 0.0), Vec3::new(0.08, 2.0, 0.08), TIMBER);
            b.cuboid_c(
                Vec3::new(0.0, 1.45, 0.0),
                Vec3::new(1.2, 0.08, 0.08),
                TIMBER,
            );
            b.cuboid_c(
                Vec3::new(0.0, 1.25, 0.04),
                Vec3::new(0.5, 0.6, 0.12),
                [0.55, 0.42, 0.3, 1.0],
            );
            b.cuboid_c(
                Vec3::new(0.0, 1.85, 0.0),
                Vec3::new(0.3, 0.3, 0.3),
                [0.85, 0.75, 0.5, 1.0],
            );
            b.cuboid_c(
                Vec3::new(0.0, 2.08, 0.0),
                Vec3::new(0.5, 0.06, 0.5),
                [0.35, 0.25, 0.15, 1.0],
            );
            b.cuboid_c(
                Vec3::new(0.0, 2.22, 0.0),
                Vec3::new(0.28, 0.24, 0.28),
                [0.35, 0.25, 0.15, 1.0],
            );
            (
                0.6,
                0.6,
                vec![(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.3, 2.0, 0.3))],
            )
        }
        "chiesa" => {
            let c = church(&mut b, &mut r);
            (7.4, 17.0, c)
        }
        "pozzo" => (2.4, 2.4, well(&mut b)),
        // "casa" e "casa-media"
        _ => {
            let timber = r.next().is_multiple_of(2);
            house(&mut b, &mut r, 6.2, 5.0, 3.5, 2, timber, 1);
            (6.2, 5.0, body_colliders(6.2, 5.0, 3.5))
        }
    };
    Building {
        snow: b.snow_mesh(),
        mesh: b.finish(),
        colliders,
        half: Vec2::new(w / 2.0, d / 2.0),
    }
}

/// Mesh unica di un campo coltivato: una zolla di terra e una pianta per ogni punto
/// (`kind` = "campo" grano, "orto" cavoli). `height` dà la quota del terreno in un punto.
pub fn crop_field_mesh(kind: &str, pts: &[Vec2], height: &dyn Fn(Vec2) -> f32) -> Mesh {
    let mut b = B::default();
    let soil: Col = [0.4, 0.28, 0.17, 1.0];
    for (i, p) in pts.iter().enumerate() {
        let y = height(*p);
        let j = |k: u32| {
            ((i as u32).wrapping_mul(2_654_435_761u32).rotate_left(k) % 100) as f32 / 100.0 - 0.5
        };
        b.cuboid(
            Vec3::new(p.x - 0.4, y - 0.1, p.y - 0.4),
            Vec3::new(p.x + 0.4, y + 0.05, p.y + 0.4),
            soil,
        );
        if kind == "orto" {
            b.cuboid_c(
                Vec3::new(p.x, y + 0.08, p.y),
                Vec3::new(0.55, 0.05, 0.55),
                [0.22, 0.5, 0.22, 1.0],
            );
            b.cuboid_c(
                Vec3::new(p.x, y + 0.24, p.y),
                Vec3::new(0.34, 0.3, 0.34),
                [0.4, 0.7, 0.32, 1.0],
            );
        } else {
            for (dx, dz) in [
                (-0.2, -0.2),
                (0.2, -0.2),
                (-0.2, 0.2),
                (0.2, 0.2),
                (0.0, 0.0),
            ] {
                let (x, z) = (p.x + dx + j(3) * 0.08, p.y + dz + j(7) * 0.08);
                let h = 0.65 + (j(11) + 0.5) * 0.15;
                b.cuboid_c(
                    Vec3::new(x, y + h / 2.0, z),
                    Vec3::new(0.035, h, 0.035),
                    [0.62, 0.62, 0.22, 1.0],
                );
                b.cuboid_c(
                    Vec3::new(x, y + h + 0.07, z),
                    Vec3::new(0.07, 0.17, 0.07),
                    [0.88, 0.72, 0.2, 1.0],
                );
            }
        }
    }
    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ogni tipo che ha un modello deve essere riconosciuto come edificio (e viceversa).
    #[test]
    fn every_building_kind_has_a_model() {
        for k in [
            "casa",
            "casa-piccola",
            "casa-media",
            "casa-grande",
            "locanda",
            "caserma",
            "fabbro",
            "forno",
            "stalla",
            "chiesa",
            "pozzo",
            "arcere",
            "bancarella-erbe",
            "bancarella-frutta",
            "bancarella-pesce",
            "pagliaio",
            "spaventapasseri",
            "faro",
            "molo",
            "barca",
        ] {
            assert!(is_building(k), "{k} non riconosciuto");
            let m = model(k, 7);
            assert!(m.mesh.count_vertices() > 8, "{k} senza mesh");
        }
    }
}
