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
        "casa-piccola" | "casa-naufrago" | "casa-naufrago-pietra" => Vec2::new(2.5, 2.0),
        "casa-grande" => Vec2::new(3.5, 3.0),
        "locanda" => Vec2::new(5.0, 3.25),
        "caserma" => Vec2::new(4.0, 3.0),
        "fabbro" => Vec2::new(5.0, 2.5),
        "forno" => Vec2::new(2.75, 2.25),
        "stalla" => Vec2::new(4.5, 2.5),
        "arcere" | "casa" | "casa-media" => Vec2::new(3.1, 2.5),
        "chiesa" => Vec2::new(3.7, 8.5),
        "faro" => Vec2::new(2.6, 2.6),
        "mulino" => Vec2::new(2.7, 2.7),
        "torre-rovina" => Vec2::new(2.3, 2.3),
        "grotta" => crate::places::CAVE_HALF,
        "santuario" => Vec2::new(2.8, 2.8),
        "campo-ladro" => Vec2::new(3.2, 3.2),
        "banditi" => Vec2::new(6.5, 5.8),
        "rovine" => Vec2::new(6.2, 5.2),
        "arco" => Vec2::new(4.2, 1.2),
        "carbonaia" => Vec2::new(2.0, 2.0),
        "palafitta" => Vec2::new(3.0, 3.0),
        _ => return None,
    })
}

/// Tipi di piazzamento che sono edifici.
pub fn is_building(kind: &str) -> bool {
    matches!(
        kind,
        "casa"
            | "casa-piccola"
            | "casa-naufrago"
            | "casa-naufrago-pietra"
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
            | "legnaia"
            | "porticina"
            | "mulino"
            | "campo-ladro"
            | "banditi"
            | "rovine"
            | "torre-rovina"
            | "grotta"
            | "santuario"
            | "arco"
            | "carbonaia"
            | "palafitta"
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
    /// Ante: 1 = porta, 2 = portone a due battenti che si aprono verso fuori (stalla).
    pub leaves: u8,
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

/// Finestra sul muro: centro (sulla faccia esterna), normale verso fuori e colore degli scuri.
/// Il vano è vero: dentro c'è un vetro che si alza (`world_props`), fuori gli scuri.
pub struct WindowSpot {
    pub center: Vec3,
    pub normal: Vec3,
    pub shutter: Col,
    /// Spessore del muro: la faccia interna sta a `center - normal * depth`.
    pub depth: f32,
}

/// Larghezza e altezza del vano delle finestre delle case.
pub const WINDOW_SIZE: Vec2 = Vec2::new(0.62, 0.86);

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
    /// Rampe delle scale: (centro, rotazione, dimensioni lunghezza/spessore/larghezza), `Ground`.
    pub ramps: Vec<(Vec3, Quat, Vec3)>,
    pub beds: Vec<Bed>,
    /// Luci interne, una per piano.
    pub lights: Vec<Vec3>,
    /// Mobili che contengono oggetti.
    pub containers: Vec<crate::containers::ContainerSpot>,
    /// Finestre con scuri apribili.
    pub windows: Vec<WindowSpot>,
    /// Focolari: (x, quota della cappa, z); il camino sale da qui.
    pub hearths: Vec<Vec3>,
    /// Bocche dei camini, da dove esce il fumo.
    pub chimneys: Vec<Vec3>,
    /// Falò sempre accesi.
    pub fires: Vec<Vec3>,
    /// Falde dei tetti come lastre inclinate (centro, rotazione, dimensioni), gronde comprese:
    /// fermano la camera, che altrimenti entrava nelle gronde sporgenti.
    pub roofs: Vec<(Vec3, Quat, Vec3)>,
    /// Teli rigidi ma scivolosi (tende delle bancarelle): si urtano ma non ci si appoggia, si
    /// scivola giù dai lati (`player::Slippery`).
    pub slides: Vec<(Vec3, Quat, Vec3)>,
    /// Scritte delle targhe: centro della tabella (sul muro davanti) e chiave della texture
    /// in `assets/textures/cartelli/<chiave>.png`.
    pub signs: Vec<(Vec3, &'static str)>,
    /// Punto subito dentro la porta.
    pub door_inside: Option<Vec3>,
    /// Per ogni piano, il percorso da dentro la porta al pianerottolo.
    pub routes: Vec<Vec<Vec3>>,
    /// Specchi d'acqua limpida (centro della superficie, semi-lati): pozzo.
    pub water: Vec<(Vec3, Vec2)>,
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
pub(crate) const TIMBER: Col = [0.3, 0.2, 0.12, 1.0];
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

/// Spessore delle lastre dei tetti per la camera (sotto la superficie della falda).
const SLAB_THICK: f32 = 0.2;

/// Lastra sotto una falda: dalla gronda `eave` al colmo `ridge` (punti a metà lunghezza),
/// larga `width` lungo `along` (l'asse del colmo). La faccia di sopra coincide con la falda.
fn roof_slab(eave: Vec3, ridge: Vec3, along: Vec3, width: f32) -> (Vec3, Quat, Vec3) {
    let slope = ridge - eave;
    let t = slope.normalize();
    let mut n = along.cross(t);
    if n.y < 0.0 {
        n = -n;
    }
    // Base destrorsa: X lungo il colmo, Y la normale (in su), Z lungo la falda.
    let z = along.cross(n);
    let rot = Quat::from_mat3(&Mat3::from_cols(along, n, z));
    let center = (eave + ridge) / 2.0 - n * (SLAB_THICK / 2.0);
    (center, rot, Vec3::new(width, SLAB_THICK, slope.length()))
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
    /// Falde dei tetti per la camera: (centro, rotazione, dimensioni), vedi `Interior::roofs`.
    slabs: Vec<(Vec3, Quat, Vec3)>,
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
            self.slabs.push(roof_slab(
                p((a0 + a1) / 2.0, y0, vs),
                p((a0 + a1) / 2.0, yr, vc),
                if ridge_x { Vec3::X } else { Vec3::Z },
                a1 - a0,
            ));
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

/// Finestra sul fronte (normale `n` = ±Z o ±X), nel vano già lasciato nel muro (`holed_wall`):
/// telaio di legno, davanzale di pietra fuori e di legno dentro. Vetro e imposte sono entità
/// apribili (`Interior.windows`).
fn window(b: &mut B, int: &mut Interior, c: Vec3, n: Vec3, shutter: Col) {
    let t = Vec3::Y.cross(n).normalize(); // verso destra guardando la finestra
    let (w, h) = (WINDOW_SIZE.x, WINDOW_SIZE.y);
    let depth = WALL_T;
    // Davanzale di pietra (fuori) e di legno (dentro).
    b.cuboid_c(
        c - Vec3::Y * (h / 2.0 + 0.04) + n * 0.06,
        t.abs() * (w + 0.3) + n.abs() * 0.16 + Vec3::Y * 0.08,
        STONE,
    );
    b.cuboid_c(
        c - n * (depth + 0.03) - Vec3::Y * (h / 2.0 + 0.02),
        t.abs() * (w + 0.16) + n.abs() * 0.1 + Vec3::Y * 0.04,
        TIMBER,
    );
    // Telaio sul filo esterno: montanti e architrave (il vetro scorre dietro, dentro il muro).
    let mid = c - n * 0.03;
    for s in [-1.0f32, 1.0] {
        b.cuboid_c(
            mid + t * (s * (w / 2.0 + 0.025)),
            t.abs() * 0.05 + n.abs() * 0.08 + Vec3::Y * (h + 0.05),
            TIMBER,
        );
    }
    b.cuboid_c(
        mid + Vec3::Y * (h / 2.0 + 0.025),
        t.abs() * (w + 0.1) + n.abs() * 0.08 + Vec3::Y * 0.05,
        TIMBER,
    );
    int.windows.push(WindowSpot {
        center: c,
        normal: n,
        shutter,
        depth,
    });
}

/// Muro pieno da `min` a `max` (lungo X) con i vani rettangolari `holes` (x0, x1, y0, y1): solo la
/// mesh; il collider resta il muro intero (il vano ha il vetro, e la camera non ci passa).
fn holed_wall(b: &mut B, min: Vec3, max: Vec3, holes: &[(f32, f32, f32, f32)], col: Col) {
    let holes: Vec<_> = holes
        .iter()
        .filter(|h| h.1 > min.x && h.0 < max.x && h.3 > min.y && h.2 < max.y)
        .collect();
    if holes.is_empty() {
        b.cuboid(min, max, col);
        return;
    }
    let mut xs = vec![min.x, max.x];
    let mut ys = vec![min.y, max.y];
    for h in &holes {
        xs.extend([h.0.clamp(min.x, max.x), h.1.clamp(min.x, max.x)]);
        ys.extend([h.2.clamp(min.y, max.y), h.3.clamp(min.y, max.y)]);
    }
    for v in [&mut xs, &mut ys] {
        v.sort_by(f32::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
    }
    // Per ogni colonna, le fasce piene consecutive diventano un solo blocco.
    for wx in xs.windows(2) {
        let cx = (wx[0] + wx[1]) / 2.0;
        let mut run: Option<f32> = None;
        for (k, wy) in ys.windows(2).enumerate() {
            let cy = (wy[0] + wy[1]) / 2.0;
            let open = holes
                .iter()
                .any(|h| cx > h.0 && cx < h.1 && cy > h.2 && cy < h.3);
            if !open && run.is_none() {
                run = Some(wy[0]);
            }
            let last = k + 2 == ys.len();
            if let Some(y_start) = run
                && (open || last)
            {
                let y_end = if open { wy[0] } else { wy[1] };
                b.cuboid(
                    Vec3::new(wx[0], y_start, min.z),
                    Vec3::new(wx[1], y_end, max.z),
                    col,
                );
                run = None;
            }
        }
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
        leaves: 1,
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
    // La casa di pietra del giocatore (QS-17): stessa casa, muri di pietra (il resto come prima).
    let picked = r.pick(&WALLS);
    let wall = if STONE_WALLS.with(|f| f.get()) {
        STONE
    } else {
        picked
    };
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
    // La stalla ha un portone largo al centro, per gli animali.
    let barn = kind == "stalla";
    let door_x = if floors > 1 || barn { 0.0 } else { -hw * 0.35 };
    let (door_w, door_h) = if barn { (2.4, 2.3) } else { (1.0, 2.05) };
    let t = WALL_T;
    // Finestre: sul fronte, a ogni piano (non accanto alla porta), e una sul retro.
    let per_floor = ((w - 1.2) / 2.2).floor().max(1.0) as usize;
    let mut front_windows: Vec<Vec2> = Vec::new();
    let mut back_windows: Vec<Vec2> = Vec::new();
    for f in 0..floors {
        let y = y0 + 1.25 + f as f32 * STOREY;
        for i in 0..per_floor {
            let x = -hw + 0.9 + (w - 1.8) * (i as f32 + 0.5) / per_floor as f32;
            if f == 0 && (x - door_x).abs() < door_w / 2.0 + 0.6 {
                continue;
            }
            front_windows.push(Vec2::new(x, y));
        }
        back_windows.push(Vec2::new(0.0, y));
    }
    let rect = |p: &Vec2| {
        let half = WINDOW_SIZE / 2.0;
        (p.x - half.x, p.x + half.x, p.y - half.y, p.y + half.y)
    };
    let front_holes: Vec<_> = front_windows.iter().map(rect).collect();
    let back_holes: Vec<_> = back_windows.iter().map(rect).collect();
    // Pareti cave: la facciata ha il vano della porta, fronte e retro i vani delle finestre.
    let mut wall_box = |b: &mut B, min: Vec3, max: Vec3| {
        let holes = if max.z >= hd - 1e-3 {
            &front_holes
        } else if min.z <= -hd + 1e-3 && max.z <= -hd + t + 1e-3 {
            &back_holes
        } else {
            &Vec::new()
        };
        holed_wall(b, min, max, holes, wall);
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
    // La falda parte oltre il muro: fra il bordo del muro e la falda resta una fessura alta
    // `rh · sporgenza / (mezza profondità + sporgenza)`; la si chiude con un listello di muro.
    let gap = rh * 0.4 / (hd + 0.4) + 0.02;
    for (z0, z1) in [(-hd, -hd + t), (hd - t, hd)] {
        b.cuboid(Vec3::new(-hw, wh, z0), Vec3::new(hw, wh + gap, z1), wall);
    }
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
    // Timpani visti da dentro: seguono la pendenza delle falde interne (che partono dalla gronda,
    // più basse del timpano esterno vicino ai muri). Il vecchio triangolo restava sotto le falde
    // vicino alle gronde e lasciava una fessura verso il cielo sopra la scala del sottotetto.
    let eave_in = wh - 0.1 + rh * 0.4 / (hd + 0.4) + 0.05;
    for s in [-1.0f32, 1.0] {
        let x = s * (hw - 0.02);
        let inward = Vec3::new(-s, 0.0, 0.0);
        b.tri(
            Vec3::new(x, eave_in, -hd),
            Vec3::new(x, eave_in, hd),
            Vec3::new(x, wh + rh, 0.0),
            inward,
            wall,
        );
        b.quad(
            Vec3::new(x, wh - 0.1, -hd),
            Vec3::new(x, wh - 0.1, hd),
            Vec3::new(x, eave_in, hd),
            Vec3::new(x, eave_in, -hd),
            inward,
            wall,
        );
    }
    door_front(b, int, door_x, hd, door_w, door_h);
    if barn && let Some(d) = int.door.as_mut() {
        d.leaves = 2;
    }
    for p in &front_windows {
        window(b, int, Vec3::new(p.x, p.y, hd), Vec3::Z, shutter);
    }
    for p in &back_windows {
        window(b, int, Vec3::new(p.x, p.y, -hd), Vec3::NEG_Z, shutter);
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
    // Sottotetto: soffitto e ripostiglio. Niente più pareti basse né fascia sotto la gronda: il
    // tetto a V lo chiude già (le falde hanno i loro collider), e in cima alla scala delle case
    // piccole il muretto, a pochi centimetri dalla rampa, non faceva entrare nel sottotetto
    // (segnalazioni 2026-11-18_23-46-45 e 23-48-25).
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
        rooms::furnish_attic(b, int, &attic_room, kind);
    }
    int.lights.push(Vec3::new(0.0, attic_y + 1.6, 0.0));
    // Camini: se la casa ha un focolare il camino sale da lì fino sopra il tetto; altrimenti
    // (forni, officine) resta il vecchio camino nel sottotetto.
    if int.hearths.is_empty() {
        for i in 0..chimneys {
            let x = if i == 0 { -hw + 1.0 } else { hw * 0.1 };
            chimney(b, x, 0.0, wh, wh + rh + 0.9);
            int.chimneys.push(Vec3::new(x, wh + rh + 1.0, 0.0));
            int.walls
                .push((Vec3::new(x, wh + 1.1, 0.0), Vec3::new(0.7, 2.2, 0.7)));
        }
    } else if chimneys > 0 {
        let h = int.hearths[0];
        // Quota del tetto sul lato del colmo del camino, più un metro sopra.
        let top = wh + rh * (1.0 - (h.z.abs() - 0.3) / hd) + 0.9;
        // Canna incassata nel muro di fondo: nelle stanze dei piani superiori sporge di 15 cm.
        let wall_z = h.z - 0.28 - 0.1;
        b.cuboid(
            Vec3::new(h.x - 0.35, h.y, wall_z - 0.25),
            Vec3::new(h.x + 0.35, top, wall_z + 0.25),
            STONE_DARK,
        );
        b.cuboid(
            Vec3::new(h.x - 0.45, top, wall_z - 0.35),
            Vec3::new(h.x + 0.45, top + 0.12, wall_z + 0.35),
            STONE,
        );
        int.chimneys.push(Vec3::new(h.x, top + 0.2, wall_z));
        int.walls.push((
            Vec3::new(h.x, (h.y + top) / 2.0, wall_z),
            Vec3::new(0.7, top - h.y, 0.5),
        ));
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
    // Il tetto della navata finisce contro il campanile: con la gronda di 0,5 m entrava nella torre
    // e da dentro se ne vedeva lo spigolo a V (segnalazione 2026-11-19_20-59-34).
    let roof_front = t_back - 0.5;
    b.gable(
        (-hw, hw, z_back, roof_front),
        wh,
        3.2,
        0.5,
        false,
        roof,
        wall,
    );
    // Interno del tetto a vista.
    b.force = Some(Mat::Planks);
    for s in [-1.0f32, 1.0] {
        b.quad(
            Vec3::new(s * (hw + 0.5), wh - 0.1, z_back),
            Vec3::new(s * (hw + 0.5), wh - 0.1, t_back),
            Vec3::new(0.0, wh + 3.1, t_back),
            Vec3::new(0.0, wh + 3.1, z_back),
            Vec3::new(-s, -1.0, 0.0),
            [0.4, 0.3, 0.2, 1.0],
        );
    }
    b.force = None;
    for (z, nz) in [(z_back + 0.02, 1.0f32), (z_front - t - 0.02, -1.0)] {
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
    let holes = rooms::square_helix(b, int, inner, y0, belfry, 0.8);
    rooms::slab_with_holes(b, int, inner, &holes, belfry);
    // Campana appesa al soffitto della cella.
    let bronze = [0.55, 0.42, 0.2, 1.0];
    // Una vera campana: tronco di cono a coste, con il bordo allargato, appesa alla trave.
    {
        use std::f32::consts::TAU;
        const SIDES: usize = 14;
        let ring = |y: f32, r: f32, k: usize| {
            let a = k as f32 / SIDES as f32 * TAU;
            Vec3::new(a.cos() * r, y, tz + a.sin() * r)
        };
        for k in 0..SIDES {
            b.beam(ring(8.9, 0.46, k), ring(9.45, 0.3, k), 0.05, bronze);
            b.beam(ring(9.45, 0.3, k), ring(9.95, 0.12, k), 0.05, bronze);
            b.beam(ring(8.9, 0.46, k), ring(8.9, 0.46, k + 1), 0.06, bronze);
        }
        b.cuboid_c(Vec3::new(0.0, 9.0, tz), Vec3::new(0.14, 0.3, 0.14), TIMBER);
        // Lanterna appesa: da qui viene la luce della cella.
        b.cuboid_c(
            Vec3::new(1.0, 9.4, tz - 1.0),
            Vec3::new(0.22, 0.3, 0.22),
            [1.0, 0.85, 0.5, 1.0],
        );
        b.cuboid_c(
            Vec3::new(1.0, 9.78, tz - 1.0),
            Vec3::new(0.04, 0.5, 0.04),
            TIMBER,
        );
    }
    // Il perno entra nella campana e la appende a una trave da muro a muro: prima tra campana e
    // perno restava un vuoto e la campana galleggiava (segnalazione 2026-11-19_20-59-34).
    b.cuboid_c(
        Vec3::new(0.0, 10.12, tz),
        Vec3::new(0.08, 0.5, 0.08),
        TIMBER,
    );
    b.cuboid_c(
        Vec3::new(0.0, 10.3, tz),
        Vec3::new(2.0 * (thw - tt), 0.2, 0.2),
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
    int.lights.push(Vec3::new(1.0, belfry + 1.4, tz - 1.0));
}

/// Larghezza e altezza della tabella con la scritta sulla targa (stesse proporzioni delle
/// texture dei cartelli, 512×192).
pub const SIGN_SIZE: Vec2 = Vec2::new(0.96, 0.36);

fn sign_arm(b: &mut B, int: &mut Interior, key: &'static str, col: Col, front: f32) {
    // Targa appesa al muro accanto alla porta, a mezza altezza del piano terra; la tabella
    // con la scritta (texture) la mette `world_props` davanti, al centro.
    b.cuboid_c(
        Vec3::new(1.9, 2.55, front + 0.05),
        Vec3::new(1.1, 0.5, 0.08),
        col,
    );
    int.signs.push((Vec3::new(1.9, 2.55, front + 0.095), key));
    for dx in [-0.45, 0.45] {
        b.cuboid_c(
            Vec3::new(1.9 + dx, 2.9, front + 0.03),
            Vec3::new(0.04, 0.3, 0.04),
            TIMBER,
        );
    }
}

/// Ruota di legno nel piano YZ: cerchio a segmenti e raggi.
fn wheel(b: &mut B, center: Vec3, radius: f32) {
    use std::f32::consts::TAU;
    const SIDES: usize = 16;
    let wood: Col = [0.34, 0.23, 0.13, 1.0];
    let rim = |k: usize| {
        let a = k as f32 / SIDES as f32 * TAU;
        center + Vec3::new(0.0, a.cos() * radius, a.sin() * radius)
    };
    for k in 0..SIDES {
        b.beam(rim(k), rim(k + 1), 0.05, wood);
    }
    for k in 0..SIDES / 2 {
        b.beam(rim(k), rim(k + SIDES / 2), 0.025, wood);
    }
    // Mozzo.
    b.cuboid_c(center, Vec3::new(0.14, 0.16, 0.16), TIMBER);
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
    // Collider come il carro che si vede: il pianale (con le ruote sotto) e le sponde. Prima era
    // un blocco unico alto 1,5 m e sopra si galleggiava (segnalazione 2026-11-19_20-36-21).
    let bed_top = y + 0.04;
    vec![
        (
            Vec3::new(0.0, bed_top / 2.0, -0.05),
            Vec3::new(1.6, bed_top, 2.3),
        ),
        (Vec3::new(-0.63, y + 0.22, 0.0), Vec3::new(0.08, 0.36, 2.2)),
        (Vec3::new(0.63, y + 0.22, 0.0), Vec3::new(0.08, 0.36, 2.2)),
        (Vec3::new(0.0, y + 0.22, -1.08), Vec3::new(1.3, 0.36, 0.08)),
    ]
}

/// Legnaia: tettoia a uno spiovente su quattro pali, con la legna spaccata accatastata sotto e una
/// scaletta sul fianco (Pietro ci sale la sera per guardare la porticina, ADR 0045).
fn woodshed(b: &mut B, r: &mut Rng) -> Vec<(Vec3, Vec3)> {
    const W: f32 = 1.6;
    const D: f32 = 1.0;
    const BACK: f32 = 2.1;
    const FRONT: f32 = 1.75;
    // La corteccia sul legno vero; la faccia tagliata, chiara, a colore pieno.
    let wood: Col = [0.42, 0.28, 0.14, 1.0];
    let bark: Col = [0.3, 0.21, 0.13, 1.0];
    for (x, z, h) in [(-W, -D, BACK), (W, -D, BACK), (-W, D, FRONT), (W, D, FRONT)] {
        b.cuboid_c(Vec3::new(x, h / 2.0, z), Vec3::new(0.12, h, 0.12), TIMBER);
    }
    // Il tetto di assi, inclinato verso il davanti, che sporge un poco.
    let tilt = ((BACK - FRONT) / (2.0 * D)).atan();
    let roof = Transform::from_xyz(0.0, (BACK + FRONT) / 2.0 + 0.06, 0.0)
        .with_rotation(Quat::from_rotation_x(tilt));
    for k in 0..9 {
        let x = -W - 0.15 + k as f32 * (2.0 * W + 0.3) / 8.5;
        let c = roof.transform_point(Vec3::new(x + 0.18, 0.0, 0.0));
        b.cuboid_c(c, Vec3::new(0.34, 0.04, 2.0 * D + 0.4), TIMBER);
    }
    // La tettoia è solida: ci si sale con la scaletta (segnalazione 2026-11-19_21-58-28).
    let edge = |z: f32| roof.transform_point(Vec3::new(0.0, 0.02, z));
    b.slabs.push(roof_slab(
        edge(D + 0.2),
        edge(-D - 0.2),
        Vec3::X,
        2.0 * W + 0.3,
    ));
    // Le cataste: tronchetti di testa, file sfalsate, contro la parete di fondo.
    let mut z = -D + 0.25;
    for row in 0..2 {
        let layers = 7 - row * 2;
        for layer in 0..layers {
            let y = 0.13 + layer as f32 * 0.22;
            let shift = if layer % 2 == 0 { 0.0 } else { 0.11 };
            let mut x = -W + 0.2 + shift;
            while x < W - 0.15 {
                let len = 0.42 + (r.next() % 10) as f32 / 100.0;
                b.force = Some(Mat::Planks);
                b.cuboid_c(Vec3::new(x, y, z), Vec3::new(0.2, 0.2, len), bark);
                // La faccia tagliata, più chiara, verso il davanti.
                b.force = Some(Mat::Plain);
                b.cuboid_c(
                    Vec3::new(x, y, z + len / 2.0 + 0.005),
                    Vec3::new(0.17, 0.17, 0.01),
                    wood,
                );
                b.force = None;
                x += 0.23;
            }
        }
        z += 0.55;
    }
    // Il ceppo con l'accetta piantata, davanti.
    b.force = Some(Mat::Planks);
    b.cuboid_c(
        Vec3::new(1.0, 0.25, D + 0.7),
        Vec3::new(0.5, 0.5, 0.5),
        bark,
    );
    b.force = None;
    b.beam(
        Vec3::new(1.0, 0.5, D + 0.7),
        Vec3::new(1.15, 0.95, D + 0.55),
        0.025,
        TIMBER,
    );
    // La scaletta appoggiata al fianco destro, fino al tetto.
    for s in [-0.22f32, 0.22] {
        b.beam(
            Vec3::new(W + 0.55, 0.0, s),
            Vec3::new(W + 0.12, BACK - 0.1, s - 0.2),
            0.03,
            TIMBER,
        );
    }
    for k in 1..7 {
        let t = k as f32 / 7.0;
        let at = Vec3::new(W + 0.55 - 0.43 * t, (BACK - 0.1) * t, -0.2 * t);
        b.cuboid_c(at, Vec3::new(0.04, 0.04, 0.48), TIMBER);
    }
    vec![
        (Vec3::new(0.0, 0.8, -D + 0.55), Vec3::new(2.0 * W, 1.6, 1.1)),
        (Vec3::new(1.0, 0.25, D + 0.7), Vec3::new(0.5, 0.5, 0.5)),
    ]
}

/// Porticina nelle mura: una piccola porta di assi ferrate con lo stipite di pietra, da appoggiare al
/// muro dal lato del paese (di notte ci passa chi esce di nascosto, ADR 0045).
fn postern(b: &mut B) -> Vec<(Vec3, Vec3)> {
    let stone: Col = [0.55, 0.53, 0.5, 1.0];
    let iron: Col = [0.2, 0.2, 0.22, 1.0];
    for x in [-0.55f32, 0.55] {
        b.cuboid_c(Vec3::new(x, 0.95, 0.0), Vec3::new(0.2, 1.9, 0.24), stone);
    }
    b.cuboid_c(Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.3, 0.22, 0.24), stone);
    b.cuboid_c(
        Vec3::new(0.0, 0.92, 0.04),
        Vec3::new(0.9, 1.84, 0.08),
        TIMBER,
    );
    for y in [0.4f32, 1.45] {
        b.cuboid_c(Vec3::new(0.0, y, 0.09), Vec3::new(0.86, 0.07, 0.02), iron);
    }
    b.cuboid_c(
        Vec3::new(0.3, 0.95, 0.11),
        Vec3::new(0.06, 0.06, 0.04),
        iron,
    );
    Vec::new()
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
    let mut out: Vec<(Vec3, Vec3)> = [-1.0f32, 1.0]
        .map(|s| {
            (
                Vec3::new(s * SPAN, TOP / 2.0, 0.0),
                Vec3::new(0.16, TOP, 0.16),
            )
        })
        .to_vec();
    // La rete stessa: una lastra sottile fra i pali, dai galleggianti alla traversa.
    out.push((
        Vec3::new(0.0, (0.47 + TOP) / 2.0, 0.0),
        Vec3::new(SPAN * 2.0 - 0.1, TOP - 0.47, 0.06),
    ));
    out
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
        // Telo con spessore (5 cm): di profilo resta visibile, da sotto si vede la stoffa.
        let th = Vec3::Y * 0.05;
        let (a, bb, c, d) = (
            Vec3::new(x0, 2.7, -1.3),
            Vec3::new(x1, 2.7, -1.3),
            Vec3::new(x1, 2.2, 1.35),
            Vec3::new(x0, 2.2, 1.35),
        );
        b.quad(a, bb, c, d, Vec3::Y, col);
        b.quad(a - th, bb - th, c - th, d - th, Vec3::NEG_Y, col);
        b.quad(a - th, bb - th, bb, a, Vec3::NEG_Z, col);
        b.quad(d - th, c - th, c, d, Vec3::Z, col);
        if i == 0 {
            b.quad(a - th, d - th, d, a, Vec3::NEG_X, col);
        }
        if i == n - 1 {
            b.quad(bb - th, c - th, c, bb, Vec3::X, col);
        }
        // Mantovana sul davanti: una fascia che pende dal bordo del telo.
        b.cuboid(Vec3::new(x0, 1.92, 1.33), Vec3::new(x1, 2.2, 1.37), col);
    }
    // Il telo è rigido ma scivoloso: non lo si attraversa, e chi ci salta sopra scivola giù dai
    // lati (segnalazione 2026-11-19_22-00-22).
    int.slides.push(roof_slab(
        Vec3::new(0.0, 2.2, 1.35),
        Vec3::new(0.0, 2.7, -1.3),
        Vec3::X,
        3.2,
    ));
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
pub(crate) fn tower_shell(
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
            _ => 1.15,
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
        let inner = [1.7, 1.7, 1.15, 1.15][i];
        inner_window(b, Vec3::new(n.x * (inner - 0.01), *h, 0.0), -n, (0.4, 0.6));
    }
    // Interni, fino al pavimento della lanterna.
    let floor1 = 3.6;
    let top = y + 0.25;
    rooms::lighthouse_interior(b, int, y0, floor1, top);
    // Balconata percorribile attorno alla lanterna, con ringhiera solida.
    for (min, max) in [
        (Vec3::new(-2.3, y, -2.3), Vec3::new(2.3, top, -1.15)),
        (Vec3::new(-2.3, y, 1.15), Vec3::new(2.3, top, 2.3)),
        (Vec3::new(-2.3, y, -1.15), Vec3::new(-1.15, top, 1.15)),
        (Vec3::new(1.15, y, -1.15), Vec3::new(2.3, top, 1.15)),
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
    // I vetri stanno appena fuori dal pozzo della chiocciola (±1,15): chi arriva in cima lungo la
    // parete non ci batte la testa.
    let (vi, vo) = (1.15, 1.21);
    pane(b, Vec3::new(-vo, top, -vo), Vec3::new(vo, head, -vi));
    pane(b, Vec3::new(-vo, top, -vi), Vec3::new(-vi, head, vo));
    pane(b, Vec3::new(vi, top, -vi), Vec3::new(vo, head, vo));
    // L'uscita sta davanti all'arrivo della chiocciola (angolo fronte-sinistra).
    pane(b, Vec3::new(-vi, top, vi), Vec3::new(-1.0, head, vo));
    pane(b, Vec3::new(0.5, top, vi), Vec3::new(vi, head, vo));
    // La lampada al centro.
    let lamp = (Vec3::new(-0.2, top, -0.2), Vec3::new(0.2, top + 1.1, 0.2));
    pane(b, lamp.0, lamp.1);
    for (x, z) in [(-1.18, -1.18), (1.18, -1.18), (-1.18, 1.18), (1.18, 1.18)] {
        b.cuboid_c(
            Vec3::new(x, y + 1.15, z),
            Vec3::new(0.12, 1.8, 0.12),
            TIMBER,
        );
    }
    b.cuboid(
        Vec3::new(-1.23, y + 2.05, -1.23),
        Vec3::new(1.23, y + 2.25, 1.23),
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
        // Casse, barile e bitte: solidi come si vedono (segnalazione 2026-11-18_23-12-06).
        (Vec3::new(-0.7, deck_y + 0.3, 1.0), Vec3::new(0.7, 0.6, 0.7)),
        (
            Vec3::new(-0.6, deck_y + 0.85, 1.0),
            Vec3::new(0.5, 0.5, 0.5),
        ),
        (
            Vec3::new(0.75, deck_y + 0.4, 2.4),
            Vec3::new(0.55, 0.8, 0.55),
        ),
        (
            Vec3::new(-1.1, deck_y + 0.18, 4.0),
            Vec3::new(0.2, 0.36, 0.2),
        ),
        (
            Vec3::new(1.1, deck_y + 0.18, 8.0),
            Vec3::new(0.2, 0.36, 0.2),
        ),
        (
            Vec3::new(-1.1, deck_y + 0.18, 11.4),
            Vec3::new(0.2, 0.36, 0.2),
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
    let v = Vec3::new;
    // Scafo vero invece di tre blocchi (segnalazione 2026-11-18_23-12-54): fondo stretto, fianchi
    // svasati, specchio di poppa dritto e prua che si chiude a punta. Le tavole sono fogli: il
    // materiale è a due facce e si vedono anche da dentro.
    let (bot_y, top_y) = (-0.2, 0.45);
    let (bot_x, top_x) = (0.3, 0.6);
    let (stern, bow) = (-1.4, 1.0);
    let (keel_tip, stem_tip) = (v(0.0, -0.1, 1.5), v(0.0, 0.52, 1.8));
    for s in [-1.0f32, 1.0] {
        // Fianco.
        b.quad(
            v(s * bot_x, bot_y, stern + 0.1),
            v(s * bot_x, bot_y, bow + 0.2),
            v(s * top_x, top_y, bow),
            v(s * top_x, top_y, stern),
            v(s, 0.4, 0.0),
            hull,
        );
        // Prua: il fianco si chiude sulla ruota di prua.
        b.quad(
            v(s * bot_x, bot_y, bow + 0.2),
            keel_tip,
            stem_tip,
            v(s * top_x, top_y, bow),
            v(s, 0.3, 0.6),
            hull,
        );
        // Falchetta (il bordo in cima al fianco), di legno chiaro.
        b.beam(
            v(s * top_x, top_y, stern),
            v(s * top_x, top_y, bow),
            0.035,
            wood,
        );
        b.beam(v(s * top_x, top_y, bow), stem_tip, 0.035, wood);
    }
    // Fondo e specchio di poppa.
    b.quad(
        v(-bot_x, bot_y, stern + 0.1),
        v(bot_x, bot_y, stern + 0.1),
        v(bot_x, bot_y, bow + 0.2),
        v(-bot_x, bot_y, bow + 0.2),
        Vec3::NEG_Y,
        hull,
    );
    b.tri(
        v(-bot_x, bot_y, bow + 0.2),
        v(bot_x, bot_y, bow + 0.2),
        keel_tip,
        Vec3::NEG_Y,
        hull,
    );
    b.quad(
        v(-bot_x, bot_y, stern + 0.1),
        v(bot_x, bot_y, stern + 0.1),
        v(top_x, top_y, stern),
        v(-top_x, top_y, stern),
        Vec3::NEG_Z,
        hull,
    );
    // Pagliolato sul fondo, panche e remi.
    b.cuboid(
        v(-bot_x + 0.04, bot_y + 0.02, stern + 0.2),
        v(bot_x - 0.04, bot_y + 0.06, bow),
        [0.35, 0.25, 0.15, 1.0],
    );
    for (z, half) in [(-0.6f32, 0.52), (0.2, 0.5)] {
        b.cuboid_c(v(0.0, 0.3, z), v(2.0 * half, 0.06, 0.28), wood);
    }
    for x in [-0.45f32, 0.45] {
        b.cuboid_c(v(x, 0.38, -0.2), v(0.05, 0.05, 1.7), wood);
    }
}

/// Pozzo: anello di pietra chiuso su tutti i lati, acqua limpida sotto il bordo, due montanti,
/// piccolo tetto e secchio.
fn well(b: &mut B, int: &mut Interior) -> Vec<(Vec3, Vec3)> {
    const OUT: f32 = 1.2;
    const IN: f32 = 0.8;
    const RIM: f32 = 0.9;
    // Quattro muri del parapetto (canna cava), più la copertina che sporge un poco.
    for (min, max) in [
        (Vec3::new(-OUT, -1.0, -OUT), Vec3::new(OUT, RIM, -IN)),
        (Vec3::new(-OUT, -1.0, IN), Vec3::new(OUT, RIM, OUT)),
        (Vec3::new(-OUT, -1.0, -IN), Vec3::new(-IN, RIM, IN)),
        (Vec3::new(IN, -1.0, -IN), Vec3::new(OUT, RIM, IN)),
    ] {
        b.cuboid(min, max, STONE);
    }
    for (min, max) in [
        (
            Vec3::new(-OUT - 0.04, RIM, -OUT - 0.04),
            Vec3::new(OUT + 0.04, RIM + 0.08, -IN + 0.02),
        ),
        (
            Vec3::new(-OUT - 0.04, RIM, IN - 0.02),
            Vec3::new(OUT + 0.04, RIM + 0.08, OUT + 0.04),
        ),
        (
            Vec3::new(-OUT - 0.04, RIM, -IN + 0.02),
            Vec3::new(-IN + 0.02, RIM + 0.08, IN - 0.02),
        ),
        (
            Vec3::new(IN - 0.02, RIM, -IN + 0.02),
            Vec3::new(OUT + 0.04, RIM + 0.08, IN - 0.02),
        ),
    ] {
        b.cuboid(min, max, STONE_DARK);
    }
    // L'acqua sta giù in fondo alla canna, al livello della vasca nella Camera di Risonanza
    // (la canna scende cava dentro la colonna, `places::resonance_chamber`).
    int.water.push((
        Vec3::new(0.0, crate::places::WELL_WATER_Y, 0.0),
        Vec2::splat(IN),
    ));
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

/// Materiale dell'acqua ferma e limpida (pozzo, pozza del risveglio): come quella del ruscello,
/// ma più trasparente e liscia, senza onde.
pub fn clear_water_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(0.1, 0.3, 0.36, 0.55),
        reflectance: 0.5,
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.08,
        // Si vede anche da sotto (prima il pozzo, guardato dal basso, perdeva l'acqua).
        double_sided: true,
        cull_mode: None,
        ..Default::default()
    }
}

thread_local! {
    /// `house` alza muri di pietra invece che d'intonaco (la casa di pietra del giocatore).
    static STONE_WALLS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
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
        // La casa del giocatore è una casa piccola come le altre (ADR 0042); ampliata, ha i muri di
        // pietra (ADR 0052).
        "casa-naufrago-pietra" => {
            STONE_WALLS.with(|f| f.set(true));
            house(
                &mut b,
                &mut r,
                &mut int,
                "casa-piccola",
                5.0,
                4.0,
                2,
                false,
                1,
                seed,
            );
            STONE_WALLS.with(|f| f.set(false));
            (5.0, 4.0, Vec::new())
        }
        "casa-piccola" | "casa-naufrago" => {
            house(
                &mut b,
                &mut r,
                &mut int,
                "casa-piccola",
                5.0,
                4.0,
                2,
                false,
                1,
                seed,
            );
            (5.0, 4.0, Vec::new())
        }
        "casa-grande" => {
            house(&mut b, &mut r, &mut int, kind, 7.0, 6.0, 3, true, 2, seed);
            (7.0, 6.0, Vec::new())
        }
        "locanda" => {
            house(&mut b, &mut r, &mut int, kind, 10.0, 6.5, 3, true, 2, seed);
            // Targa appesa al muro accanto alla porta.
            sign_arm(&mut b, &mut int, "locanda", [0.55, 0.3, 0.15, 1.0], 3.25);
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
            sign_arm(&mut b, &mut int, "fabbro", [0.35, 0.36, 0.4, 1.0], 2.5);
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
            // Targa appesa al muro accanto alla porta.
            sign_arm(&mut b, &mut int, "forno", [0.42, 0.28, 0.16, 1.0], 2.25);
            (5.5, 4.5, Vec::new())
        }
        "stalla" => {
            // Il portone a due battenti è la porta della stalla (`house` lo apre al centro).
            house(&mut b, &mut r, &mut int, kind, 9.0, 5.0, 1, false, 0, seed);
            (9.0, 5.0, Vec::new())
        }
        "arcere" => {
            house(&mut b, &mut r, &mut int, kind, 6.2, 5.0, 2, true, 1, seed);
            sign_arm(&mut b, &mut int, "arcere", [0.25, 0.5, 0.3, 1.0], 2.5);
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
        // Arco di pietra del Nord: due piloni e un architrave, a cavallo del sentiero.
        "arco" => {
            for s in [-1.0f32, 1.0] {
                b.cuboid_c(
                    Vec3::new(s * 3.1, 2.6, 0.0),
                    Vec3::new(1.5, 5.6, 1.7),
                    STONE,
                );
                b.cuboid_c(
                    Vec3::new(s * 3.1, 0.2, 0.0),
                    Vec3::new(2.0, 0.8, 2.2),
                    STONE_DARK,
                );
                b.cuboid_c(
                    Vec3::new(s * 2.2, 5.0, 0.0),
                    Vec3::new(0.9, 0.8, 1.5),
                    STONE_DARK,
                );
            }
            b.cuboid_c(Vec3::new(0.0, 5.95, 0.0), Vec3::new(8.6, 1.1, 1.9), STONE);
            b.cuboid_c(
                Vec3::new(0.0, 6.75, 0.0),
                Vec3::new(5.0, 0.5, 1.5),
                STONE_DARK,
            );
            (
                8.4,
                2.4,
                vec![
                    (Vec3::new(-3.1, 2.8, 0.0), Vec3::new(2.0, 5.8, 2.2)),
                    (Vec3::new(3.1, 2.8, 0.0), Vec3::new(2.0, 5.8, 2.2)),
                ],
            )
        }
        // Carbonaia: montagnola di legna coperta di terra e muschio, con la bocca in cima che fuma.
        "carbonaia" => {
            b.force = Some(Mat::Plain);
            b.pyramid(Vec3::new(0.0, -0.1, 0.0), 1.9, 2.3, [0.2, 0.19, 0.15, 1.0]);
            b.pyramid(Vec3::new(0.0, 1.5, 0.0), 0.7, 0.75, [0.08, 0.08, 0.08, 1.0]);
            b.force = None;
            for (x, z) in [(1.7f32, 0.6f32), (-1.5, -1.1)] {
                b.cuboid_c(Vec3::new(x, 0.25, z), Vec3::new(0.9, 0.5, 0.35), TIMBER);
            }
            int.chimneys.push(Vec3::new(0.0, 2.2, 0.0));
            (
                3.8,
                3.8,
                vec![(Vec3::new(0.0, 0.9, 0.0), Vec3::new(2.6, 1.8, 2.6))],
            )
        }
        // Palafitta: capanna su un impalcato sopra l'acqua, con la passerella verso riva (+Z).
        "palafitta" => {
            let deck = 1.5;
            for (x, z) in [
                (-2.3f32, -2.3f32),
                (2.3, -2.3),
                (-2.3, 2.3),
                (2.3, 2.3),
                (0.0, 0.0),
            ] {
                b.cuboid_c(
                    Vec3::new(x, deck / 2.0 - 1.2, z),
                    Vec3::new(0.26, deck + 2.4, 0.26),
                    TIMBER,
                );
            }
            b.force = Some(Mat::Planks);
            b.cuboid_c(
                Vec3::new(0.0, deck, 0.0),
                Vec3::new(5.4, 0.16, 5.4),
                [0.46, 0.34, 0.2, 1.0],
            );
            // passerella
            b.cuboid_c(
                Vec3::new(0.0, deck, 5.2),
                Vec3::new(1.3, 0.14, 5.0),
                [0.44, 0.32, 0.19, 1.0],
            );
            // capanna (aperta sul davanti)
            let wall: Col = [0.52, 0.4, 0.24, 1.0];
            b.cuboid_c(
                Vec3::new(0.0, deck + 1.1, -2.0),
                Vec3::new(4.2, 2.1, 0.14),
                wall,
            );
            for s in [-1.0f32, 1.0] {
                b.cuboid_c(
                    Vec3::new(s * 2.05, deck + 1.1, -0.6),
                    Vec3::new(0.14, 2.1, 2.9),
                    wall,
                );
            }
            b.force = None;
            b.gable(
                (-2.3, 2.3, -2.2, 1.0),
                deck + 2.15,
                1.3,
                0.3,
                true,
                [0.62, 0.55, 0.3, 1.0],
                wall,
            );
            for z in [3.0f32, 5.0, 7.0] {
                b.cuboid_c(
                    Vec3::new(0.0, deck / 2.0 - 0.8, z),
                    Vec3::new(0.18, deck + 1.6, 0.18),
                    TIMBER,
                );
            }
            int.ground
                .push((Vec3::new(0.0, deck, 0.0), Vec3::new(5.4, 0.16, 5.4)));
            int.ground
                .push((Vec3::new(0.0, deck, 5.2), Vec3::new(1.3, 0.14, 5.0)));
            int.walls
                .push((Vec3::new(0.0, deck + 1.1, -2.0), Vec3::new(4.2, 2.1, 0.2)));
            for s in [-1.0f32, 1.0] {
                int.walls.push((
                    Vec3::new(s * 2.05, deck + 1.1, -0.6),
                    Vec3::new(0.2, 2.1, 2.9),
                ));
            }
            (5.4, 5.4, Vec::new())
        }
        "pagliaio" => {
            b.cuboid(
                Vec3::new(-0.9, -0.2, -0.9),
                Vec3::new(0.9, 1.1, 0.9),
                [0.85, 0.7, 0.3, 1.0],
            );
            let (base, half, h) = (1.1, 1.15, 1.4);
            b.pyramid(Vec3::new(0.0, base, 0.0), half, h, [0.8, 0.62, 0.22, 1.0]);
            // Il cono con le sue quattro falde: prima un cubo alto 1,8 m e sui versanti le gambe
            // sprofondavano nel fieno (segnalazione 2026-11-19_20-38-31).
            let apex = Vec3::new(0.0, base + h, 0.0);
            for (dir, along) in [
                (Vec3::Z, Vec3::X),
                (Vec3::NEG_Z, Vec3::X),
                (Vec3::X, Vec3::Z),
                (Vec3::NEG_X, Vec3::Z),
            ] {
                let eave = Vec3::new(0.0, base, 0.0) + dir * half;
                b.slabs.push(roof_slab(eave, apex, along, 2.0 * half));
            }
            (
                1.8,
                1.8,
                vec![(Vec3::new(0.0, 0.45, 0.0), Vec3::new(1.8, 1.3, 1.8))],
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
        "pozzo" => {
            // Sotto il pozzo della piazza, la Camera di Risonanza (QS-12, ADR 0052).
            crate::places::resonance_chamber(&mut b, &mut int);
            (2.4, 2.4, well(&mut b, &mut int))
        }
        "mulino" => {
            house(&mut b, &mut r, &mut int, kind, 5.4, 5.4, 2, false, 0, seed);
            // Albero delle pale: esce dalla facciata, sopra il tetto della porta.
            b.beam(
                Vec3::new(0.0, 6.0, 2.4),
                Vec3::new(0.0, 6.0, 3.7),
                0.12,
                TIMBER,
            );
            (5.4, 5.4, Vec::new())
        }
        k if crate::places::is_place(k) => crate::places::build(k, &mut b, &mut int),
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
        "legnaia" => (3.4, 2.2, woodshed(&mut b, &mut r)),
        "porticina" => (1.2, 0.3, postern(&mut b)),
        // "casa" e "casa-media"
        _ => {
            let timber = r.next().is_multiple_of(2);
            house(&mut b, &mut r, &mut int, kind, 6.2, 5.0, 2, timber, 1, seed);
            (6.2, 5.0, Vec::new())
        }
    };
    int.roofs = std::mem::take(&mut b.slabs);
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
            "legnaia",
            "porticina",
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

    /// Triangoli (mondo locale) di tutte le parti di un edificio.
    fn triangles(b: &Building) -> Vec<[Vec3; 3]> {
        use bevy::mesh::VertexAttributeValues;
        let mut out = Vec::new();
        for (_, mesh) in &b.parts {
            let Some(VertexAttributeValues::Float32x3(pos)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                continue;
            };
            let idx: Vec<u32> = match mesh.indices() {
                Some(Indices::U32(i)) => i.clone(),
                Some(Indices::U16(i)) => i.iter().map(|v| u32::from(*v)).collect(),
                None => (0..pos.len() as u32).collect(),
            };
            let (chunks, _) = idx.as_chunks::<3>();
            for t in chunks {
                let p = |k: u32| Vec3::from_array(pos[k as usize]);
                out.push([p(t[0]), p(t[1]), p(t[2])]);
            }
        }
        out
    }

    /// Il raggio colpisce almeno un triangolo (da entrambi i lati)?
    fn ray_hits(tris: &[[Vec3; 3]], o: Vec3, d: Vec3) -> bool {
        tris.iter().any(|t| {
            let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
            let h = d.cross(e2);
            let a = e1.dot(h);
            if a.abs() < 1e-7 {
                return false;
            }
            let f = 1.0 / a;
            let sv = o - t[0];
            let u = f * sv.dot(h);
            if !(0.0..=1.0).contains(&u) {
                return false;
            }
            let q = sv.cross(e1);
            let v = f * d.dot(q);
            if v < 0.0 || u + v > 1.0 {
                return false;
            }
            f * e2.dot(q) > 1e-3
        })
    }

    #[test]
    fn windows_are_clear_and_chimneys_sit_on_hearths() {
        for kind in [
            "casa-piccola",
            "casa-media",
            "casa-grande",
            "locanda",
            "caserma",
            "forno",
            "arcere",
        ] {
            for seed in [1u32, 7, 33] {
                let int = model(kind, seed).interior;
                assert!(
                    !int.windows.is_empty() || kind == "forno",
                    "{kind}: senza finestre"
                );
                // Nessun mobile entro 0,5 m davanti a una finestra, a quota di scuri e vetro.
                for w in &int.windows {
                    let inward = -w.normal;
                    let probe = w.center + inward * 0.5;
                    for (c, size) in int.furniture.iter().chain(&int.walls) {
                        let (lo, hi) = (*c - *size / 2.0, *c + *size / 2.0);
                        let blocks_y = lo.y < w.center.y + 0.5 && hi.y > w.center.y - 0.5;
                        let blocks_xz = lo.x - 0.15 <= probe.x
                            && probe.x <= hi.x + 0.15
                            && lo.z - 0.15 <= probe.z
                            && probe.z <= hi.z + 0.15;
                        // Le pareti con la finestra stessa non contano: sono più lontane di 0,4 m.
                        assert!(
                            !(blocks_y && blocks_xz),
                            "{kind}/{seed}: ostacolo davanti alla finestra in {:?}: {:?} {:?}",
                            w.center,
                            c,
                            size
                        );
                    }
                }
                // Ogni camino con focolare parte dalla quota della cappa e sta sopra il focolare.
                for (i, mouth) in int.chimneys.iter().enumerate() {
                    if let Some(h) = int.hearths.get(i) {
                        assert!(
                            (mouth.x - h.x).abs() < 1e-3,
                            "{kind}: camino fuori dal focolare"
                        );
                        assert!(mouth.y > h.y + 1.0);
                    }
                }
            }
        }
    }

    #[test]
    fn roof_slabs_follow_the_slopes_out_to_the_eaves() {
        let m = model("casa-piccola", 7);
        assert_eq!(m.interior.roofs.len(), 2);
        for (c, rot, size) in &m.interior.roofs {
            // La faccia di sopra della lastra passa per la gronda e per il colmo.
            let up = *rot * Vec3::Y;
            let along_slope = *rot * Vec3::Z;
            assert!(up.y > 0.3, "normale in su: {up:?}");
            let top = *c + up * size.y / 2.0;
            for end in [-0.5f32, 0.5] {
                let p = top + along_slope * size.z * end;
                // Un estremo è la gronda (fuori dal muro di 0,4 m), l'altro il colmo (z = 0).
                assert!(
                    p.z.abs() < 0.05 || (p.z.abs() - (2.0 + 0.4)).abs() < 0.05,
                    "{p:?}"
                );
            }
            // Gronde comprese: più larga della casa (5 m) di due sporgenze.
            assert!((size.x - 5.8).abs() < 0.01, "{size:?}");
        }
    }

    #[test]
    fn roofs_are_closed_seen_from_inside() {
        for kind in [
            "casa-piccola",
            "casa-media",
            "casa-grande",
            "locanda",
            "caserma",
            "forno",
            "arcere",
        ] {
            let b = model(kind, 7);
            let tris = triangles(&b);
            let inner = interior_half(kind).expect("edificio con interno");
            let (hx, hz) = (inner.x * 0.7, inner.y * 0.7);
            let mut leaks = Vec::new();
            // Piano terra, primo piano e sottotetto delle case a due piani (a 1 m dal pavimento):
            // lì c'era la fessura sopra la scala (segnalazione 31).
            let attic2 = GROUND_FLOOR + 2.0 * STOREY + 0.02 + 1.0;
            for y in [1.2f32, 3.6, attic2] {
                for ix in -2..=2 {
                    for iz in -2..=2 {
                        let o = Vec3::new(hx * ix as f32 / 2.0, y, hz * iz as f32 / 2.0);
                        for az in 0..12 {
                            let a = az as f32 / 12.0 * std::f32::consts::TAU;
                            // Verso l'alto, tra 25° e 90° sull'orizzonte.
                            for el in [0.45f32, 0.8, 1.2, 1.5] {
                                let d = Vec3::new(a.cos() * el.cos(), el.sin(), a.sin() * el.cos());
                                // Dal piano terra verso la facciata c'è la porta, che è un'apertura vera.
                                if y < 2.0 && d.z > 0.0 {
                                    continue;
                                }
                                // Le finestre sono vani veri (ADR 0060): un raggio che esce da
                                // un vano non è un buco nel tetto.
                                let through_window = b.interior.windows.iter().any(|w| {
                                    let n = w.normal;
                                    let along = d.dot(n);
                                    if along <= 1e-4 {
                                        return false;
                                    }
                                    let k = (w.center - o).dot(n) / along;
                                    let q = o + d * k - w.center;
                                    let side = q - n * q.dot(n);
                                    side.y.abs() <= WINDOW_SIZE.y / 2.0 + 0.01
                                        && (side - Vec3::Y * side.y).length()
                                            <= WINDOW_SIZE.x / 2.0 + 0.01
                                });
                                if !through_window && !ray_hits(&tris, o, d) {
                                    leaks.push(format!(
                                        "({:.1},{:.1},{:.1}) dir {:.2},{:.2},{:.2}",
                                        o.x, o.y, o.z, d.x, d.y, d.z
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            assert!(
                leaks.is_empty(),
                "{kind}: {} raggi escono, es. {:?}",
                leaks.len(),
                &leaks[..leaks.len().min(3)]
            );
        }
    }
}
