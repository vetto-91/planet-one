//! Abeti: tronco con corteccia PBR e rami a "carte" con i rametti fotografati (texture CC0 Poly Haven).
//!
//! I modelli scansionati hanno milioni di triangoli: qui bastano circa 300 triangoli per albero, una
//! mesh condivisa da tutti (Bevy li disegna in batch).

use crate::pbr;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::f32::consts::TAU;

/// Altezza dell'abete (m) a scala 1.
pub const FIR_HEIGHT: f32 = 6.5;
/// Riquadri (u0, u1, v0, v1) dei rametti nell'atlante: la punta è in alto.
const TWIGS: [[f32; 4]; 4] = [
    [0.303, 0.647, 0.397, 0.778],
    [0.632, 0.960, 0.449, 0.833],
    [0.649, 0.940, 0.037, 0.372],
    [0.187, 0.432, 0.044, 0.314],
];

#[derive(Default)]
struct Buf {
    pos: Vec<[f32; 3]>,
    nor: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    idx: Vec<u32>,
}

impl Buf {
    fn vertex(&mut self, p: Vec3, n: Vec3, uv: [f32; 2]) -> u32 {
        self.pos.push(p.to_array());
        self.nor.push(n.to_array());
        self.uv.push(uv);
        self.pos.len() as u32 - 1
    }

    fn finish(self) -> Mesh {
        let mut m = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.nor);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uv);
        m.insert_indices(Indices::U32(self.idx));
        m.with_generated_tangents().expect("mesh dell'abete con UV")
    }
}

/// Tronco conico da terra alla cima, con corteccia ripetuta in metri.
fn trunk() -> Mesh {
    let mut b = Buf::default();
    let (sides, rings) = (8u32, 5u32);
    for r in 0..=rings {
        let t = r as f32 / rings as f32;
        let y = -0.4 + t * (FIR_HEIGHT + 0.4);
        let radius = 0.24 * (1.0 - t).powf(0.8) + 0.015;
        for s in 0..=sides {
            let a = s as f32 / sides as f32 * TAU;
            let n = Vec3::new(a.cos(), 0.12, a.sin()).normalize();
            let p = Vec3::new(a.cos() * radius, y, a.sin() * radius);
            b.vertex(p, n, [s as f32 / sides as f32 * 1.5, y / 1.5]);
        }
    }
    for r in 0..rings {
        for s in 0..sides {
            let i = r * (sides + 1) + s;
            let up = i + sides + 1;
            b.idx.extend([i, up, i + 1, i + 1, up, up + 1]);
        }
    }
    b.finish()
}

/// Ramo: una carta in due segmenti (si piega verso il basso), normali morbide verso l'alto.
fn card(b: &mut Buf, base: Vec3, dir: Vec3, len: f32, droop: f32, twig: usize) {
    wide_card(b, base, dir, len, droop, twig, 0.9);
}

/// Come [`card`], con la larghezza in proporzione alla lunghezza: da lontano servono carte più piene.
fn wide_card(b: &mut Buf, base: Vec3, dir: Vec3, len: f32, droop: f32, twig: usize, wide: f32) {
    let [u0, u1, v0, v1] = TWIGS[twig];
    let side = dir.cross(Vec3::Y).normalize_or_zero();
    let width = len * wide;
    let vmid = (v0 + v1) / 2.0;
    let mid = base + dir * (len * 0.5);
    let tip = mid + (dir + Vec3::NEG_Y * droop).normalize() * (len * 0.5);
    let up = side.cross(dir).normalize_or_zero();
    let n = (up.abs() * 0.5 + Vec3::Y).normalize();
    let row = |p: Vec3, v: f32, w: f32, b: &mut Buf| {
        (
            b.vertex(p - side * w, n, [u0, v]),
            b.vertex(p + side * w, n, [u1, v]),
        )
    };
    let (a0, a1) = row(base, v1, width * 0.25, b);
    let (m0, m1) = row(mid, vmid, width * 0.5, b);
    let (t0, t1) = row(tip, v0, width * 0.3, b);
    b.idx
        .extend([a0, a1, m0, m0, a1, m1, m0, m1, t0, t0, m1, t1]);
}

/// Chioma: palchi di rami dalla base alla cima, lunghi in basso e corti in alto. `seed` varia l'albero.
fn foliage(seed: f32) -> Mesh {
    let mut b = Buf::default();
    let levels = 15;
    for l in 0..levels {
        let t = l as f32 / (levels - 1) as f32;
        let y = 1.3 + t * (FIR_HEIGHT - 1.5);
        let len = 2.1 * (1.0 - t).powf(0.85) + 0.32;
        let count = 8 - (t * 4.0) as usize;
        let lift = 0.30 * t - 0.05;
        for c in 0..count {
            let a = (c as f32 + l as f32 * 0.37 + seed) / count as f32 * TAU
                + (l as f32 * 7.3 + c as f32 * 3.1 + seed).sin() * 0.25;
            let dir = Vec3::new(a.cos(), lift, a.sin()).normalize();
            let k = (l + c) % TWIGS.len();
            let base = Vec3::new(a.cos() * 0.08, y, a.sin() * 0.08);
            card(&mut b, base, dir, len, 0.55 - 0.3 * t, k);
        }
    }
    // Punta verticale.
    for a in [0.0f32, 1.6] {
        let dir = Vec3::new(a.cos() * 0.08, 1.0, a.sin() * 0.08).normalize();
        card(&mut b, Vec3::Y * (FIR_HEIGHT - 0.6), dir, 0.9, 0.0, 0);
    }
    b.finish()
}

/// Chioma semplificata dell'abete, per i boschi lontani: pochi rami grandi.
pub fn fir_foliage_far(seed: f32) -> Mesh {
    let mut b = Buf::default();
    let levels = 8;
    for l in 0..levels {
        let t = l as f32 / (levels - 1) as f32;
        let y = 1.4 + t * (FIR_HEIGHT - 1.7);
        let len = 2.5 * (1.0 - t).powf(0.85) + 0.55;
        for c in 0..5 {
            let a = (c as f32 + l as f32 * 0.5 + seed) / 5.0 * TAU;
            let dir = Vec3::new(a.cos(), 0.05, a.sin()).normalize();
            let k = (l + c) % TWIGS.len();
            wide_card(&mut b, Vec3::new(0.0, y, 0.0), dir, len, 0.4, k, 2.0);
        }
    }
    b.finish()
}

/// Tronco di latifoglia: corto, largo alla base, con tre rami principali.
pub fn broadleaf_trunk() -> Mesh {
    let mut b = Buf::default();
    let mut limb = |from: Vec3, to: Vec3, r0: f32, r1: f32| {
        let (sides, rings) = (6u32, 3u32);
        let base = b.pos.len() as u32;
        let axis = (to - from).normalize_or(Vec3::Y);
        let x = axis.cross(Vec3::Z).normalize_or(Vec3::X);
        let z = x.cross(axis).normalize_or(Vec3::Z);
        for r in 0..=rings {
            let t = r as f32 / rings as f32;
            let c = from.lerp(to, t);
            let radius = r0 + (r1 - r0) * t;
            for k in 0..=sides {
                let a = k as f32 / sides as f32 * TAU;
                let n = x * a.cos() + z * a.sin();
                b.vertex(
                    c + n * radius,
                    n,
                    [k as f32 / sides as f32 * 1.5, c.y / 1.5],
                );
            }
        }
        for r in 0..rings {
            for k in 0..sides {
                let i = base + r * (sides + 1) + k;
                let up = i + sides + 1;
                b.idx.extend([i, up, i + 1, i + 1, up, up + 1]);
            }
        }
    };
    let fork = Vec3::Y * 2.6;
    limb(Vec3::NEG_Y * 0.4, fork, 0.3, 0.2);
    for a in [0.4f32, 2.5, 4.6] {
        let tip = fork + Vec3::new(a.cos() * 1.5, 1.9, a.sin() * 1.5);
        limb(fork, tip, 0.17, 0.05);
    }
    b.finish()
}

/// Chioma di latifoglia: una massa arrotondata di carte, rivolte verso l'esterno.
pub fn broadleaf_foliage(seed: f32, cards: usize) -> Mesh {
    let mut b = Buf::default();
    let size = if cards > 100 { 1.25 } else { 2.2 };
    let center = Vec3::Y * 4.3;
    for i in 0..cards {
        // Punti distribuiti sulla sfera (spirale di Fibonacci), schiacciata in altezza.
        let t = (i as f32 + 0.5) / cards as f32;
        let phi = (1.0 - 2.0 * t).acos();
        let theta = i as f32 * 2.399_963 + seed;
        let d = Vec3::new(
            phi.sin() * theta.cos(),
            phi.cos() * 0.72,
            phi.sin() * theta.sin(),
        );
        let shell = 1.5 + 0.7 * ((i as f32 * 7.77 + seed).sin() * 0.5 + 0.5);
        let dir = (d + Vec3::Y * 0.15).normalize();
        card(
            &mut b,
            center + d * shell * 0.55,
            dir,
            size,
            0.35,
            i % TWIGS.len(),
        );
    }
    b.finish()
}

/// Cespuglio: poche carte da terra verso l'esterno.
pub fn bush(seed: f32) -> Mesh {
    let mut b = Buf::default();
    for i in 0..14 {
        let a = i as f32 * 2.399_963 + seed;
        let dir = Vec3::new(a.cos(), 0.55 + 0.35 * (i as f32 * 1.3).sin(), a.sin()).normalize();
        card(&mut b, Vec3::Y * 0.1, dir, 0.85, 0.45, i % TWIGS.len());
    }
    b.finish()
}

/// Mesh e materiali degli abeti.
pub struct FirAssets {
    pub trunk: Handle<Mesh>,
    pub foliage: [Handle<Mesh>; 3],
    pub bark: Handle<StandardMaterial>,
    pub needles: [Handle<StandardMaterial>; 3],
    pub snow: Handle<StandardMaterial>,
}

impl FirAssets {
    pub fn new(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        server: &AssetServer,
    ) -> Self {
        let twig = pbr::load_tex(server, "textures/pbr/fir_twig_diff.png".into(), true);
        let nor = pbr::load_tex(server, "textures/pbr/fir_twig_nor.jpg".into(), false);
        let needles = [0, 1, 2].map(|_| {
            materials.add(StandardMaterial {
                base_color_texture: Some(twig.clone()),
                normal_map_texture: Some(nor.clone()),
                alpha_mode: AlphaMode::Mask(0.3),
                double_sided: true,
                cull_mode: None,
                perceptual_roughness: 0.85,
                ..default()
            })
        });
        let snow = materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.97, 1.0),
            base_color_texture: Some(twig),
            emissive: LinearRgba::rgb(0.5, 0.52, 0.56),
            alpha_mode: AlphaMode::Mask(0.45),
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        Self {
            trunk: meshes.add(trunk()),
            foliage: [0.0, 1.7, 3.9].map(|s| meshes.add(foliage(s))),
            bark: materials.add(pbr::material(server, "fir_bark", Vec2::ONE)),
            needles,
            snow,
        }
    }
}
