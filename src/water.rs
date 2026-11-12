//! Forma dell'isola e acque (ADR 0026): alture, scogliere, ruscello, laghetto, ponte, onde.
//!
//! I dati stanno in `assets/world/isola.txt`. Il ruscello scava il terreno: la superficie
//! scende sempre verso il mare, così l'acqua non risale mai.

use crate::terrain::SEA_LEVEL;
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::math::Affine2;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[cfg(test)]
const SPEC: &str = include_str!("../assets/world/isola.txt");
/// Passo (m) con cui si percorre il ruscello.
const STREAM_STEP: f32 = 0.5;
/// Larghezza della sponda che raccorda l'alveo al prato (m).
const BANK: f32 = 2.5;
/// Profondità al centro di ruscello e laghetto (m).
const STREAM_DEPTH: f32 = 0.45;
const LAKE_DEPTH: f32 = 0.9;
/// Sotto questa quota il fondo sarebbe scambiato per riva di mare.
const MIN_BED: f32 = -0.25;

/// Altura: gaussiana attorno al centro.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hill {
    pub center: Vec2,
    pub radius: f32,
    pub height: f32,
}

/// Tratto di costa a scogliera (gradi: 0 = Est, 90 = Sud).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cliff {
    pub from: f32,
    pub to: f32,
    pub height: f32,
}

/// Dati di `isola.txt`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IslandSpec {
    pub hills: Vec<Hill>,
    pub cliffs: Vec<Cliff>,
    pub stream: Vec<Vec2>,
    pub stream_half: f32,
    pub lake: Option<(Vec2, f32)>,
    pub bridges: Vec<(Vec2, Vec2)>,
    pub forests: Vec<Forest>,
    pub herds: Vec<Herd>,
    pub coops: Vec<Coop>,
    /// Nomi da scrivere sulla mappa.
    pub places: Vec<(String, Vec2)>,
    /// Centro dell'orbita dei gabbiani e quanti sono.
    pub gulls: Option<(Vec2, usize)>,
    /// Terraferma invece di isola (`costa:`): z della riva a sud, oltre c'è il mare.
    pub coast: Option<f32>,
    /// Confine percorribile della terraferma: |x| massimo e z minimo, a nord (`confine:`).
    pub bounds: Option<(f32, f32)>,
    /// Grande lago a livello del mare: centro e semiassi (`lago-grande:`).
    pub basin: Option<(Vec2, Vec2)>,
    /// Isolotto nel lago: centro e raggio (`isolotto:`).
    pub islet: Option<(Vec2, f32)>,
    /// Canale a livello del mare (l'emissario del lago): polilinea e semi-larghezza (`canale:`).
    pub channel: Vec<Vec2>,
    pub channel_half: f32,
}

/// Mandria o gregge: `kind` è l'indice in `animals::GRAZERS`.
#[derive(Clone, Debug, PartialEq)]
pub struct Herd {
    pub kind: usize,
    pub center: Vec2,
    pub radius: f32,
    pub count: usize,
}

/// Galline attorno a un punto.
#[derive(Clone, Debug, PartialEq)]
pub struct Coop {
    pub center: Vec2,
    pub radius: f32,
    pub count: usize,
}

/// Tipo di bosco.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForestKind {
    Firs,
    Mixed,
    Broadleaf,
    /// Continente: grandi querce, betulle bianche, o le due insieme.
    Oaks,
    Birches,
    OakBirch,
}

/// Zona di bosco.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Forest {
    pub center: Vec2,
    pub radius: f32,
    pub density: f32,
    pub kind: ForestKind,
}

/// Interpreta `isola.txt`; le righe errate vengono ignorate.
pub fn parse_spec(text: &str) -> IslandSpec {
    let mut s = IslandSpec {
        stream_half: 0.9,
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
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let n = nums(value);
        if key.trim() == "bosco" {
            let kind = match value.split_whitespace().last() {
                Some("abeti") => Some(ForestKind::Firs),
                Some("misto") => Some(ForestKind::Mixed),
                Some("latifoglie") => Some(ForestKind::Broadleaf),
                Some("querce") => Some(ForestKind::Oaks),
                Some("betulle") => Some(ForestKind::Birches),
                Some("querce-betulle") => Some(ForestKind::OakBirch),
                _ => None,
            };
            if let (Some(kind), [x, z, radius, density]) = (kind, n.as_slice()) {
                s.forests.push(Forest {
                    center: Vec2::new(*x, *z),
                    radius: *radius,
                    density: *density,
                    kind,
                });
            }
            continue;
        }
        if key.trim() == "luogo" {
            if let Some((name, at)) = value.split_once('|')
                && let [x, z] = nums(at).as_slice()
            {
                s.places.push((name.trim().to_string(), Vec2::new(*x, *z)));
            }
            continue;
        }
        if key.trim() == "gregge" {
            let kind = match value.split_whitespace().next() {
                Some("mucca") => Some(0),
                Some("pecora") => Some(1),
                _ => None,
            };
            if let (Some(kind), [x, z, radius, count]) = (kind, n.as_slice()) {
                s.herds.push(Herd {
                    kind,
                    center: Vec2::new(*x, *z),
                    radius: *radius,
                    count: *count as usize,
                });
            }
            continue;
        }
        match (key.trim(), n.as_slice()) {
            ("altura", [x, z, radius, height]) => s.hills.push(Hill {
                center: Vec2::new(*x, *z),
                radius: *radius,
                height: *height,
            }),
            ("scogliera", [from, to, height]) => s.cliffs.push(Cliff {
                from: *from,
                to: *to,
                height: *height,
            }),
            ("pollaio", [x, z, r, count]) => s.coops.push(Coop {
                center: Vec2::new(*x, *z),
                radius: *r,
                count: *count as usize,
            }),
            ("gabbiani", [x, z, count]) => s.gulls = Some((Vec2::new(*x, *z), *count as usize)),
            ("lago", [x, z, r]) => s.lake = Some((Vec2::new(*x, *z), *r)),
            ("costa", [z]) => s.coast = Some(*z),
            ("confine", [x, z]) => s.bounds = Some((*x, *z)),
            ("lago-grande", [x, z, rx, rz]) => {
                s.basin = Some((Vec2::new(*x, *z), Vec2::new(*rx, *rz)));
            }
            ("isolotto", [x, z, r]) => s.islet = Some((Vec2::new(*x, *z), *r)),
            ("canale", _) => {
                if let Some((head, pts)) = value.split_once('|') {
                    s.channel_half = nums(head).first().copied().unwrap_or(4.0);
                    s.channel = pts
                        .split(',')
                        .filter_map(|p| {
                            let n = nums(p);
                            (n.len() >= 2).then(|| Vec2::new(n[0], n[1]))
                        })
                        .collect();
                }
            }
            ("ponte", [x1, z1, x2, z2]) => {
                s.bridges.push((Vec2::new(*x1, *z1), Vec2::new(*x2, *z2)));
            }
            ("ruscello", _) => {
                if let Some((head, pts)) = value.split_once('|') {
                    if let Some(half) = nums(head).first() {
                        s.stream_half = *half;
                    }
                    s.stream = pts
                        .split(',')
                        .filter_map(|p| {
                            let n = nums(p);
                            (n.len() >= 2).then(|| Vec2::new(n[0], n[1]))
                        })
                        .collect();
                }
            }
            _ => {}
        }
    }
    s
}

/// Legge l'`isola.txt` della regione corrente (disco, poi copia incorporata).
pub fn load_spec() -> IslandSpec {
    parse_spec(&crate::region::world_text("isola.txt"))
}

/// Dati della regione corrente, letti una volta per regione (per chi li chiede spesso).
pub fn load_spec_cached() -> std::sync::Arc<IslandSpec> {
    use crate::region::Region;
    use std::sync::{Arc, Mutex};
    static CACHE: Mutex<Vec<(Region, Arc<IslandSpec>)>> = Mutex::new(Vec::new());
    let region = crate::region::current();
    let mut cache = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((_, s)) = cache.iter().find(|(r, _)| *r == region) {
        return s.clone();
    }
    let spec = Arc::new(load_spec());
    cache.push((region, spec.clone()));
    spec
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl IslandSpec {
    /// Quota aggiunta dalle alture in (x, z).
    pub fn hill_height(&self, p: Vec2) -> f32 {
        self.hills
            .iter()
            .map(|h| {
                let d = p.distance(h.center) / h.radius;
                h.height * (-d * d).exp()
            })
            .sum()
    }

    /// Quanto è scogliera la costa a questo angolo (0..1) e con che altezza.
    pub fn cliff_at(&self, angle: f32) -> (f32, f32) {
        let deg = angle.to_degrees();
        let mut best = (0.0f32, 0.0f32);
        for c in &self.cliffs {
            let k = smooth(c.from, c.from + 6.0, deg) * (1.0 - smooth(c.to - 6.0, c.to, deg));
            if k > best.0 {
                best = (k, c.height);
            }
        }
        best
    }
}

/// Punto del ruscello già percorso a passi regolari, con la quota dell'acqua.
#[derive(Clone, Copy, Debug)]
pub struct StreamPoint {
    pub pos: Vec2,
    pub surface: f32,
}

/// Il ruscello pronto per scavare e per il mesh.
#[derive(Clone, Debug, Default)]
pub struct WaterPlan {
    pub stream: Vec<StreamPoint>,
    pub half: f32,
    pub lake: Option<(Vec2, f32, f32)>,
}

/// Punti a passo regolare lungo la linea spezzata.
pub fn resample(line: &[Vec2], step: f32) -> Vec<Vec2> {
    let mut out = Vec::new();
    for seg in line.windows(2) {
        let (a, b) = (seg[0], seg[1]);
        let n = (a.distance(b) / step).ceil().max(1.0) as usize;
        for k in 0..n {
            out.push(a + (b - a) * (k as f32 / n as f32));
        }
    }
    if let Some(last) = line.last() {
        out.push(*last);
    }
    out
}

/// Calcola la superficie: non sale mai, e nel laghetto è piatta.
pub fn plan_water(spec: &IslandSpec, ground: impl Fn(Vec2) -> f32) -> WaterPlan {
    let points = resample(&spec.stream, STREAM_STEP);
    // Il laghetto sta sotto il punto più basso del suo bordo: l'acqua non resta sospesa.
    let lake_level = spec.lake.map(|(c, r)| {
        (0..24)
            .map(|k| {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                ground(c + Vec2::new(a.cos(), a.sin()) * r)
            })
            .fold(ground(c), f32::min)
            - 0.25
    });
    let lake_level = lake_level.map(|l| l.max(SEA_LEVEL));
    let mut stream = Vec::with_capacity(points.len());
    let mut level = f32::MAX;
    let mut passed_lake = false;
    for pos in points {
        let in_lake = spec.lake.is_some_and(|(c, r)| pos.distance(c) <= r);
        if in_lake && let Some(l) = lake_level {
            level = l;
            passed_lake = true;
            stream.push(StreamPoint { pos, surface: l });
            continue;
        }
        // Prima del laghetto l'acqua resta sopra il suo livello, poi riprende a scendere.
        let floor = lake_level.filter(|_| !passed_lake).unwrap_or(SEA_LEVEL);
        level = level.min(ground(pos) - 0.3).max(floor);
        stream.push(StreamPoint {
            pos,
            surface: level,
        });
    }
    WaterPlan {
        stream,
        half: spec.stream_half,
        lake: spec.lake.zip(lake_level).map(|((c, r), l)| (c, r, l)),
    }
}

/// Scava alveo e laghetto nel campo di altezze `h` (griglia `n`×`n`, passo `step`).
pub fn carve(plan: &WaterPlan, h: &mut [f32], n: usize, step: f32) -> Vec<bool> {
    let original = h.to_vec();
    let mut wet = vec![false; n * n];
    let index = |w: f32| ((w + crate::terrain::size() / 2.0) / step).round() as isize;
    let world = |i: usize| -crate::terrain::size() / 2.0 + i as f32 * step;
    let clamp = |i: isize| i.clamp(0, n as isize - 1) as usize;
    let mut dig = |center: Vec2, half: f32, surface: f32, depth: f32| {
        let reach = half + BANK;
        for iz in clamp(index(center.y - reach))..=clamp(index(center.y + reach)) {
            for ix in clamp(index(center.x - reach))..=clamp(index(center.x + reach)) {
                let d = Vec2::new(world(ix), world(iz)).distance(center);
                if d > reach {
                    continue;
                }
                let i = iz * n + ix;
                let t = (d / half).min(1.0);
                let target = (surface - depth * (1.0 - t * t)).max(MIN_BED.min(original[i]));
                let blend = 1.0 - smooth(half, reach, d);
                let value = original[i] + (target - original[i]) * blend;
                if value < h[i] {
                    h[i] = value;
                }
                if d < half {
                    wet[i] = true;
                }
            }
        }
    };
    for p in &plan.stream {
        dig(p.pos, plan.half, p.surface, STREAM_DEPTH);
    }
    if let Some((c, r, level)) = plan.lake {
        dig(c, r, level, LAKE_DEPTH);
    }
    wet
}

fn mesh_from(pos: Vec<[f32; 3]>, uv: Vec<[f32; 2]>, idx: Vec<u32>) -> Mesh {
    let n = pos.len();
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; n]);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[1.0f32, 0.0, 0.0, 1.0]; n]);
    m.insert_indices(Indices::U32(idx));
    m
}

/// Nastro d'acqua del ruscello e disco del laghetto.
pub fn water_mesh(plan: &WaterPlan) -> Mesh {
    let (mut pos, mut uv, mut idx) = (Vec::new(), Vec::new(), Vec::new());
    let width = plan.half + 0.2;
    let mut along = 0.0f32;
    for (i, p) in plan.stream.iter().enumerate() {
        let prev = plan.stream[i.saturating_sub(1)].pos;
        let next = plan.stream[(i + 1).min(plan.stream.len() - 1)].pos;
        let dir = (next - prev).normalize_or_zero();
        let side = Vec2::new(-dir.y, dir.x) * width;
        for (q, v) in [(p.pos - side, 0.0), (p.pos + side, 1.0)] {
            pos.push([q.x, p.surface, q.y]);
            uv.push([v, along / 3.0]);
        }
        // Dentro il laghetto basta la sua superficie: due strati si vedrebbero come una fascia.
        let in_lake = |q: Vec2| plan.lake.is_some_and(|(c, r, _)| q.distance(c) < r - 0.2);
        if i > 0 && !(in_lake(p.pos) && in_lake(prev)) {
            let b = (pos.len() - 4) as u32;
            idx.extend([b, b + 1, b + 2, b + 1, b + 3, b + 2]);
        }
        along += STREAM_STEP;
    }
    if let Some((c, r, level)) = plan.lake {
        let center = pos.len() as u32;
        pos.push([c.x, level, c.y]);
        uv.push([0.0, 0.0]);
        const RIM: usize = 40;
        for k in 0..=RIM {
            let a = k as f32 / RIM as f32 * std::f32::consts::TAU;
            let q = c + Vec2::new(a.cos(), a.sin()) * (r + 0.3);
            pos.push([q.x, level, q.y]);
            uv.push([(q.x - c.x) / 3.0, (q.y - c.y) / 3.0]);
        }
        for k in 0..RIM as u32 {
            idx.extend([center, center + 2 + k, center + 1 + k]);
        }
    }
    mesh_from(pos, uv, idx)
}

/// Mesh di una passerella tra due sponde: tavolato leggermente arcuato, con corrimano.
/// Restituisce i tronconi (centro, rotazione, lunghezza) per mesh e collider.
pub fn bridge_segments(a: Vec3, b: Vec3, parts: usize) -> Vec<(Vec3, Quat, f32)> {
    const ARCH: f32 = 0.45;
    let point = |t: f32| {
        let flat = a.lerp(b, t);
        flat + Vec3::Y * (ARCH * (t * std::f32::consts::PI).sin())
    };
    (0..parts)
        .map(|i| {
            let (p, q) = (
                point(i as f32 / parts as f32),
                point((i + 1) as f32 / parts as f32),
            );
            let dir = (q - p).normalize_or_zero();
            (
                (p + q) / 2.0,
                Quat::from_rotation_arc(Vec3::Z, dir),
                p.distance(q),
            )
        })
        .collect()
}

/// Mappa delle normali per le onde: somma di seni ripetibile sul bordo.
pub fn wave_normals(size: u32) -> Image {
    const WAVES: [(f32, f32, f32, f32); 6] = [
        (1.0, 2.0, 0.040, 0.3),
        (3.0, -1.0, 0.030, 1.7),
        (-2.0, 4.0, 0.024, 4.1),
        (5.0, 3.0, 0.016, 2.2),
        (-6.0, -2.0, 0.012, 0.9),
        (2.0, -7.0, 0.010, 5.3),
    ];
    let mut base = Vec::with_capacity((size * size) as usize);
    let tau = std::f32::consts::TAU;
    for y in 0..size {
        for x in 0..size {
            let (u, v) = (x as f32 / size as f32, y as f32 / size as f32);
            let (mut dx, mut dy) = (0.0f32, 0.0f32);
            for (kx, ky, amp, phase) in WAVES {
                let c = (tau * (kx * u + ky * v) + phase).cos() * amp;
                dx += c * kx * tau;
                dy += c * ky * tau;
            }
            base.push(Vec3::new(-dx * 0.12, -dy * 0.12, 1.0).normalize());
        }
    }
    // Catena di mipmap: da lontano le increspature si appiattiscono invece di sfarfallare.
    let mut data = Vec::new();
    let mut level = base;
    let mut side = size;
    let mut mips = 0;
    loop {
        for n in &level {
            data.extend([
                ((n.x * 0.5 + 0.5) * 255.0) as u8,
                ((n.y * 0.5 + 0.5) * 255.0) as u8,
                ((n.z * 0.5 + 0.5) * 255.0) as u8,
                255,
            ]);
        }
        mips += 1;
        if side == 1 {
            break;
        }
        let next = side / 2;
        level = (0..next * next)
            .map(|i| {
                let (x, y) = ((i % next) * 2, (i / next) * 2);
                let at = |x: u32, y: u32| level[(y * side + x) as usize];
                (at(x, y) + at(x + 1, y) + at(x, y + 1) + at(x + 1, y + 1)).normalize_or(Vec3::Z)
            })
            .collect();
        side = next;
    }
    let mut image = Image {
        data: Some(data),
        asset_usage: RenderAssetUsages::default(),
        ..Default::default()
    };
    image.texture_descriptor.size = Extent3d {
        width: size,
        height: size,
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.dimension = TextureDimension::D2;
    image.texture_descriptor.format = TextureFormat::Rgba8Unorm;
    image.texture_descriptor.mip_level_count = mips;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..Default::default()
    });
    image
}

/// Materiale del mare: lo si fa scorrere per le onde.
#[derive(Component)]
pub struct SeaSurface;
/// Materiale di ruscello e laghetto.
#[derive(Component)]
pub struct StreamSurface;
/// Schiuma lungo la riva.
#[derive(Component)]
pub struct ShoreFoam;

/// Sistema: onde, corrente e schiuma; con le onde spente l'acqua resta ferma.
#[allow(clippy::type_complexity)]
pub fn animate_water(
    time: Res<Time>,
    settings: Res<crate::audio::AudioSettings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sea: Query<&MeshMaterial3d<StandardMaterial>, With<SeaSurface>>,
    stream: Query<&MeshMaterial3d<StandardMaterial>, With<StreamSurface>>,
    mut foam: Query<(&MeshMaterial3d<StandardMaterial>, &mut Visibility), With<ShoreFoam>>,
) {
    let on = settings.gfx.waves;
    let t = time.elapsed_secs();
    for m in &sea {
        if let Some(mut mat) = materials.get_mut(&m.0) {
            // Due moti sovrapposti: la corrente che scorre e un lento respiro della scala delle onde.
            mat.uv_transform = if on {
                let swell = 1.0 + 0.06 * (t * 0.35).sin();
                Affine2::from_scale_angle_translation(
                    Vec2::splat(swell),
                    0.04 * (t * 0.1).sin(),
                    Vec2::new(t * 0.035, t * 0.022),
                )
            } else {
                Affine2::IDENTITY
            };
        }
    }
    for m in &stream {
        if let Some(mut mat) = materials.get_mut(&m.0) {
            let offset = if on {
                Vec2::new(0.0, -t * 0.25)
            } else {
                Vec2::ZERO
            };
            mat.uv_transform = Affine2::from_translation(offset);
        }
    }
    for (m, mut vis) in &mut foam {
        let wanted = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != wanted {
            *vis = wanted;
        }
        if on && let Some(mut mat) = materials.get_mut(&m.0) {
            let alpha = 0.45 + 0.3 * (t * 0.7).sin();
            mat.base_color = Color::srgba(0.95, 0.97, 1.0, alpha);
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, animate_water);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_island_file_is_complete() {
        let s = parse_spec(SPEC);
        assert!(s.hills.len() >= 3);
        assert_eq!(s.cliffs.len(), 2);
        assert!(s.stream.len() >= 4);
        assert!(s.lake.is_some());
        assert_eq!(s.bridges.len(), 1);
        assert!(s.forests.len() >= 8);
    }

    #[test]
    fn broken_lines_are_ignored() {
        let s = parse_spec("altura: 1 2\nscogliera: x y z\nniente\n");
        assert!(s.hills.is_empty() && s.cliffs.is_empty());
    }

    #[test]
    fn water_never_flows_uphill_and_the_lake_is_flat() {
        let spec = parse_spec(SPEC);
        // Terreno con una gobba: la superficie deve restare scendente.
        let plan = plan_water(&spec, |p| 4.0 + (p.x * 0.3).sin() * 1.5 - p.x * 0.03);
        for w in plan.stream.windows(2) {
            assert!(
                w[1].surface <= w[0].surface + 1e-3,
                "{} -> {}",
                w[0].surface,
                w[1].surface
            );
        }
        let (c, r, level) = plan.lake.expect("laghetto");
        for p in plan.stream.iter().filter(|p| p.pos.distance(c) <= r) {
            assert!((p.surface - level).abs() < 1e-3);
        }
    }

    #[test]
    fn the_water_never_drops_below_the_sea() {
        let spec = parse_spec(SPEC);
        let plan = plan_water(&spec, |_| -20.0);
        assert!(plan.stream.iter().all(|p| p.surface >= SEA_LEVEL));
    }

    #[test]
    fn hills_fade_out_with_distance() {
        let spec = parse_spec(SPEC);
        let top = spec.hills[0];
        assert!(spec.hill_height(top.center) >= top.height);
        assert!(spec.hill_height(Vec2::new(0.0, 10.0)) < 1.0);
    }

    #[test]
    fn cliffs_only_on_their_stretch_of_coast() {
        let spec = parse_spec(SPEC);
        assert!(spec.cliff_at(30f32.to_radians()).0 > 0.99);
        assert_eq!(spec.cliff_at(90f32.to_radians()).0, 0.0);
    }

    #[test]
    fn bridge_arches_between_the_banks() {
        let a = Vec3::new(0.0, 1.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 9.0);
        let s = bridge_segments(a, b, 6);
        assert_eq!(s.len(), 6);
        assert!(s[2].0.y > 1.3, "{}", s[2].0.y);
        assert!(s[0].0.y < s[2].0.y);
    }
}
