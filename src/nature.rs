//! Vegetazione dell'isola (ADR 0027): boschi, macchie, cespugli, rocce, fiori ed erba alta.
//!
//! Gli alberi non sono entità singole: per ogni riquadro di 30 m si fondono in poche mesh. Vicino al
//! giocatore si vede la versione piena, lontano quella semplificata senza ombre; erba e fiori solo vicino.

use crate::debug_place::DebugPlace;
use crate::scene::SceneNode;
use crate::seasons::{SeasonGround, SeasonLeaf, SeasonLook, SeasonSnowRoof};
use crate::terrain::{TERRAIN_SIZE, TerrainField, noise2};
use crate::trees::{FirAssets, broadleaf_foliage, broadleaf_trunk, bush, fir_foliage_far};
use crate::village::{VillageInfo, dist_to_segment, in_polygon};
use crate::water::{Forest, ForestKind, IslandSpec};
use crate::world_props::hash01;
use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{
    Indices, MeshVertexAttribute, Meshable, PrimitiveTopology, VertexAttributeValues,
};
use bevy::prelude::*;
use std::collections::BTreeMap;

const CHUNK: f32 = 30.0;
/// Passo della griglia degli alberi (m): nel fitto un albero ogni 3 m circa.
const TREE_STEP: f32 = 2.9;
/// Fin dove si vedono alberi pieni e dettagli (erba, fiori, cespugli), in metri.
const NEAR_TREES: f32 = 55.0;
const NEAR_DETAIL: f32 = 42.0;
/// Densità fuori dai boschi: qualche albero isolato.
const SCATTER: f32 = 0.05;

/// Copia modificabile di una mesh, per fonderne molte.
#[derive(Clone)]
struct Proto {
    pos: Vec<Vec3>,
    nor: Vec<Vec3>,
    uv: Vec<[f32; 2]>,
    tan: Option<Vec<[f32; 4]>>,
    col: Option<Vec<[f32; 4]>>,
    idx: Vec<u32>,
}

fn attr3(mesh: &Mesh, a: MeshVertexAttribute) -> Vec<Vec3> {
    match mesh.attribute(a) {
        Some(VertexAttributeValues::Float32x3(v)) => {
            v.iter().map(|p| Vec3::from_array(*p)).collect()
        }
        _ => Vec::new(),
    }
}

/// Modelli di `scripts/build_animals_trees.py`: nome e tipo di corteccia e foglie.
const LEAFY_MODELS: [(&str, &str, &str); 3] = [
    ("birch_a", "birch_bark.png", "birch_leaves.png"),
    ("leafy_a", "tree_bark.jpg", "tree_leaves.png"),
    ("birch_b", "birch_bark.png", "birch_leaves.png"),
];

/// Un albero importato: corteccia e foglie, da vicino e da lontano.
struct LeafyTree {
    bark: Proto,
    leaves: Proto,
    bark_far: Proto,
    leaves_far: Proto,
}

impl Proto {
    /// Legge una parte di un file `.pmesh`; `None` se il file è tronco.
    fn read(data: &[u8], at: &mut usize) -> Option<Self> {
        let number = |at: &mut usize| -> Option<u32> {
            let raw = data.get(*at..*at + 4)?;
            *at += 4;
            Some(u32::from_le_bytes(raw.try_into().ok()?))
        };
        let verts = number(at)? as usize;
        let count = number(at)? as usize;
        let mut p = Self {
            pos: Vec::with_capacity(verts),
            nor: Vec::with_capacity(verts),
            uv: Vec::with_capacity(verts),
            tan: None,
            col: None,
            idx: Vec::with_capacity(count),
        };
        for _ in 0..verts {
            let mut v = [0.0f32; 8];
            for x in &mut v {
                *x = f32::from_bits(number(at)?);
            }
            p.pos.push(Vec3::new(v[0], v[1], v[2]));
            p.nor.push(Vec3::new(v[3], v[4], v[5]));
            p.uv.push([v[6], v[7]]);
        }
        for _ in 0..count {
            let i = number(at)?;
            if i as usize >= verts {
                return None;
            }
            p.idx.push(i);
        }
        Some(p)
    }

    fn of(mesh: &Mesh) -> Self {
        let uv = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            Some(VertexAttributeValues::Float32x2(v)) => v.clone(),
            _ => Vec::new(),
        };
        let tan = match mesh.attribute(Mesh::ATTRIBUTE_TANGENT) {
            Some(VertexAttributeValues::Float32x4(v)) => Some(v.clone()),
            _ => None,
        };
        let col = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(VertexAttributeValues::Float32x4(v)) => Some(v.clone()),
            _ => None,
        };
        let idx = match mesh.indices() {
            Some(Indices::U32(i)) => i.clone(),
            Some(Indices::U16(i)) => i.iter().map(|v| u32::from(*v)).collect(),
            None => Vec::new(),
        };
        Self {
            pos: attr3(mesh, Mesh::ATTRIBUTE_POSITION),
            nor: attr3(mesh, Mesh::ATTRIBUTE_NORMAL),
            uv,
            tan,
            col,
            idx,
        }
    }
}

/// Carica un albero importato; se manca o è rovinato si usa quello generato nel codice.
fn load_leafy(name: &str) -> Option<LeafyTree> {
    let rel = format!("assets/models/trees/{name}.pmesh");
    let data = std::fs::read(&rel)
        .or_else(|_| std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(&rel)))
        .ok()?;
    if data.get(..4)? != b"PMS1" {
        return None;
    }
    let mut at = 4;
    Some(LeafyTree {
        bark: Proto::read(&data, &mut at)?,
        leaves: Proto::read(&data, &mut at)?,
        bark_far: Proto::read(&data, &mut at)?,
        leaves_far: Proto::read(&data, &mut at)?,
    })
}

/// Dove mettere una copia di un prototipo.
#[derive(Clone, Copy)]
struct Place {
    pos: Vec3,
    yaw: f32,
    scale: f32,
}

/// Fonde le copie in una mesh sola, in coordinate del mondo.
fn merge(proto: &Proto, places: &[Place]) -> Mesh {
    let n = proto.pos.len() * places.len();
    let (mut pos, mut nor, mut uv) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    let mut tan = proto.tan.as_ref().map(|_| Vec::with_capacity(n));
    let mut col = proto.col.as_ref().map(|_| Vec::with_capacity(n));
    let mut idx = Vec::with_capacity(proto.idx.len() * places.len());
    for (k, place) in places.iter().enumerate() {
        let rot = Quat::from_rotation_y(place.yaw);
        for p in &proto.pos {
            pos.push((place.pos + rot * (*p * place.scale)).to_array());
        }
        for v in &proto.nor {
            nor.push((rot * *v).to_array());
        }
        uv.extend_from_slice(&proto.uv);
        if let (Some(out), Some(src)) = (&mut tan, &proto.tan) {
            for t in src {
                let v = rot * Vec3::new(t[0], t[1], t[2]);
                out.push([v.x, v.y, v.z, t[3]]);
            }
        }
        if let (Some(out), Some(src)) = (&mut col, &proto.col) {
            out.extend_from_slice(src);
        }
        let base = (k * proto.pos.len()) as u32;
        idx.extend(proto.idx.iter().map(|i| i + base));
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nor);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    if let Some(t) = tan {
        m.insert_attribute(Mesh::ATTRIBUTE_TANGENT, t);
    }
    if let Some(c) = col {
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
    }
    m.insert_indices(Indices::U32(idx));
    m
}

/// Roccia: sfera sfaccettata, deformata e schiacciata.
fn rock(seed: f32) -> Mesh {
    let mut m: Mesh = Sphere::new(1.0).mesh().ico(2).expect("sfera");
    if let Some(VertexAttributeValues::Float32x3(pos)) = m.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for p in pos.iter_mut() {
            let v = Vec3::from_array(*p);
            let bump = 0.7 + 0.5 * noise2(v.x * 1.9 + seed, v.z * 1.9 + v.y * 1.3 - seed);
            p[0] *= bump;
            p[1] *= bump * 0.62;
            p[2] *= bump;
        }
    }
    m.compute_smooth_normals();
    m.with_generated_tangents().expect("roccia con UV")
}

/// Ciuffo d'erba alta: fili stretti con il colore sui vertici.
fn grass_tuft(seed: f32) -> Mesh {
    let (mut pos, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![]);
    for i in 0..7u32 {
        let a = (i as f32 * 2.399 + seed).rem_euclid(std::f32::consts::TAU);
        let spread = Vec3::new(a.cos(), 0.0, a.sin()) * (0.05 + 0.03 * i as f32);
        let side = Vec3::new(-a.sin(), 0.0, a.cos()) * 0.035;
        let lean = spread * 3.0;
        let h = 0.42 + 0.06 * (i % 4) as f32;
        let base = pos.len() as u32;
        for (p, c) in [
            (spread - side, [0.24f32, 0.38, 0.13, 1.0]),
            (spread + side, [0.24, 0.38, 0.13, 1.0]),
            (spread + lean + Vec3::Y * h, [0.55, 0.70, 0.27, 1.0]),
        ] {
            pos.push(p.to_array());
            uv.push([0.0f32, 0.0]);
            col.push(c);
        }
        idx.extend([base, base + 1, base + 2]);
    }
    let n = pos.len();
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; n]);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    m.insert_indices(Indices::U32(idx));
    m
}

/// Mazzetto di fiori di un colore: stelo e corolla a croce.
fn flower(color: [f32; 4]) -> Mesh {
    let (mut pos, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![]);
    for (x, z) in [(0.0f32, 0.0f32), (0.13, 0.08), (-0.1, 0.12)] {
        let c = Vec3::new(x, 0.0, z);
        let green = [0.2f32, 0.42, 0.14, 1.0];
        let mut tri = |a: Vec3, b: Vec3, d: Vec3, tint: [f32; 4]| {
            let base = pos.len() as u32;
            for p in [a, b, d] {
                pos.push(p.to_array());
                uv.push([0.0f32, 0.0]);
                col.push(tint);
            }
            idx.extend([base, base + 1, base + 2]);
        };
        let top = c + Vec3::Y * 0.3;
        tri(c - Vec3::X * 0.012, c + Vec3::X * 0.012, top, green);
        let r = 0.075;
        for (dx, dz) in [(1.0f32, 0.0f32), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)] {
            let tip = top + Vec3::new(dx * r, 0.02, dz * r);
            let side = Vec3::new(-dz, 0.0, dx) * r * 0.6;
            tri(top, tip + side, tip - side, color);
        }
    }
    let n = pos.len();
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; n]);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    m.insert_indices(Indices::U32(idx));
    m
}

/// Mappa a celle di 1 m dei punti dove non crescono piante.
pub(crate) struct Blocked {
    n: usize,
    cells: Vec<bool>,
}

impl Blocked {
    fn index(&self, p: Vec2) -> Option<usize> {
        let f = |v: f32| (v + TERRAIN_SIZE / 2.0).floor() as isize;
        let (x, z) = (f(p.x), f(p.y));
        let n = self.n as isize;
        ((0..n).contains(&x) && (0..n).contains(&z)).then(|| (z * n + x) as usize)
    }

    pub(crate) fn is(&self, p: Vec2) -> bool {
        self.index(p).is_none_or(|i| self.cells[i])
    }

    fn stamp(&mut self, c: Vec2, radius: f32) {
        let r = radius.ceil() as i32;
        for dz in -r..=r {
            for dx in -r..=r {
                let q = c + Vec2::new(dx as f32, dz as f32);
                if q.distance(c) <= radius
                    && let Some(i) = self.index(q)
                {
                    self.cells[i] = true;
                }
            }
        }
    }
}

/// Dove non si pianta: paese, strade, sentieri, edifici, campi, ponti.
pub(crate) fn blocked_map(dbg: &DebugPlace, village: &VillageInfo, island: &IslandSpec) -> Blocked {
    let n = TERRAIN_SIZE as usize + 1;
    let mut b = Blocked {
        n,
        cells: vec![false; n * n],
    };
    let poly = &village.poly;
    for z in 0..n {
        for x in 0..n {
            let p = Vec2::new(
                x as f32 - TERRAIN_SIZE / 2.0 + 0.5,
                z as f32 - TERRAIN_SIZE / 2.0 + 0.5,
            );
            let near_wall = poly.len() >= 3
                && (in_polygon(poly, p)
                    || (0..poly.len())
                        .any(|i| dist_to_segment(p, poly[i], poly[(i + 1) % poly.len()]) < 7.0));
            if near_wall {
                b.cells[z * n + x] = true;
            }
        }
    }
    for road in &village.spec.roads {
        for seg in road.pts.windows(2) {
            let steps = (seg[0].distance(seg[1]).ceil() as usize).max(1);
            for k in 0..=steps {
                b.stamp(
                    seg[0].lerp(seg[1], k as f32 / steps as f32),
                    road.width / 2.0 + 1.4,
                );
            }
        }
    }
    for (x, y) in &island.bridges {
        b.stamp(*x, 3.0);
        b.stamp(*y, 3.0);
    }
    for pl in &dbg.items {
        let radius = match pl.kind.as_str() {
            "campo" | "orto" => 1.6,
            "albero" => 2.2,
            "fiori" | "panchina" | "lampione" | "cartello" => 1.5,
            kind => crate::buildings::interior_half(kind).map_or(4.5, |h| h.length() + 4.5),
        };
        for p in &pl.points {
            b.stamp(*p, radius);
        }
    }
    for (c, r) in crate::world_props::tree_keep_out() {
        b.stamp(c, r + 1.0);
    }
    b
}

/// Quanto bosco c'è in un punto, e di che tipo.
fn forest_at(forests: &[Forest], p: Vec2) -> (f32, ForestKind) {
    let mut best = (SCATTER, ForestKind::Mixed);
    for f in forests {
        let t = (p.distance(f.center) / f.radius).clamp(0.0, 1.0);
        // Pieno fino a metà raggio, poi sfuma verso il margine.
        let t = ((t - 0.5) * 2.0).clamp(0.0, 1.0);
        let d = f.density * (1.0 - t * t * (3.0 - 2.0 * t));
        if d > best.0 {
            best = (d, f.kind);
        }
    }
    best
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Ciò che cresce in un riquadro.
#[derive(Default)]
struct Chunk {
    firs: [Vec<Place>; 3],
    broad: [Vec<Place>; 3],
    bushes: [Vec<Place>; 3],
    rocks: [Vec<Place>; 3],
    grass: Vec<Place>,
    flowers: [Vec<Place>; 4],
    colliders: Vec<(Vec3, f32, bool)>,
}

/// Decide cosa cresce dove.
#[allow(clippy::too_many_lines)]
fn plant(
    field: &TerrainField,
    blocked: &Blocked,
    forests: &[Forest],
    vegetation: f32,
) -> BTreeMap<(i32, i32), Chunk> {
    let mut chunks: BTreeMap<(i32, i32), Chunk> = BTreeMap::new();
    let half = TERRAIN_SIZE / 2.0;
    let key = |p: Vec2| ((p.x / CHUNK).floor() as i32, (p.y / CHUNK).floor() as i32);
    // Terra asciutta, non in pendenza forte.
    let ground = |p: Vec2| -> Option<Vec3> {
        let h = field.height_at(p.x, p.y);
        if h < 0.2 || blocked.is(p) || field.in_fresh_water(p.x, p.y) {
            return None;
        }
        let dx = field.height_at(p.x + 1.0, p.y) - field.height_at(p.x - 1.0, p.y);
        let dz = field.height_at(p.x, p.y + 1.0) - field.height_at(p.x, p.y - 1.0);
        ((dx * dx + dz * dz).sqrt() / 2.0 < 0.65).then_some(Vec3::new(p.x, h, p.y))
    };
    let grid = |step: f32| {
        let n = (TERRAIN_SIZE / step) as i32;
        (0..n).flat_map(move |iz| {
            (0..n).map(move |ix| Vec2::new(ix as f32 * step - half, iz as f32 * step - half))
        })
    };
    let jitter = |c: Vec2, step: f32, salt: f32| {
        c + Vec2::new(hash01(c, salt) - 0.5, hash01(c, salt + 1.0) - 0.5) * step * 0.85
    };
    // Radure: il rumore apre spazi anche dentro i boschi.
    let clearing = |p: Vec2| smooth(0.18, 0.4, noise2(p.x * 0.06 + 40.0, p.y * 0.06 + 13.0));
    for c in grid(TREE_STEP) {
        let p = jitter(c, TREE_STEP, 1.0);
        let (d, kind) = forest_at(forests, p);
        if hash01(c, 3.0) >= d * clearing(p) * vegetation {
            continue;
        }
        let Some(at) = ground(p) else {
            continue;
        };
        let chunk = chunks.entry(key(p)).or_default();
        let broad = match kind {
            ForestKind::Firs => false,
            ForestKind::Broadleaf => true,
            ForestKind::Mixed => hash01(c, 4.0) < 0.5,
        };
        let place = Place {
            pos: at - Vec3::Y * 0.1,
            yaw: hash01(c, 5.0) * std::f32::consts::TAU,
            scale: 0.8 + hash01(c, 6.0) * 0.55,
        };
        let v = (hash01(c, 7.0) * 3.0) as usize % 3;
        if broad {
            chunk.broad[v].push(place);
        } else {
            chunk.firs[v].push(place);
        }
        chunk
            .colliders
            .push((at + Vec3::Y * 1.0, 0.32 * place.scale, false));
    }
    for c in grid(3.0) {
        let p = jitter(c, 3.0, 11.0);
        let (d, _) = forest_at(forests, p);
        if hash01(c, 13.0) >= (d * 0.4 + 0.02) * vegetation {
            continue;
        }
        if let Some(at) = ground(p) {
            let v = (hash01(c, 14.0) * 3.0) as usize % 3;
            chunks.entry(key(p)).or_default().bushes[v].push(Place {
                pos: at,
                yaw: hash01(c, 15.0) * std::f32::consts::TAU,
                scale: 0.8 + hash01(c, 16.0) * 0.8,
            });
        }
    }
    for c in grid(8.0) {
        let p = jitter(c, 8.0, 21.0);
        let height = field.height_at(p.x, p.y);
        if hash01(c, 22.0) >= 0.07 + smooth(4.0, 11.0, height) * 0.25 {
            continue;
        }
        if let Some(at) = ground(p) {
            let scale = 0.35 + hash01(c, 23.0).powi(2) * 1.3;
            let v = (hash01(c, 24.0) * 3.0) as usize % 3;
            let chunk = chunks.entry(key(p)).or_default();
            chunk.rocks[v].push(Place {
                pos: at - Vec3::Y * 0.08 * scale,
                yaw: hash01(c, 25.0) * std::f32::consts::TAU,
                scale,
            });
            if scale > 0.65 {
                chunk.colliders.push((at, scale * 0.75, true));
            }
        }
    }
    // Prati: erba alta e fiori a macchie, fuori dal fitto.
    for c in grid(1.7) {
        let p = jitter(c, 1.7, 31.0);
        let meadow = noise2(p.x * 0.09 + 7.0, p.y * 0.09 + 29.0);
        let (d, _) = forest_at(forests, p);
        if meadow < 0.5 || d > 0.55 {
            continue;
        }
        let r = hash01(c, 32.0);
        if r > 0.45 * vegetation {
            continue;
        }
        let Some(at) = ground(p) else {
            continue;
        };
        let chunk = chunks.entry(key(p)).or_default();
        let place = Place {
            pos: at,
            yaw: hash01(c, 33.0) * std::f32::consts::TAU,
            scale: 0.85 + hash01(c, 34.0) * 0.6,
        };
        if meadow > 0.66 && r < 0.13 {
            chunk.flowers[(hash01(c, 35.0) * 4.0) as usize % 4].push(place);
        } else {
            chunk.grass.push(place);
        }
    }
    chunks
}

/// Quando appare una parte: stagione delle foglie o della fioritura.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum SeasonGate {
    Leaves,
    Blooms,
}

/// Riquadro di vegetazione.
#[derive(Component)]
pub struct NatureChunk {
    center: Vec2,
}

/// Gruppo mostrato secondo la distanza.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum NatureGroup {
    NearTrees,
    FarTrees,
    Detail,
}

/// Tutto ciò che serve alla vegetazione, costruito una volta.
struct Kit {
    fir_trunk: Handle<Mesh>,
    bark: Handle<StandardMaterial>,
    /// Corteccia di ogni variante di latifoglia.
    broad_bark: [Handle<StandardMaterial>; 3],
    needles: [Handle<StandardMaterial>; 3],
    leaves: [Handle<StandardMaterial>; 3],
    /// Da lontano la trasparenza si taglia più in basso, altrimenti i rami spariscono.
    needles_far: [Handle<StandardMaterial>; 3],
    leaves_far: [Handle<StandardMaterial>; 3],
    snow: Handle<StandardMaterial>,
    stone: Handle<StandardMaterial>,
    flowers: Handle<StandardMaterial>,
    grass: Handle<StandardMaterial>,
    protos: Protos,
}

struct Protos {
    fir_trunk: Proto,
    fir: [Proto; 3],
    fir_far: [Proto; 3],
    broad_bark: [Proto; 3],
    broad_bark_far: [Proto; 3],
    broad: [Proto; 3],
    broad_far: [Proto; 3],
    bush: [Proto; 3],
    rock: [Proto; 3],
    grass: Proto,
    flowers: [Proto; 4],
}

fn make_kit(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    server: &AssetServer,
    pbr: &crate::pbr::PbrMaterials,
) -> Kit {
    let firs = FirAssets::new(meshes, materials, server);
    let copy = |m: &Handle<Mesh>, meshes: &Assets<Mesh>| {
        Proto::of(meshes.get(m).expect("mesh dell'abete appena creata"))
    };
    let mut leaves = firs.needles.clone().map(|h| {
        let mut m = materials.get(&h).cloned().unwrap_or_default();
        m.perceptual_roughness = 0.7;
        materials.add(m)
    });
    let models = LEAFY_MODELS.map(|(name, ..)| load_leafy(name));
    let mut broad_bark = [firs.bark.clone(), firs.bark.clone(), firs.bark.clone()];
    for (v, (model, bark, leaf)) in LEAFY_MODELS.iter().enumerate().map(|(v, m)| (v, *m)) {
        if models[v].is_none() {
            warn!("[NATURE] modello {model} assente: uso la latifoglia generata");
            continue;
        }
        broad_bark[v] = materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.9, 0.82),
            base_color_texture: Some(server.load(format!("textures/alberi/{bark}"))),
            perceptual_roughness: 0.92,
            ..default()
        });
        leaves[v] = materials.add(StandardMaterial {
            base_color_texture: Some(server.load(format!("textures/alberi/{leaf}"))),
            perceptual_roughness: 0.7,
            alpha_mode: AlphaMode::Mask(0.3),
            cull_mode: None,
            double_sided: true,
            ..default()
        });
    }
    let mut far = |h: &Handle<StandardMaterial>| {
        let mut m = materials.get(h).cloned().unwrap_or_default();
        m.alpha_mode = AlphaMode::Mask(0.12);
        materials.add(m)
    };
    let needles_far = [0, 1, 2].map(|i| far(&firs.needles[i]));
    let leaves_far = [0, 1, 2].map(|i| far(&leaves[i]));
    let vertex_colors = |materials: &mut Assets<StandardMaterial>| {
        materials.add(StandardMaterial {
            perceptual_roughness: 0.9,
            double_sided: true,
            cull_mode: None,
            ..default()
        })
    };
    let part = |v: usize, pick: fn(&LeafyTree) -> &Proto, fallback: &dyn Fn() -> Proto| {
        models[v]
            .as_ref()
            .map_or_else(fallback, |t| pick(t).clone())
    };
    let seeds = [0.3, 2.1, 4.4];
    let trunk = || Proto::of(&broadleaf_trunk());
    let protos = Protos {
        fir_trunk: copy(&firs.trunk, meshes),
        fir: [0, 1, 2].map(|i| copy(&firs.foliage[i], meshes)),
        fir_far: [0.0, 1.7, 3.9].map(|s| Proto::of(&fir_foliage_far(s))),
        broad_bark: [0, 1, 2].map(|v| part(v, |t| &t.bark, &trunk)),
        broad_bark_far: [0, 1, 2].map(|v| part(v, |t| &t.bark_far, &trunk)),
        broad: [0, 1, 2].map(|v| {
            part(v, |t| &t.leaves, &|| {
                Proto::of(&broadleaf_foliage(seeds[v], 130))
            })
        }),
        broad_far: [0, 1, 2].map(|v| {
            part(v, |t| &t.leaves_far, &|| {
                Proto::of(&broadleaf_foliage(seeds[v], 34))
            })
        }),
        bush: [0.5, 1.9, 3.3].map(|s| Proto::of(&bush(s))),
        rock: [0.7, 2.9, 5.1].map(|s| Proto::of(&rock(s))),
        grass: Proto::of(&grass_tuft(0.4)),
        flowers: [
            [0.95, 0.45, 0.7, 1.0],
            [0.98, 0.85, 0.25, 1.0],
            [0.96, 0.96, 0.92, 1.0],
            [0.6, 0.4, 0.9, 1.0],
        ]
        .map(|c| Proto::of(&flower(c))),
    };
    Kit {
        fir_trunk: firs.trunk.clone(),
        bark: firs.bark.clone(),
        broad_bark,
        needles: firs.needles.clone(),
        leaves,
        needles_far,
        leaves_far,
        snow: firs.snow.clone(),
        stone: pbr.get(crate::pbr::Mat::Stone),
        flowers: vertex_colors(materials),
        grass: vertex_colors(materials),
        protos,
    }
}

/// Costruisce i riquadri.
fn spawn_chunks(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    kit: &Kit,
    chunks: BTreeMap<(i32, i32), Chunk>,
) {
    let p = &kit.protos;
    for ((cx, cz), c) in chunks {
        let center = Vec2::new((cx as f32 + 0.5) * CHUNK, (cz as f32 + 0.5) * CHUNK);
        commands
            .spawn((
                SceneNode,
                NatureChunk { center },
                RigidBody::Static,
                Transform::default(),
                Visibility::default(),
            ))
            .with_children(|root| {
                for (at, radius, rock) in &c.colliders {
                    if *rock {
                        root.spawn((Collider::sphere(*radius), Transform::from_translation(*at)));
                    } else {
                        root.spawn((
                            Collider::cylinder(*radius, 2.6),
                            Transform::from_translation(*at),
                        ));
                    }
                }
                // Tronchi e rocce si vedono sempre.
                let firs: Vec<Place> = c.firs.iter().flatten().copied().collect();
                if !firs.is_empty() {
                    root.spawn((
                        Mesh3d(meshes.add(merge(&p.fir_trunk, &firs))),
                        MeshMaterial3d(kit.bark.clone()),
                    ));
                }
                for v in 0..3 {
                    if !c.rocks[v].is_empty() {
                        root.spawn((
                            Mesh3d(meshes.add(merge(&p.rock[v], &c.rocks[v]))),
                            MeshMaterial3d(kit.stone.clone()),
                        ));
                    }
                }
                for (group, fir_set, broad_set, bark_set) in [
                    (NatureGroup::NearTrees, &p.fir, &p.broad, &p.broad_bark),
                    (
                        NatureGroup::FarTrees,
                        &p.fir_far,
                        &p.broad_far,
                        &p.broad_bark_far,
                    ),
                ] {
                    let far = group == NatureGroup::FarTrees;
                    root.spawn((group, Transform::default(), Visibility::Hidden))
                        .with_children(|g| {
                            for v in 0..3 {
                                if !c.firs[v].is_empty() {
                                    let mesh = meshes.add(merge(&fir_set[v], &c.firs[v]));
                                    let mut e = g.spawn((
                                        SeasonLeaf(v),
                                        Mesh3d(mesh.clone()),
                                        MeshMaterial3d(if far {
                                            kit.needles_far[v].clone()
                                        } else {
                                            kit.needles[v].clone()
                                        }),
                                    ));
                                    if far {
                                        e.insert(NotShadowCaster);
                                    }
                                    g.spawn((
                                        SeasonSnowRoof,
                                        Mesh3d(mesh),
                                        MeshMaterial3d(kit.snow.clone()),
                                        Transform::from_xyz(0.0, 0.04, 0.0),
                                        Visibility::Hidden,
                                        NotShadowCaster,
                                    ));
                                }
                                if !c.broad[v].is_empty() {
                                    let mut trunk = g.spawn((
                                        Mesh3d(meshes.add(merge(&bark_set[v], &c.broad[v]))),
                                        MeshMaterial3d(kit.broad_bark[v].clone()),
                                    ));
                                    if far {
                                        trunk.insert(NotShadowCaster);
                                    }
                                    let mut e = g.spawn((
                                        SeasonLeaf(v),
                                        SeasonGate::Leaves,
                                        Mesh3d(meshes.add(merge(&broad_set[v], &c.broad[v]))),
                                        MeshMaterial3d(if far {
                                            kit.leaves_far[v].clone()
                                        } else {
                                            kit.leaves[v].clone()
                                        }),
                                    ));
                                    if far {
                                        e.insert(NotShadowCaster);
                                    }
                                }
                            }
                        });
                }
                root.spawn((
                    NatureGroup::Detail,
                    Transform::default(),
                    Visibility::Hidden,
                ))
                .with_children(|g| {
                    for v in 0..3 {
                        if !c.bushes[v].is_empty() {
                            g.spawn((
                                SeasonLeaf(v),
                                Mesh3d(meshes.add(merge(&p.bush[v], &c.bushes[v]))),
                                MeshMaterial3d(kit.needles[v].clone()),
                            ));
                        }
                    }
                    if !c.grass.is_empty() {
                        g.spawn((
                            SeasonGround { sand: false },
                            Mesh3d(meshes.add(merge(&p.grass, &c.grass))),
                            MeshMaterial3d(kit.grass.clone()),
                            NotShadowCaster,
                        ));
                    }
                    for v in 0..4 {
                        if !c.flowers[v].is_empty() {
                            g.spawn((
                                SeasonGate::Blooms,
                                Mesh3d(meshes.add(merge(&p.flowers[v], &c.flowers[v]))),
                                MeshMaterial3d(kit.flowers.clone()),
                                NotShadowCaster,
                            ));
                        }
                    }
                });
            });
    }
    let _ = &kit.fir_trunk;
}

/// Sistema: pianta tutto quando il terreno è pronto, dopo un reset o se cambia la densità.
#[allow(clippy::too_many_arguments)]
pub fn sync_nature(
    mut commands: Commands,
    field: Option<Res<TerrainField>>,
    dbg: Res<DebugPlace>,
    village: Res<VillageInfo>,
    settings: Res<crate::audio::AudioSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
    pbr: Res<crate::pbr::PbrMaterials>,
    existing: Query<Entity, With<NatureChunk>>,
    mut built: Local<Option<(u32, u8)>>,
) {
    let Some(field) = field else {
        return;
    };
    if village.poly.len() < 3 {
        return;
    }
    let want = (dbg.revision(), settings.gfx.vegetation);
    let wiped = existing.is_empty() && built.is_some();
    if *built == Some(want) && !wiped {
        return;
    }
    *built = Some(want);
    for e in &existing {
        commands.entity(e).despawn();
    }
    let island = crate::water::load_spec();
    let blocked = blocked_map(&dbg, &village, &island);
    let chunks = plant(
        &field,
        &blocked,
        &island.forests,
        f32::from(settings.gfx.vegetation) / 100.0,
    );
    let trees: usize = chunks
        .values()
        .map(|c| c.firs.iter().chain(&c.broad).map(Vec::len).sum::<usize>())
        .sum();
    info!("[NATURE] {} riquadri, {trees} alberi", chunks.len());
    let kit = make_kit(&mut meshes, &mut materials, &server, &pbr);
    spawn_chunks(&mut commands, &mut meshes, &kit, chunks);
}

/// Sistema: mostra la versione giusta di ogni riquadro secondo la distanza.
#[allow(clippy::type_complexity)]
pub fn nature_lod(
    time: Res<Time>,
    mut timer: Local<f32>,
    player: Query<&GlobalTransform, With<crate::player::Player>>,
    chunks: Query<(&NatureChunk, &Children)>,
    mut groups: Query<(&NatureGroup, &mut Visibility)>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 0.4;
    let Ok(player) = player.single() else {
        return;
    };
    let p = player.translation();
    let here = Vec2::new(p.x, p.z);
    for (chunk, children) in &chunks {
        let d = chunk.center.distance(here);
        for child in children {
            if let Ok((group, mut vis)) = groups.get_mut(*child) {
                let show = match group {
                    NatureGroup::NearTrees => d < NEAR_TREES,
                    NatureGroup::FarTrees => d >= NEAR_TREES,
                    NatureGroup::Detail => d < NEAR_DETAIL,
                };
                let wanted = if show {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *vis != wanted {
                    *vis = wanted;
                }
            }
        }
    }
}

/// Sistema: le latifoglie perdono le foglie d'inverno, i fiori sbocciano nella loro stagione.
pub fn apply_nature_season(
    season: Res<SeasonLook>,
    mut gates: Query<(&SeasonGate, &mut Visibility)>,
    fresh: Query<(), Added<SeasonGate>>,
) {
    if !season.is_changed() && fresh.is_empty() {
        return;
    }
    for (gate, mut vis) in &mut gates {
        let show = match gate {
            SeasonGate::Leaves => season.0.snow < 0.2,
            SeasonGate::Blooms => season.0.flowers > 0.3,
        };
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (sync_nature, nature_lod, apply_nature_season));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forests_fade_towards_their_edge() {
        let forests = [Forest {
            center: Vec2::ZERO,
            radius: 20.0,
            density: 1.0,
            kind: ForestKind::Firs,
        }];
        assert_eq!(forest_at(&forests, Vec2::ZERO).0, 1.0);
        let mid = forest_at(&forests, Vec2::new(10.0, 0.0)).0;
        assert_eq!(mid, 1.0, "pieno fino a metà raggio");
        let edge = forest_at(&forests, Vec2::new(16.0, 0.0)).0;
        assert!(edge > SCATTER && edge < 1.0);
        assert_eq!(forest_at(&forests, Vec2::new(40.0, 0.0)).0, SCATTER);
    }

    #[test]
    fn merging_multiplies_vertices_and_moves_them() {
        let proto = Proto::of(&grass_tuft(0.1));
        let places = [
            Place {
                pos: Vec3::new(5.0, 1.0, 2.0),
                yaw: 0.0,
                scale: 1.0,
            },
            Place {
                pos: Vec3::new(-5.0, 2.0, 0.0),
                yaw: 1.0,
                scale: 2.0,
            },
        ];
        let m = merge(&proto, &places);
        assert_eq!(m.count_vertices(), proto.pos.len() * 2);
        let Some(VertexAttributeValues::Float32x3(pos)) = m.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("senza posizioni");
        };
        assert!((pos[0][0] - (5.0 + proto.pos[0].x)).abs() < 1e-4);
        let Some(Indices::U32(idx)) = m.indices() else {
            panic!("senza indici");
        };
        assert_eq!(*idx.iter().max().unwrap() as usize, m.count_vertices() - 1);
    }

    #[test]
    fn blocked_cells_are_marked_within_the_radius() {
        let n = TERRAIN_SIZE as usize + 1;
        let mut b = Blocked {
            n,
            cells: vec![false; n * n],
        };
        b.stamp(Vec2::new(10.5, 10.5), 3.0);
        assert!(b.is(Vec2::new(12.5, 10.5)));
        assert!(!b.is(Vec2::new(20.5, 10.5)));
        // Fuori dalla mappa non si pianta.
        assert!(b.is(Vec2::new(900.0, 0.0)));
    }
}
