//! Edifici del villaggio costruiti a runtime: case di vari tipi, locanda, fabbro, forno,
//! stalla, chiesa con campanile e pozzo.
//!
//! Ogni edificio è **una sola mesh con colori di vertice** (stile cel-shaded, un materiale
//! condiviso) più qualche collider a parallelepipedo. Sistema locale: origine a terra al centro,
//! +Y su, **+Z = davanti** (lato della porta). Le varianti (colori, finestre) dipendono dal seme.

use crate::pbr::{MATS, Mat};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Semi-lati dell'impronta degli edifici visitabili (`None` per gli altri).
pub fn interior_half(kind: &str) -> Option<Vec2> {
    Some(match kind {
        "casa-piccola" => Vec2::new(2.5, 2.0),
        "casa-grande" => Vec2::new(3.5, 3.0),
        "locanda" => Vec2::new(5.0, 3.25),
        "caserma" => Vec2::new(4.0, 3.0),
        "fabbro" => Vec2::new(5.0, 2.5),
        "forno" => Vec2::new(2.75, 2.25),
        "stalla" => Vec2::new(4.5, 2.5),
        "arcere" | "casa" | "casa-media" => Vec2::new(3.1, 2.5),
        "chiesa" => Vec2::new(3.7, 8.5),
        "faro" => Vec2::new(2.6, 2.6),
        _ => return None,
    })
}

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
            | "carretto"
            | "botti"
            | "casse"
            | "panni"
            | "reti"
    )
}

/// Edificio costruito: mesh, collider (centro, dimensioni) e semi-lati dell'impronta.
pub struct Building {
    /// Una mesh per materiale.
    pub parts: Vec<(Mat, Mesh)>,
    /// Manto di neve sui tetti (mostrato solo d'inverno); vuoto se non ci sono tetti.
    pub snow: Mesh,
    pub colliders: Vec<(Vec3, Vec3)>,
    pub half: Vec2,
    /// Porta, pavimenti calpestabili, letti e luci (vuoto per gli oggetti pieni).
    pub interior: Interior,
}

/// Porta vera di un edificio: cerniera in basso a sinistra, l'anta si estende verso +X.
#[derive(Clone, Copy, Debug)]
pub struct Door {
    pub hinge: Vec3,
    pub width: f32,
    pub height: f32,
}

/// Posto letto (coordinate locali all'edificio).
#[derive(Clone, Copy, Debug)]
pub struct Bed {
    /// Centro del materasso.
    pub pos: Vec3,
    /// Direzione della testata (±X o ±Z).
    pub head: Vec3,
    /// Piano del letto (0 = terra).
    pub floor: usize,
    /// Punto libero accanto al letto, a livello del pavimento.
    pub stand: Vec3,
}

/// Parti interne di un edificio visitabile.
#[derive(Default)]
pub struct Interior {
    pub door: Option<Door>,
    /// Pareti, soffitti: fermano giocatore e camera.
    pub walls: Vec<(Vec3, Vec3)>,
    /// Mobili grandi: fermano il giocatore, non la camera.
    pub furniture: Vec<(Vec3, Vec3)>,
    /// Pavimenti e gradini: `Ground` per il controllo di appoggio.
    pub ground: Vec<(Vec3, Vec3)>,
    pub beds: Vec<Bed>,
    /// Luci interne, una per piano.
    pub lights: Vec<Vec3>,
    /// Mobili che contengono oggetti.
    pub containers: Vec<crate::containers::ContainerSpot>,
    /// Bocche dei camini, da dove esce il fumo.
    pub chimneys: Vec<Vec3>,
    /// Punto subito dentro la porta.
    pub door_inside: Option<Vec3>,
    /// Per ogni piano, il percorso da dentro la porta al pianerottolo.
    pub routes: Vec<Vec<Vec3>>,
}

impl Interior {
    /// Percorso a piedi da dentro la porta al fianco del letto (coordinate locali).
    pub fn route_to_bed(&self, bed: &Bed) -> Vec<Vec3> {
        let mut path: Vec<Vec3> = self.door_inside.into_iter().collect();
        if let Some(stairs) = self.routes.get(bed.floor) {
            path.extend(stairs.iter().copied());
        }
        path.push(bed.stand);
        path
    }
}

pub(crate) type Col = [f32; 4];
const WHITE: Col = [1.0, 1.0, 1.0, 1.0];
const WATER: Col = [0.1, 0.25, 0.4, 1.0];
const HAY: Col = [0.85, 0.7, 0.3, 1.0];
const HAY_DARK: Col = [0.8, 0.62, 0.22, 1.0];

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
    uv: Vec<[f32; 2]>,
    col: Vec<Col>,
    idx: Vec<u32>,
    /// Materiale di ogni triangolo.
    tri_mat: Vec<Mat>,
    /// Materiale imposto alle prossime facce (`None` = dedotto dal colore).
    pub(crate) force: Option<Mat>,
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

    /// Aggiunge i vertici di una faccia piana, con UV in metri e tinta secondo il materiale.
    fn face(&mut self, pts: &[Vec3], n: Vec3, col: Col) -> (u32, Mat) {
        let (mat, tint) = match self.force {
            Some(Mat::Plain) => (Mat::Plain, col),
            Some(m) => (m, [col[0] * 1.8, col[1] * 1.8, col[2] * 1.8, 1.0]),
            None => classify(col),
        };
        let u = if n.y.abs() > 0.99 {
            Vec3::X
        } else {
            Vec3::Y.cross(n).normalize()
        };
        let v = n.cross(u).normalize();
        let base = self.pos.len() as u32;
        for p in pts {
            self.pos.push(p.to_array());
            self.nor.push(n.to_array());
            self.uv.push([p.dot(u), -p.dot(v)]);
            self.col.push(tint);
        }
        (base, mat)
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
        let (base, mat) = self.face(&pts, n, col);
        self.idx
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        self.tri_mat.extend([mat, mat]);
    }

    pub(crate) fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, hint: Vec3, col: Col) {
        let mut n = (b - a).cross(c - a).normalize_or_zero();
        let pts = if n.dot(hint) < 0.0 {
            n = -n;
            [a, c, b]
        } else {
            [a, b, c]
        };
        let (base, mat) = self.face(&pts, n, col);
        self.idx.extend([base, base + 1, base + 2]);
        self.tri_mat.push(mat);
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
        // Faccia inferiore: si vede da sotto (soffitti, gradini della chiocciola).
        self.quad(
            v(x0, y0, z0),
            v(x1, y0, z0),
            v(x1, y0, z1),
            v(x0, y0, z1),
            Vec3::NEG_Y,
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

    /// Piramide su base rettangolare orientata lungo `t` e `n` (orizzontali).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn pyramid_oriented(
        &mut self,
        c: Vec3,
        t: Vec2,
        n: Vec2,
        half_t: f32,
        half_n: f32,
        h: f32,
        col: Col,
    ) {
        let top = c + Vec3::Y * h;
        let (t3, n3) = (Vec3::new(t.x, 0.0, t.y), Vec3::new(n.x, 0.0, n.y));
        let pts = [
            c - t3 * half_t - n3 * half_n,
            c + t3 * half_t - n3 * half_n,
            c + t3 * half_t + n3 * half_n,
            c - t3 * half_t + n3 * half_n,
        ];
        for i in 0..4 {
            let (a, b) = (pts[i], pts[(i + 1) % 4]);
            let out = ((a + b) / 2.0 - c).normalize_or_zero() + Vec3::Y * 0.3;
            self.tri(a, b, top, out, col);
            self.snow_tri(a, b, top);
        }
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

    /// Trave a sezione quadrata fra due punti qualsiasi.
    pub(crate) fn beam(&mut self, a: Vec3, b: Vec3, half: f32, col: Col) {
        let d = (b - a).normalize_or(Vec3::Y);
        let refer = if d.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
        let u = d.cross(refer).normalize() * half;
        let v = d.cross(u).normalize() * half;
        for (s, t) in [(u, v), (v, -u), (-u, -v), (-v, u)] {
            let corner = |p: Vec3, x: f32| p + s * x + t;
            self.quad(
                corner(a, -1.0),
                corner(a, 1.0),
                corner(b, 1.0),
                corner(b, -1.0),
                t,
                col,
            );
        }
    }

    /// Mesh unica a colori di vertice (campi coltivati).
    pub(crate) fn finish(self) -> Mesh {
        let mut m = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.nor);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uv);
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.col);
        m.insert_indices(Indices::U32(self.idx));
        m
    }

    /// Una mesh per materiale, con le tangenti per le normal map.
    pub(crate) fn finish_parts(self) -> Vec<(Mat, Mesh)> {
        let mut out = Vec::new();
        for mat in MATS {
            let mut remap = std::collections::HashMap::new();
            let (mut pos, mut nor, mut uv, mut col, mut idx) =
                (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
            for (t, tri) in self.idx.as_chunks::<3>().0.iter().enumerate() {
                if self.tri_mat[t] != mat {
                    continue;
                }
                for &i in tri {
                    let new = *remap.entry(i).or_insert_with(|| {
                        let i = i as usize;
                        pos.push(self.pos[i]);
                        nor.push(self.nor[i]);
                        uv.push(self.uv[i]);
                        col.push(self.col[i]);
                        pos.len() as u32 - 1
                    });
                    idx.push(new);
                }
            }
            if idx.is_empty() {
                continue;
            }
            let mut m = Mesh::new(
                PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            );
            m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
            m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nor);
            m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
            m.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
            m.insert_indices(Indices::U32(idx));
            if m.generate_tangents().is_err() {
                warn!("[EDIFICI] tangenti non generate per {mat:?}");
            }
            out.push((mat, m));
        }
        out
    }
}

/// Materiale e tinta di un colore della tavolozza. Le tinte leggermente variate (`tint`) restano riconosciute.
fn classify(col: Col) -> (Mat, Col) {
    let near = |c: Col| (0..3).all(|i| ((col[i] / c[i].max(0.001)) - 1.0).abs() < 0.12);
    let ratio = |c: Col| {
        let k = (col[0] + col[1] + col[2]) / (c[0] + c[1] + c[2]).max(0.001);
        [k, k, k, 1.0]
    };
    if near(STONE) {
        return (Mat::Stone, ratio(STONE));
    }
    if near(STONE_DARK) {
        let k = ratio(STONE_DARK)[0] * 0.68;
        return (Mat::Stone, [k, k, k, 1.0]);
    }
    if near(ROOFS[0]) {
        return (Mat::Tiles, WHITE);
    }
    if near(ROOFS[1]) {
        return (Mat::Slate, WHITE);
    }
    if near(ROOFS[2]) {
        return (Mat::Thatch, WHITE);
    }
    if near(GLASS) {
        return (Mat::Glass, [col[0] * 0.3, col[1] * 0.3, col[2] * 0.3, 1.0]);
    }
    if near(WATER) {
        return (Mat::Glass, col);
    }
    if near(HAY) || near(HAY_DARK) {
        return (Mat::Thatch, [1.25, 1.1, 0.75, 1.0]);
    }
    if near(TIMBER) {
        return (Mat::Planks, [0.8, 0.75, 0.7, 1.0]);
    }
    if near(DOOR) {
        return (Mat::Planks, [1.05, 0.95, 0.85, 1.0]);
    }
    if SHUTTERS.iter().any(|c| near(*c)) {
        let k = 1.9;
        return (Mat::Planks, [col[0] * k, col[1] * k, col[2] * k, 1.0]);
    }
    if WALLS.iter().any(|c| near(*c)) {
        return (
            Mat::Plaster,
            [col[0] * 1.7, col[1] * 1.7, col[2] * 1.7, 1.0],
        );
    }
    // Marroni sparsi (molo, barche, pali, casse): legno, schiarito o scurito come il colore originale.
    let lum = (col[0] + col[1] + col[2]) / 3.0;
    if col[0] > col[1] && col[1] > col[2] && col[0] / col[2].max(0.01) > 1.4 && lum < 0.62 {
        let k = lum / 0.3;
        return (Mat::Planks, [k, k * 0.95, k * 0.9, 1.0]);
    }
    // Il resto (fasce del faro, stendardi): vernice su intonaco.
    (
        Mat::Plaster,
        [col[0] * 1.7, col[1] * 1.7, col[2] * 1.7, 1.0],
    )
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

/// Quota del pavimento del piano terra, sopra lo zoccolo.
pub(crate) const GROUND_FLOOR: f32 = 0.42;
/// Altezza di un piano.
const STOREY: f32 = 2.5;
/// Spessore delle pareti esterne.
const WALL_T: f32 = 0.22;

/// Cornice e gradini della porta sul fronte (+Z); l'anta vera è in `world_props`.
fn door_front(b: &mut B, int: &mut Interior, x: f32, z: f32, w: f32, h: f32) {
    let y0 = GROUND_FLOOR;
    b.cuboid(
        Vec3::new(x - w / 2.0 - 0.1, y0, z - 0.02),
        Vec3::new(x - w / 2.0, y0 + h + 0.1, z + 0.08),
        TIMBER,
    );
    b.cuboid(
        Vec3::new(x + w / 2.0, y0, z - 0.02),
        Vec3::new(x + w / 2.0 + 0.1, y0 + h + 0.1, z + 0.08),
        TIMBER,
    );
    b.cuboid(
        Vec3::new(x - w / 2.0, y0 + h, z - 0.02),
        Vec3::new(x + w / 2.0, y0 + h + 0.1, z + 0.08),
        TIMBER,
    );
    // Due gradini di pietra.
    for (top, depth, back) in [(0.21f32, 0.75f32, 0.0), (GROUND_FLOOR, 0.38, WALL_T + 0.02)] {
        let min = Vec3::new(x - w / 2.0 - 0.25, -0.4, z - back);
        let max = Vec3::new(x + w / 2.0 + 0.25, top, z + depth);
        b.cuboid(min, max, STONE);
        int.ground.push(((min + max) / 2.0, max - min));
    }
    int.door = Some(Door {
        hinge: Vec3::new(x - w / 2.0, y0, z - WALL_T / 2.0),
        width: w,
        height: h,
    });
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

/// Casa a base `w`×`d` su `floors` piani, cava: pareti, solai, scale, porta e sottotetto. Restituisce l'altezza dei muri.
#[allow(clippy::too_many_arguments)]
fn house(
    b: &mut B,
    r: &mut Rng,
    int: &mut Interior,
    kind: &str,
    w: f32,
    d: f32,
    floors: usize,
    timber: bool,
    chimneys: usize,
    seed: u32,
) -> f32 {
    use crate::interiors as rooms;
    let wall = r.pick(&WALLS);
    let roof = r.pick(&ROOFS);
    let shutter = r.pick(&SHUTTERS);
    let (hw, hd) = (w / 2.0, d / 2.0);
    let y0 = GROUND_FLOOR;
    let wh = y0 + STOREY * floors as f32 + 0.04;
    // Zoccolo di pietra che affonda nel terreno (compensa le pendenze): è la base del pavimento.
    b.cuboid(
        Vec3::new(-hw - 0.12, -1.3, -hd - 0.12),
        Vec3::new(hw + 0.12, 0.4, hd + 0.12),
        STONE,
    );
    let door_x = if floors > 1 { 0.0 } else { -hw * 0.35 };
    let (door_w, door_h) = (1.0, 2.05);
    let t = WALL_T;
    // Pareti cave: la facciata ha il vano della porta.
    let mut wall_box = |b: &mut B, min: Vec3, max: Vec3| {
        b.cuboid(min, max, wall);
        int.walls.push(((min + max) / 2.0, max - min));
    };
    wall_box(b, Vec3::new(-hw, 0.4, -hd), Vec3::new(hw, wh, -hd + t));
    wall_box(
        b,
        Vec3::new(-hw, 0.4, -hd + t),
        Vec3::new(-hw + t, wh, hd - t),
    );
    wall_box(
        b,
        Vec3::new(hw - t, 0.4, -hd + t),
        Vec3::new(hw, wh, hd - t),
    );
    wall_box(
        b,
        Vec3::new(-hw, 0.4, hd - t),
        Vec3::new(door_x - door_w / 2.0, wh, hd),
    );
    wall_box(
        b,
        Vec3::new(door_x + door_w / 2.0, 0.4, hd - t),
        Vec3::new(hw, wh, hd),
    );
    wall_box(
        b,
        Vec3::new(door_x - door_w / 2.0, y0 + door_h, hd - t),
        Vec3::new(door_x + door_w / 2.0, wh, hd),
    );
    if timber {
        // Travi a vista: montanti d'angolo e fasce orizzontali.
        for (x, z) in [(-hw, -hd), (hw, -hd), (-hw, hd), (hw, hd)] {
            b.cuboid_c(
                Vec3::new(x, wh / 2.0 + 0.2, z),
                Vec3::new(0.2, wh - 0.4, 0.2),
                TIMBER,
            );
        }
        for k in 1..=floors {
            let y = y0 + STOREY * k as f32 - 0.05;
            for z in [hd + 0.01, -hd - 0.01] {
                b.cuboid_c(Vec3::new(0.0, y, z), Vec3::new(w + 0.1, 0.16, 0.12), TIMBER);
            }
        }
    }
    // Tetto ripido con il colmo lungo X (lato lungo): il sottotetto è abitabile.
    let rh = d * 0.8;
    b.gable((-hw, hw, -hd, hd), wh, rh, 0.4, true, roof, wall);
    // Dentro: falde e timpani visti dal sottotetto.
    b.force = Some(Mat::Planks);
    let side = (hd + 0.4, rh * (hd + 0.4) / hd);
    for s in [-1.0f32, 1.0] {
        b.quad(
            Vec3::new(-hw, wh - 0.1, s * side.0),
            Vec3::new(hw, wh - 0.1, s * side.0),
            Vec3::new(hw, wh + rh - 0.1, 0.0),
            Vec3::new(-hw, wh + rh - 0.1, 0.0),
            Vec3::new(0.0, -1.0, -s),
            [0.4, 0.3, 0.2, 1.0],
        );
    }
    b.force = None;
    for s in [-1.0f32, 1.0] {
        b.tri(
            Vec3::new(s * (hw - 0.02), wh, -hd),
            Vec3::new(s * (hw - 0.02), wh, hd),
            Vec3::new(s * (hw - 0.02), wh + rh, 0.0),
            Vec3::new(-s, 0.0, 0.0),
            wall,
        );
    }
    door_front(b, int, door_x, hd, door_w, door_h);
    let per_floor = ((w - 1.2) / 2.2).floor().max(1.0) as usize;
    for f in 0..floors {
        let y = y0 + 1.25 + f as f32 * STOREY;
        for i in 0..per_floor {
            let x = -hw + 0.9 + (w - 1.8) * (i as f32 + 0.5) / per_floor as f32;
            if f == 0 && (x - door_x).abs() < 1.1 {
                continue;
            }
            window(b, Vec3::new(x, y, hd), Vec3::Z, shutter);
            inner_window(b, Vec3::new(x, y, hd - t - 0.01), Vec3::NEG_Z, (0.62, 0.86));
        }
        window(b, Vec3::new(0.0, y, -hd), Vec3::NEG_Z, shutter);
        inner_window(b, Vec3::new(0.0, y, -hd + t + 0.01), Vec3::Z, (0.62, 0.86));
    }
    // Piani, scale e arredi.
    let (x0, x1, z0, z1) = (-hw + t, hw - t, -hd + t, hd - t);
    let mut ys: Vec<f32> = (0..=floors).map(|k| y0 + k as f32 * STOREY).collect();
    let attic_y = *ys.last().unwrap() + 0.02;
    *ys.last_mut().unwrap() = attic_y;
    // Semi-larghezza del sottotetto in cui si sta in piedi.
    let knee = hd * (1.0 - 1.85 / rh).max(0.25);
    let layout = rooms::floors_and_stairs(b, int, (x0, x1, z0, z1), &ys, knee);
    int.door_inside = Some(Vec3::new(door_x, y0, hd - t - 0.45));
    int.routes = layout.routes.clone();
    for (f, &y) in ys.iter().enumerate().take(floors) {
        // L'ultimo piano lascia libera la rampa del sottotetto, a destra.
        let room_x1 = if f + 1 == floors {
            layout.top_x0 - 0.15
        } else {
            x1 - 0.1
        };
        let room = rooms::Room {
            x0: x0 + 0.1,
            x1: room_x1,
            z0: z0 + 1.2,
            z1: z1 - 0.1,
            y,
            floor: f,
            back_z: z0,
            back_x: layout.back_x,
        };
        rooms::furnish(b, int, kind, &room, f, seed);
        int.lights.push(Vec3::new(0.0, y + 2.1, (z0 + z1) / 2.0));
    }
    // Sottotetto: pareti basse (quella posteriore si ferma alla rampa), soffitto e ripostiglio.
    b.force = Some(Mat::Planks);
    for s in [-1.0f32, 1.0] {
        let right = if s < 0.0 { layout.top_x0 - 0.05 } else { x1 };
        let cx = (x0 + right) / 2.0;
        let c = Vec3::new(cx, attic_y + 0.9, s * (knee + 0.05));
        let size = Vec3::new(right - x0, 1.8, 0.1);
        b.cuboid_c(c, size, [0.4, 0.3, 0.2, 1.0]);
        int.walls.push((c, size));
    }
    b.force = None;
    let ceiling_y = attic_y + 2.15;
    int.walls.push((
        Vec3::new(0.0, ceiling_y + 0.6, 0.0),
        Vec3::new(w, 1.2, 2.0 * knee + 0.4),
    ));
    for s in [-1.0f32, 1.0] {
        // Le testate salgono fino al soffitto; le falde esterne sono piene per la camera.
        int.walls.push((
            Vec3::new(s * (hw - t / 2.0), attic_y + 1.1, 0.0),
            Vec3::new(t, 2.2, 2.0 * knee + 0.4),
        ));
        int.walls.push((
            Vec3::new(0.0, wh + 0.6, s * (knee + 0.1 + (hd - knee) / 2.0)),
            Vec3::new(w, 1.2, hd - knee - 0.1),
        ));
    }
    let attic_room = rooms::Room {
        x0: x0 + 0.9,
        x1: layout.top_x0 - 0.9,
        z0: -knee + 0.05,
        z1: knee - 0.05,
        y: attic_y,
        floor: floors,
        back_z: z0,
        back_x: x0,
    };
    if attic_room.x1 - attic_room.x0 > 0.9 {
        rooms::furnish_attic(b, int, &attic_room);
    }
    int.lights.push(Vec3::new(0.0, attic_y + 1.6, 0.0));
    // Camini: uno addossato alla testata sinistra, l'altro passante nel sottotetto.
    for i in 0..chimneys {
        let x = if i == 0 { -hw + 0.35 } else { hw * 0.1 };
        chimney(b, x, 0.0, wh, wh + rh + 0.9);
        int.chimneys.push(Vec3::new(x, wh + rh + 1.0, 0.0));
        int.walls
            .push((Vec3::new(x, wh + 1.1, 0.0), Vec3::new(0.7, 2.2, 0.7)));
    }
    wh
}

/// Finestra vista da dentro: un pannello luminoso con il telaio.
fn inner_window(b: &mut B, c: Vec3, n: Vec3, (w, h): (f32, f32)) {
    let t = Vec3::Y.cross(n).normalize();
    b.force = Some(Mat::Plain);
    b.quad(
        c - t * (w / 2.0) - Vec3::Y * (h / 2.0),
        c + t * (w / 2.0) - Vec3::Y * (h / 2.0),
        c + t * (w / 2.0) + Vec3::Y * (h / 2.0),
        c - t * (w / 2.0) + Vec3::Y * (h / 2.0),
        n,
        [0.62, 0.76, 0.9, 1.0],
    );
    b.force = None;
    b.cuboid_c(
        c,
        t.abs() * (w + 0.1) + Vec3::Y * 0.05 + n.abs() * 0.04,
        TIMBER,
    );
}

/// Chiesa cava: navata con panche e altare, campanile con scala a chiocciola fino alla cella delle campane.
fn church(b: &mut B, r: &mut Rng, int: &mut Interior) {
    use crate::interiors as rooms;
    let wall = [0.93, 0.9, 0.82, 1.0];
    let roof = ROOFS[1];
    let (hw, wh) = (3.5, 5.4);
    let t = 0.3;
    let y0 = GROUND_FLOOR;
    let (z_back, z_front) = (-7.5, 3.5); // navata
    let tz = z_front + 1.7; // centro del campanile
    let thw = 1.7;
    let (t_back, t_front) = (tz - 1.8, tz + 1.8);
    // Muri con collider.
    let mut mur = |b: &mut B, min: Vec3, max: Vec3| {
        b.cuboid(min, max, wall);
        int.walls.push(((min + max) / 2.0, max - min));
    };
    // Basamenti.
    b.cuboid(
        Vec3::new(-hw - 0.15, -1.4, z_back - 0.15),
        Vec3::new(hw + 0.15, 0.4, z_front + 0.15),
        STONE,
    );
    b.cuboid(
        Vec3::new(-1.8, -1.5, tz - 1.9),
        Vec3::new(1.8, 0.4, tz + 1.9),
        STONE,
    );
    // Navata: fianchi, fondo e facciata con la porta verso il campanile.
    mur(
        b,
        Vec3::new(-hw, 0.4, z_back),
        Vec3::new(-hw + t, wh, z_front),
    );
    mur(
        b,
        Vec3::new(hw - t, 0.4, z_back),
        Vec3::new(hw, wh, z_front),
    );
    mur(
        b,
        Vec3::new(-hw + t, 0.4, z_back),
        Vec3::new(hw - t, wh, z_back + t),
    );
    let (nd_w, nd_h) = (1.4, 2.25);
    mur(
        b,
        Vec3::new(-hw + t, 0.4, z_front - t),
        Vec3::new(-nd_w / 2.0, wh, z_front),
    );
    mur(
        b,
        Vec3::new(nd_w / 2.0, 0.4, z_front - t),
        Vec3::new(hw - t, wh, z_front),
    );
    mur(
        b,
        Vec3::new(-nd_w / 2.0, y0 + nd_h, z_front - t),
        Vec3::new(nd_w / 2.0, wh, z_front),
    );
    b.gable((-hw, hw, z_back, z_front), wh, 3.2, 0.5, false, roof, wall);
    // Interno del tetto a vista.
    b.force = Some(Mat::Planks);
    for s in [-1.0f32, 1.0] {
        b.quad(
            Vec3::new(s * (hw + 0.5), wh - 0.1, z_back),
            Vec3::new(s * (hw + 0.5), wh - 0.1, z_front),
            Vec3::new(0.0, wh + 3.1, z_front),
            Vec3::new(0.0, wh + 3.1, z_back),
            Vec3::new(-s, -1.0, 0.0),
            [0.4, 0.3, 0.2, 1.0],
        );
    }
    b.force = None;
    for (z, nz) in [(z_back + 0.02, 1.0f32), (z_front - 0.02, -1.0)] {
        b.tri(
            Vec3::new(-hw, wh, z),
            Vec3::new(hw, wh, z),
            Vec3::new(0.0, wh + 3.2, z),
            Vec3::new(0.0, 0.0, nz),
            wall,
        );
    }
    // Pavimento di pietra.
    for (x0, x1, z0, z1) in [
        (-hw, hw, z_back, z_front + 0.2),
        (-thw, thw, t_back, t_front),
    ] {
        let min = Vec3::new(x0, y0 - 0.18, z0);
        let max = Vec3::new(x1, y0, z1);
        b.cuboid(min, max, STONE);
        int.ground.push(((min + max) / 2.0, max - min));
    }
    // Abside poligonale (più bassa, piena).
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
    // Campanile cavo sul fronte, con le aperture della cella campanaria vere.
    let tt = 0.25;
    let top = 10.4;
    let (sill, lintel) = (8.9, 10.1);
    let open_half = 0.55;
    // Fianchi e retro/fronte: pieni sotto e sopra la cella, con pilastri ai lati delle aperture.
    let mut tower_wall =
        |b: &mut B, x_side: Option<f32>, z_side: Option<f32>, gap: Option<(f32, f32)>| {
            let (min, max) = match (x_side, z_side) {
                (Some(s), _) => (
                    Vec3::new(if s < 0.0 { -thw } else { thw - tt }, 0.4, t_back),
                    Vec3::new(if s < 0.0 { -thw + tt } else { thw }, top, t_front),
                ),
                (_, Some(s)) => (
                    Vec3::new(-thw, 0.4, if s < 0.0 { t_back } else { t_front - tt }),
                    Vec3::new(thw, top, if s < 0.0 { t_back + tt } else { t_front }),
                ),
                _ => return,
            };
            let along_x = z_side.is_some();
            let (a0, a1) = if along_x {
                (min.x, max.x)
            } else {
                (min.z, max.z)
            };
            let c = if along_x { 0.0 } else { tz };
            let span = |b: &mut B, ua: f32, ub: f32, ya: f32, yb: f32| {
                let (mut lo, mut hi) = (min, max);
                if along_x {
                    lo.x = ua;
                    hi.x = ub;
                } else {
                    lo.z = ua;
                    hi.z = ub;
                }
                lo.y = ya;
                hi.y = yb;
                b.cuboid(lo, hi, wall);
            };
            // Sotto la cella, con l'eventuale vano della porta.
            match gap {
                Some((half, h)) => {
                    span(b, a0, c - half, 0.4, sill);
                    span(b, c + half, a1, 0.4, sill);
                    span(b, c - half, c + half, y0 + h, sill);
                }
                None => span(b, a0, a1, 0.4, sill),
            }
            span(b, a0, c - open_half, sill, lintel);
            span(b, c + open_half, a1, sill, lintel);
            span(b, a0, a1, lintel, top);
            // Il collider è pieno: dalle aperture si guarda ma non si cade.
            let mut block = |ua: f32, ub: f32, ya: f32| {
                let (mut lo, mut hi) = (min, max);
                if along_x {
                    lo.x = ua;
                    hi.x = ub;
                } else {
                    lo.z = ua;
                    hi.z = ub;
                }
                lo.y = ya;
                int.walls.push(((lo + hi) / 2.0, hi - lo));
            };
            match gap {
                Some((half, h)) => {
                    block(a0, c - half, 0.4);
                    block(c + half, a1, 0.4);
                    block(c - half, c + half, y0 + h);
                }
                None => block(a0, a1, 0.4),
            }
        };
    tower_wall(b, Some(-1.0), None, None);
    tower_wall(b, Some(1.0), None, None);
    tower_wall(b, None, Some(-1.0), Some((nd_w / 2.0, nd_h)));
    tower_wall(b, None, Some(1.0), Some((0.85, 2.8)));
    // Scala a chiocciola e cella campanaria.
    let inner = (-thw + tt, thw - tt, t_back + tt, t_front - tt);
    let belfry = 8.0;
    let hole = rooms::square_helix(b, int, inner, y0, belfry, 0.8);
    rooms::slab_with_hole(b, int, inner, hole, belfry);
    // Campana appesa al soffitto della cella.
    let bronze = [0.55, 0.42, 0.2, 1.0];
    for (y, half) in [(9.05f32, 0.42f32), (9.35, 0.34), (9.65, 0.26), (9.9, 0.16)] {
        b.cuboid_c(
            Vec3::new(0.0, y, tz),
            Vec3::new(half * 2.0, 0.3, half * 2.0),
            bronze,
        );
    }
    b.cuboid_c(
        Vec3::new(0.0, 10.25, tz),
        Vec3::new(0.08, 0.5, 0.08),
        TIMBER,
    );
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
    door_front(b, int, 0.0, t_front, 1.7, 2.8);
    b.cuboid_c(
        Vec3::new(0.0, 6.8, tz + 1.82),
        Vec3::new(1.0, 1.0, 0.08),
        [0.9, 0.85, 0.6, 1.0],
    );
    // Finestre alte e strette sui fianchi della navata, viste anche da dentro.
    for i in 0..3 {
        let z = z_back + 2.0 + i as f32 * 3.0;
        for s in [-1.0f32, 1.0] {
            b.cuboid_c(
                Vec3::new(s * (hw + 0.02), 3.2, z),
                Vec3::new(0.06, 2.0, 0.8),
                GLASS,
            );
            inner_window(
                b,
                Vec3::new(s * (hw - t - 0.01), 3.2, z),
                Vec3::new(-s, 0.0, 0.0),
                (0.8, 2.0),
            );
        }
    }
    let _ = r.next();
    // Panche e altare.
    let nave = rooms::Room {
        x0: -hw + t,
        x1: hw - t,
        z0: z_back + t,
        z1: z_front - t,
        y: y0,
        floor: 0,
        back_z: z_back + t,
        back_x: -hw + t,
    };
    rooms::furnish_church(b, int, &nave);
    // Tetto pieno per la camera; l'abside è un volume unico.
    int.walls.push((
        Vec3::new(0.0, wh + 0.9, (z_back + z_front) / 2.0),
        Vec3::new(7.0, 1.8, z_front - z_back),
    ));
    int.walls
        .push((Vec3::new(0.0, 2.0, z_back - 1.2), Vec3::new(4.2, 4.2, 2.6)));
    int.walls
        .push((Vec3::new(0.0, 10.55, tz), Vec3::new(3.7, 0.3, 3.8)));
    int.lights.push(Vec3::new(0.0, 4.6, -4.0));
    int.lights.push(Vec3::new(0.0, 4.6, 0.8));
    int.lights.push(Vec3::new(0.0, 2.6, tz));
    int.lights.push(Vec3::new(0.0, belfry + 2.0, tz));
}

fn sign_arm(b: &mut B, col: Col) {
    b.cuboid_c(Vec3::new(1.9, 3.1, 3.0), Vec3::new(0.08, 0.08, 1.0), TIMBER);
    b.cuboid_c(Vec3::new(1.9, 2.65, 3.45), Vec3::new(0.08, 0.7, 0.9), col);
    b.cuboid_c(
        Vec3::new(1.9, 2.65, 3.5),
        Vec3::new(0.1, 0.1, 0.55),
        [0.95, 0.9, 0.7, 1.0],
    );
}

/// Ruota di legno nel piano YZ: cerchio a segmenti e raggi.
fn wheel(b: &mut B, center: Vec3, radius: f32) {
    use std::f32::consts::TAU;
    const SIDES: usize = 12;
    let wood: Col = [0.34, 0.23, 0.13, 1.0];
    let rim = |k: usize| {
        let a = k as f32 / SIDES as f32 * TAU;
        center + Vec3::new(0.0, a.cos() * radius, a.sin() * radius)
    };
    for k in 0..SIDES {
        b.beam(rim(k), rim(k + 1), 0.045, wood);
    }
    for k in (0..SIDES / 2).step_by(2) {
        b.beam(rim(k), rim(k + SIDES / 2), 0.03, wood);
    }
}

/// Carretto a due ruote con una cassa sul pianale.
fn cart(b: &mut B, int: &mut Interior) -> Vec<(Vec3, Vec3)> {
    let y = 0.7;
    b.cuboid_c(Vec3::new(0.0, y, 0.0), Vec3::new(1.3, 0.08, 2.2), TIMBER);
    for side in [-1.0f32, 1.0] {
        b.cuboid_c(
            Vec3::new(side * 0.63, y + 0.22, 0.0),
            Vec3::new(0.05, 0.36, 2.2),
            TIMBER,
        );
        wheel(b, Vec3::new(side * 0.8, 0.5, -0.1), 0.5);
        b.beam(
            Vec3::new(side * 0.42, y - 0.02, 1.05),
            Vec3::new(side * 0.42, 0.28, 2.95),
            0.035,
            TIMBER,
        );
    }
    b.cuboid_c(
        Vec3::new(0.0, y + 0.22, -1.08),
        Vec3::new(1.3, 0.36, 0.05),
        TIMBER,
    );
    b.cuboid_c(
        Vec3::new(0.0, 0.5, -0.1),
        Vec3::new(1.5, 0.08, 0.08),
        TIMBER,
    );
    crate::interiors::crate_box(b, int, 0.0, -0.45, y + 0.04, 0.62);
    vec![(Vec3::new(0.0, 0.75, -0.05), Vec3::new(1.6, 1.5, 2.3))]
}

/// Filo con i panni stesi tra due pali.
fn laundry(b: &mut B, r: &mut Rng) -> Vec<(Vec3, Vec3)> {
    const SPAN: f32 = 2.3;
    const TOP: f32 = 2.05;
    for side in [-1.0f32, 1.0] {
        b.cuboid_c(
            Vec3::new(side * SPAN, TOP / 2.0, 0.0),
            Vec3::new(0.08, TOP, 0.08),
            TIMBER,
        );
    }
    b.cuboid_c(
        Vec3::new(0.0, TOP - 0.04, 0.0),
        Vec3::new(SPAN * 2.0, 0.02, 0.02),
        [0.75, 0.72, 0.62, 1.0],
    );
    let colors: [Col; 5] = [
        [0.86, 0.84, 0.76, 1.0],
        [0.62, 0.3, 0.26, 1.0],
        [0.32, 0.42, 0.58, 1.0],
        [0.78, 0.66, 0.4, 1.0],
        [0.42, 0.52, 0.36, 1.0],
    ];
    b.force = Some(Mat::Cloth);
    let mut x = -SPAN + 0.55;
    while x < SPAN - 0.4 {
        let w = 0.45 + r.next() as f32 % 40.0 / 100.0;
        let h = 0.6 + r.next() as f32 % 45.0 / 100.0;
        b.cuboid_c(
            Vec3::new(x + w / 2.0, TOP - 0.05 - h / 2.0, 0.0),
            Vec3::new(w, h, 0.025),
            r.pick(&colors),
        );
        x += w + 0.14;
    }
    b.force = None;
    [-1.0f32, 1.0]
        .map(|s| {
            (
                Vec3::new(s * SPAN, TOP / 2.0, 0.0),
                Vec3::new(0.14, TOP, 0.14),
            )
        })
        .to_vec()
}

/// Reti da pesca stese ad asciugare su un telaio.
fn nets(b: &mut B) -> Vec<(Vec3, Vec3)> {
    const SPAN: f32 = 1.6;
    const TOP: f32 = 2.1;
    for side in [-1.0f32, 1.0] {
        b.cuboid_c(
            Vec3::new(side * SPAN, TOP / 2.0, 0.0),
            Vec3::new(0.1, TOP, 0.1),
            TIMBER,
        );
    }
    b.cuboid_c(
        Vec3::new(0.0, TOP, 0.0),
        Vec3::new(SPAN * 2.0 + 0.3, 0.08, 0.08),
        TIMBER,
    );
    let cord: Col = [0.3, 0.3, 0.26, 1.0];
    let mut x = -SPAN + 0.15;
    while x < SPAN {
        b.cuboid_c(Vec3::new(x, 1.3, 0.0), Vec3::new(0.012, 1.55, 0.012), cord);
        x += 0.16;
    }
    let mut y = 0.55;
    while y < TOP {
        b.cuboid_c(
            Vec3::new(0.0, y, 0.0),
            Vec3::new(SPAN * 2.0 - 0.2, 0.012, 0.012),
            cord,
        );
        y += 0.16;
    }
    for k in 0..7 {
        b.cuboid_c(
            Vec3::new(-SPAN + 0.3 + k as f32 * 0.45, 0.52, 0.0),
            Vec3::new(0.09, 0.09, 0.09),
            [0.75, 0.55, 0.25, 1.0],
        );
    }
    [-1.0f32, 1.0]
        .map(|s| {
            (
                Vec3::new(s * SPAN, TOP / 2.0, 0.0),
                Vec3::new(0.16, TOP, 0.16),
            )
        })
        .to_vec()
}

/// Bancarella di mercato: bancone, tendone a strisce, montanti e merce (`kind` dà il tipo).
fn stall(b: &mut B, int: &mut Interior, kind: &str) -> Vec<(Vec3, Vec3)> {
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
    b.force = Some(Mat::Cloth);
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
    b.force = None;
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
    for x in [-0.9, 0.9] {
        int.containers.push(crate::containers::ContainerSpot {
            // Sul fronte delle casse, appena fuori dal collider: dentro sarebbe nascosta alla vista.
            pos: Vec3::new(x, 0.55, -1.2),
            kind: crate::containers::ContainerKind::Crate,
        });
    }
    vec![
        (Vec3::new(0.0, 0.5, 0.45), Vec3::new(2.9, 1.1, 0.95)),
        (Vec3::new(0.0, 0.5, -0.8), Vec3::new(2.9, 1.1, 0.7)),
    ]
}

/// Faro: zoccolo di pietra, torre a fasce bianche e rosse, balconata, lanterna e cupola.
/// Anello di pareti di una sezione di torre attorno a un vano quadrato, con l'eventuale porta sul fronte (+Z).
#[allow(clippy::too_many_arguments)]
fn tower_shell(
    b: &mut B,
    int: &mut Interior,
    hw: f32,
    inner: f32,
    y0: f32,
    y1: f32,
    col: Col,
    door: Option<(f32, f32)>,
) {
    let mut wall = |b: &mut B, min: Vec3, max: Vec3| {
        b.cuboid(min, max, col);
        int.walls.push(((min + max) / 2.0, max - min));
    };
    wall(b, Vec3::new(-hw, y0, -hw), Vec3::new(hw, y1, -inner));
    wall(b, Vec3::new(-hw, y0, -inner), Vec3::new(-inner, y1, inner));
    wall(b, Vec3::new(inner, y0, -inner), Vec3::new(hw, y1, inner));
    match door {
        Some((half, top)) => {
            wall(b, Vec3::new(-hw, y0, inner), Vec3::new(-half, y1, hw));
            wall(b, Vec3::new(half, y0, inner), Vec3::new(hw, y1, hw));
            if y1 > top {
                wall(
                    b,
                    Vec3::new(-half, y0.max(top), inner),
                    Vec3::new(half, y1, hw),
                );
            }
        }
        None => wall(b, Vec3::new(-hw, y0, inner), Vec3::new(hw, y1, hw)),
    }
}

/// Faro cavo: alloggio del guardiano, ripostiglio, scala a chiocciola, lanterna e balconata.
fn lighthouse(b: &mut B, int: &mut Interior) {
    use crate::interiors as rooms;
    let (white, red) = ([0.95, 0.93, 0.88, 1.0], [0.78, 0.18, 0.15, 1.0]);
    let y0 = GROUND_FLOOR;
    let (door_w, door_h) = (0.95, 1.9);
    let door = Some((door_w / 2.0, y0 + door_h));
    // Basamento pieno, calpestabile, con un gradino d'accesso.
    let base = (Vec3::new(-2.6, -1.8, -2.6), Vec3::new(2.6, 0.4, 2.6));
    b.cuboid(base.0, base.1, STONE);
    int.ground.push(((base.0 + base.1) / 2.0, base.1 - base.0));
    let access = (Vec3::new(-0.8, -0.4, 2.6), Vec3::new(0.8, 0.2, 3.2));
    b.cuboid(access.0, access.1, STONE);
    int.ground
        .push(((access.0 + access.1) / 2.0, access.1 - access.0));
    tower_shell(b, int, 1.95, 1.7, 0.4, 1.0, STONE_DARK, door);
    // Le due sezioni basse sono stanze; dalla terza in su resta il pozzo della scala.
    let mut y = 1.0;
    for i in 0..5 {
        let hw = 1.95 - i as f32 * 0.14;
        let inner = match i {
            // La seconda ha pareti sottili: la testa di chi sale la rampa passa sotto il suo bordo.
            0 | 1 => 1.7,
            _ => 0.97,
        };
        let col = if i % 2 == 0 { white } else { red };
        let gap = if i == 0 { door } else { None };
        tower_shell(b, int, hw, inner, y, y + 2.6, col, gap);
        y += 2.6;
    }
    door_front(b, int, 0.0, 1.95, door_w, door_h);
    int.door_inside = Some(Vec3::new(0.0, y0, 1.3));
    int.routes = vec![Vec::new()];
    // Finestrelle, visibili anche da dentro.
    for (i, h) in [3.4f32, 6.0, 8.6, 11.2].iter().enumerate() {
        let n = if i % 2 == 0 { Vec3::X } else { Vec3::NEG_X };
        b.cuboid_c(
            Vec3::new(n.x * (1.78 - i as f32 * 0.1), *h, 0.0),
            Vec3::new(0.08, 0.6, 0.4),
            GLASS,
        );
        let inner = [1.7, 1.7, 0.97, 0.97][i];
        inner_window(b, Vec3::new(n.x * (inner - 0.01), *h, 0.0), -n, (0.4, 0.6));
    }
    // Interni, fino al pavimento della lanterna.
    let floor1 = 3.6;
    let top = y + 0.25;
    rooms::lighthouse_interior(b, int, y0, floor1, top);
    // Balconata percorribile attorno alla lanterna, con ringhiera solida.
    for (min, max) in [
        (Vec3::new(-2.3, y, -2.3), Vec3::new(2.3, top, -0.97)),
        (Vec3::new(-2.3, y, 0.97), Vec3::new(2.3, top, 2.3)),
        (Vec3::new(-2.3, y, -0.97), Vec3::new(-0.97, top, 0.97)),
        (Vec3::new(0.97, y, -0.97), Vec3::new(2.3, top, 0.97)),
    ] {
        b.cuboid(min, max, STONE_DARK);
        int.ground.push(((min + max) / 2.0, max - min));
    }
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
        int.walls.push((
            Vec3::new(cx, top + 0.7, cz),
            Vec3::new(sx.max(0.1), 1.4, sz.max(0.1)),
        ));
    }
    // Lanterna cava (vetri chiari), con l'apertura verso la balconata sul fronte.
    let glow = [1.0, 0.92, 0.62, 1.0];
    let head = y + 2.05;
    let mut pane = |b: &mut B, min: Vec3, max: Vec3| {
        b.cuboid(min, max, glow);
        int.walls.push(((min + max) / 2.0, max - min));
    };
    pane(
        b,
        Vec3::new(-0.95, top, -0.95),
        Vec3::new(0.95, head, -0.89),
    );
    pane(
        b,
        Vec3::new(-0.95, top, -0.89),
        Vec3::new(-0.89, head, 0.95),
    );
    pane(b, Vec3::new(0.89, top, -0.89), Vec3::new(0.95, head, 0.95));
    pane(b, Vec3::new(-0.89, top, 0.89), Vec3::new(-0.6, head, 0.95));
    pane(b, Vec3::new(0.6, top, 0.89), Vec3::new(0.89, head, 0.95));
    // La lampada al centro.
    let lamp = (Vec3::new(-0.2, top, -0.2), Vec3::new(0.2, top + 1.1, 0.2));
    pane(b, lamp.0, lamp.1);
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
    int.lights.push(Vec3::new(0.0, 2.9, 0.2));
    int.lights.push(Vec3::new(-1.0, 5.6, 1.0));
    int.lights.push(Vec3::new(0.0, 10.0, 0.0));
    int.lights.push(Vec3::new(0.3, top + 1.5, 0.3));
}

/// Quota (m) del piano del molo rispetto al livello del mare.
pub const PIER_DECK_TOP: f32 = 0.55;

fn pier(b: &mut B, int: &mut Interior) -> Vec<(Vec3, Vec3)> {
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
    use crate::containers::{ContainerKind, ContainerSpot};
    for (pos, kind) in [
        (Vec3::new(-0.7, deck_y + 0.45, 1.42), ContainerKind::Crate),
        (Vec3::new(-0.6, deck_y + 1.23, 1.0), ContainerKind::Crate),
        (Vec3::new(0.75, deck_y + 0.88, 2.4), ContainerKind::Barrel),
    ] {
        int.containers.push(ContainerSpot { pos, kind });
    }
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
    use crate::interiors as rooms;
    let mut b = B::default();
    let mut r = Rng(seed | 1);
    for _ in 0..3 {
        r.next();
    }
    let mut int = Interior::default();
    let (w, d, colliders): (f32, f32, Vec<(Vec3, Vec3)>) = match kind {
        "casa-piccola" => {
            house(&mut b, &mut r, &mut int, kind, 5.0, 4.0, 2, false, 1, seed);
            (5.0, 4.0, Vec::new())
        }
        "casa-grande" => {
            house(&mut b, &mut r, &mut int, kind, 7.0, 6.0, 3, true, 2, seed);
            (7.0, 6.0, Vec::new())
        }
        "locanda" => {
            house(&mut b, &mut r, &mut int, kind, 10.0, 6.5, 3, true, 2, seed);
            // Insegna a bandiera accanto alla porta.
            b.cuboid_c(Vec3::new(1.6, 3.4, 3.4), Vec3::new(0.08, 0.08, 0.9), TIMBER);
            b.cuboid_c(
                Vec3::new(1.6, 3.0, 3.8),
                Vec3::new(0.7, 0.55, 0.06),
                [0.55, 0.3, 0.15, 1.0],
            );
            (10.0, 6.5, Vec::new())
        }
        "caserma" => {
            // Casa delle guardie: tre piani, travi a vista, stendardo rosso accanto alla porta.
            house(&mut b, &mut r, &mut int, kind, 8.0, 6.0, 3, true, 1, seed);
            b.cuboid_c(Vec3::new(1.9, 3.6, 3.4), Vec3::new(0.08, 0.08, 0.9), TIMBER);
            b.force = Some(Mat::Cloth);
            b.cuboid_c(
                Vec3::new(1.9, 3.1, 3.8),
                Vec3::new(0.7, 0.9, 0.06),
                [0.7, 0.14, 0.12, 1.0],
            );
            b.force = None;
            (8.0, 6.0, Vec::new())
        }
        "fabbro" => {
            house(&mut b, &mut r, &mut int, kind, 6.5, 5.0, 2, false, 1, seed);
            sign_arm(&mut b, [0.35, 0.36, 0.4, 1.0]);
            b.cuboid(
                Vec3::new(3.25, 0.0, -1.5),
                Vec3::new(5.0, 2.3, 2.0),
                [0.45, 0.38, 0.32, 1.0],
            );
            (
                10.0,
                5.0,
                vec![(Vec3::new(4.1, 1.1, 0.25), Vec3::new(1.8, 2.4, 3.5))],
            )
        }
        "forno" => {
            house(&mut b, &mut r, &mut int, kind, 5.5, 4.5, 2, false, 1, seed);
            // Insegna a bandiera: una pagnotta sulla tavola.
            b.cuboid_c(
                Vec3::new(1.7, 3.1, 2.75),
                Vec3::new(0.08, 0.08, 1.0),
                TIMBER,
            );
            b.cuboid_c(
                Vec3::new(1.7, 2.65, 3.1),
                Vec3::new(0.08, 0.7, 0.8),
                [0.42, 0.28, 0.16, 1.0],
            );
            b.cuboid_c(
                Vec3::new(1.7, 2.65, 3.1),
                Vec3::new(0.12, 0.2, 0.5),
                [0.8, 0.56, 0.24, 1.0],
            );
            (5.5, 4.5, Vec::new())
        }
        "stalla" => {
            house(&mut b, &mut r, &mut int, kind, 9.0, 5.0, 1, false, 0, seed);
            b.cuboid(
                Vec3::new(-1.4, 0.0, 2.52),
                Vec3::new(1.4, 2.1, 2.64),
                TIMBER,
            );
            (9.0, 5.0, Vec::new())
        }
        "arcere" => {
            house(&mut b, &mut r, &mut int, kind, 6.2, 5.0, 2, true, 1, seed);
            sign_arm(&mut b, [0.25, 0.5, 0.3, 1.0]);
            (6.2, 5.0, Vec::new())
        }
        "bancarella-erbe" | "bancarella-frutta" | "bancarella-pesce" => {
            let c = stall(&mut b, &mut int, kind);
            (3.0, 2.8, c)
        }
        "faro" => {
            lighthouse(&mut b, &mut int);
            (5.2, 5.2, Vec::new())
        }
        "molo" => {
            let c = pier(&mut b, &mut int);
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
            church(&mut b, &mut r, &mut int);
            (7.4, 17.0, Vec::new())
        }
        "pozzo" => (2.4, 2.4, well(&mut b)),
        "carretto" => (1.6, 3.2, cart(&mut b, &mut int)),
        "botti" => {
            for (x, z) in [(-0.42, 0.0), (0.42, 0.12), (0.02, -0.68)] {
                rooms::barrel(&mut b, &mut int, x, z, 0.0);
            }
            (1.5, 1.5, Vec::new())
        }
        "casse" => {
            rooms::crate_box(&mut b, &mut int, -0.42, 0.0, 0.0, 0.8);
            rooms::crate_box(&mut b, &mut int, 0.44, 0.1, 0.0, 0.66);
            rooms::crate_box(&mut b, &mut int, -0.4, 0.05, 0.8, 0.52);
            (1.7, 1.0, Vec::new())
        }
        "panni" => (4.8, 0.5, laundry(&mut b, &mut r)),
        "reti" => (3.6, 0.5, nets(&mut b)),
        // "casa" e "casa-media"
        _ => {
            let timber = r.next().is_multiple_of(2);
            house(&mut b, &mut r, &mut int, kind, 6.2, 5.0, 2, timber, 1, seed);
            (6.2, 5.0, Vec::new())
        }
    };
    Building {
        snow: b.snow_mesh(),
        parts: b.finish_parts(),
        colliders,
        half: Vec2::new(w / 2.0, d / 2.0),
        interior: int,
    }
}

/// Mesh unica di un campo coltivato: una zolla di terra e una pianta per ogni punto
/// (`kind` = "campo" grano, "orto" cavoli). `height` dà la quota del terreno in un punto.
pub fn crop_field_mesh(kind: &str, pts: &[Vec2], height: &dyn Fn(Vec2) -> f32) -> Mesh {
    let mut b = B {
        force: Some(Mat::Plain),
        ..Default::default()
    };
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
    fn every_bed_can_be_reached_on_foot() {
        for kind in [
            "casa",
            "casa-piccola",
            "casa-media",
            "casa-grande",
            "locanda",
            "caserma",
            "fabbro",
            "forno",
            "arcere",
            "faro",
        ] {
            let int = model(kind, 7).interior;
            for bed in &int.beds {
                let path = int.route_to_bed(bed);
                assert!((path[0].y - GROUND_FLOOR).abs() < 1e-3, "{kind}");
                let last = path.last().unwrap();
                assert!(
                    (last.y - (bed.pos.y - 0.53)).abs() < 1e-3,
                    "{kind}: {path:?}"
                );
                // Si sale soltanto, e con la pendenza di una scala.
                for w in path.windows(2) {
                    let dy = w[1].y - w[0].y;
                    assert!(dy >= -1e-3, "{kind}");
                    let run = Vec2::new(w[1].x - w[0].x, w[1].z - w[0].z).length();
                    assert!(dy <= run * 1.1 + 1e-3, "{kind}: {w:?}");
                }
            }
        }
    }

    #[test]
    fn lighthouse_climbs_to_the_lantern_with_a_real_door() {
        let int = model("faro", 1).interior;
        assert!(int.door.is_some());
        let top = int
            .ground
            .iter()
            .map(|(c, s)| c.y + s.y / 2.0)
            .fold(f32::MIN, f32::max);
        assert!((top - 14.25).abs() < 1e-3, "{top}");
        assert!(int.containers.len() >= 4);
    }

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
            assert!(
                m.parts
                    .iter()
                    .map(|(_, p)| p.count_vertices())
                    .sum::<usize>()
                    > 8,
                "{k} senza mesh"
            );
        }
    }

    #[test]
    fn palette_colors_get_a_real_material() {
        assert_eq!(classify(STONE).0, Mat::Stone);
        assert_eq!(classify(ROOFS[0]).0, Mat::Tiles);
        assert_eq!(classify(ROOFS[2]).0, Mat::Thatch);
        assert_eq!(classify(WALLS[0]).0, Mat::Plaster);
        assert_eq!(classify([0.55, 0.4, 0.25, 1.0]).0, Mat::Planks);
        assert_eq!(classify(HAY).0, Mat::Thatch);
        assert_eq!(classify(WATER).0, Mat::Glass);
        // Nessun colore della tavolozza resta piatto.
        assert_ne!(classify([0.78, 0.18, 0.15, 1.0]).0, Mat::Plain);
    }
}
