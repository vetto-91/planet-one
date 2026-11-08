//! Abitanti e guardie di Fontevivo: personaggi con dialogo scritto breve + chat con la IA locale.
//!
//! I dati stanno in `assets/world/abitanti.txt` (posto, orari, aspetto); carattere, voce e
//! saluto in `assets/ai/<nome>.txt`. Routine semplice: di giorno al loro posto, la sera in piazza
//! (cammina sulle strade), di notte spariscono in casa; le guardie restano ai cancelli sempre,
//! con la lanterna accesa di notte.

use crate::daynight::GameClock;
use crate::entities::{DialogState, Interactable, Npc, NpcMode};
use crate::scene::SceneNode;
use crate::terrain::TerrainField;
use crate::village::{Road, VillageInfo};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::VecDeque;

const SPEC: &str = include_str!("../assets/world/abitanti.txt");
/// Velocità di camminata (m/s) e altezza del centro del corpo sul terreno.
const WALK_SPEED: f32 = 1.3;
const CENTER_Y: f32 = 0.85;

/// Dati di un abitante letti da `abitanti.txt`.
#[derive(Clone, Debug, Default)]
pub struct VillagerDef {
    pub name: String,
    pub role: String,
    pub skin: String,
    pub guard: bool,
    /// Assortimento del negozio (chiave di `shop.rs`), se è un venditore.
    pub shop: Option<String>,
    /// Posto di lavoro e direzione dello sguardo (dx, dz).
    pub work: (Vec2, Vec2),
    /// Posto serale (se diverso) e sguardo.
    pub evening: Option<(Vec2, Vec2)>,
    /// Inizio lavoro, fine lavoro (va in piazza), fine serata (sparisce in casa).
    pub hours: (f32, f32, f32),
}

fn parse(text: &str) -> Vec<VillagerDef> {
    let mut v: Vec<VillagerDef> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            v.push(VillagerDef {
                name: name.trim().to_string(),
                hours: (6.0, 18.0, 22.0),
                ..Default::default()
            });
            continue;
        }
        let (Some(d), Some((k, val))) = (v.last_mut(), line.split_once(':')) else {
            continue;
        };
        let nums: Vec<f32> = val
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        let pos =
            |n: &[f32]| (n.len() >= 4).then(|| (Vec2::new(n[0], n[1]), Vec2::new(n[2], n[3])));
        match k.trim() {
            "ruolo" => d.role = val.trim().to_string(),
            "skin" => d.skin = val.trim().to_string(),
            "guardia" => d.guard = val.trim() == "si",
            "negozio" => d.shop = Some(val.trim().to_string()),
            "lavoro" => {
                if let Some(p) = pos(&nums) {
                    d.work = p;
                }
            }
            "sera" => d.evening = pos(&nums),
            "orari" if nums.len() >= 3 => d.hours = (nums[0], nums[1], nums[2]),
            _ => {}
        }
    }
    v
}

/// Legge `assets/world/abitanti.txt` (disco, poi copia incorporata).
pub fn load_defs() -> Vec<VillagerDef> {
    let rel = "assets/world/abitanti.txt";
    let candidates = [
        std::path::PathBuf::from(rel),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel),
    ];
    match candidates
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
    {
        Some(t) => parse(&t),
        None => parse(SPEC),
    }
}

/// Fase della giornata di un abitante.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Work,
    Evening,
    Night,
}

/// Componente di un abitante (routine).
#[derive(Component)]
pub struct Villager {
    def: VillagerDef,
    phase: Phase,
    path: VecDeque<Vec2>,
    placed: bool,
}

/// Lanterna di una guardia (accesa di notte).
#[derive(Component)]
pub struct GuardLantern;

fn desired_phase(def: &VillagerDef, hour: f32) -> Phase {
    if def.guard {
        return Phase::Work;
    }
    let (start, end, night) = def.hours;
    if hour >= start && hour < end {
        Phase::Work
    } else if hour >= end && hour < night {
        Phase::Evening
    } else {
        Phase::Night
    }
}

fn spot(def: &VillagerDef, phase: Phase) -> (Vec2, Vec2) {
    match phase {
        Phase::Evening => def.evening.unwrap_or(def.work),
        _ => def.work,
    }
}

/// Direzione (yaw) perché il davanti (-Z locale) guardi verso `d`.
fn yaw_towards(d: Vec2) -> f32 {
    (-d.x).atan2(-d.y)
}

/// Crea gli abitanti (chiamata da `scene::spawn_world`, rifatta a ogni reset).
pub fn spawn_villagers(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    let figure = meshes.add(crate::figure::figure_mesh(
        Vec3::new(0.7, 1.6, 0.7),
        &crate::figure::FigureUv::standard(),
    ));
    let hands = crate::figure::HandAssets::new(meshes, materials);
    let steel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.56, 0.6),
        metallic: 0.5,
        perceptual_roughness: 0.5,
        ..default()
    });
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.27, 0.15),
        ..default()
    });
    for def in load_defs() {
        let persona = crate::ai::persona::load(&def.name);
        let lines: Vec<&'static str> = persona
            .greeting
            .iter()
            .map(|l| &*Box::leak(l.clone().into_boxed_str()))
            .collect();
        let lines: &'static [&'static str] = Box::leak(lines.into_boxed_slice());
        let name: &'static str = Box::leak(def.name.clone().into_boxed_str());
        let skin = materials.add(crate::figure::skin_material(
            asset_server.load(format!("textures/abitante-{}.png", def.skin)),
        ));
        let (pos, look) = def.work;
        let guard = def.guard;
        let shop_key: Option<&'static str> =
            def.shop.clone().map(|k| &*Box::leak(k.into_boxed_str()));
        let lantern_mesh = meshes.add(Cuboid::new(0.14, 0.2, 0.14));
        let lantern_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.45),
            emissive: LinearRgba::rgb(0.2, 0.12, 0.03),
            ..default()
        });
        let helmet = meshes.add(Cuboid::new(0.84, 0.22, 0.84));
        let spear = meshes.add(Cylinder::new(0.03, 2.5));
        let spear_tip = meshes.add(Cone::new(0.07, 0.25));
        commands
            .spawn((
                SceneNode,
                Npc {
                    name,
                    mode: NpcMode::Ready,
                    lines,
                    shop: shop_key,
                },
                Interactable {
                    prompt: "[E] Parla".to_string(),
                    range: 3.0,
                    aim: Some(0.5),
                    use_from: None,
                },
                crate::hud::NameAnchor(1.3),
                Villager {
                    def,
                    phase: Phase::Work,
                    path: VecDeque::new(),
                    placed: false,
                },
                RigidBody::Kinematic,
                Collider::capsule(0.35, 1.0),
                Mesh3d(figure.clone()),
                MeshMaterial3d(skin),
                Transform::from_xyz(pos.x, CENTER_Y, pos.y)
                    .with_rotation(Quat::from_rotation_y(yaw_towards(look))),
                Visibility::default(),
            ))
            .with_children(|c| {
                hands.spawn(c, Vec3::new(0.7, 1.6, 0.7));
                if guard {
                    c.spawn((
                        Mesh3d(helmet.clone()),
                        MeshMaterial3d(steel.clone()),
                        Transform::from_xyz(0.0, 0.86, 0.0),
                    ));
                    c.spawn((
                        Mesh3d(spear.clone()),
                        MeshMaterial3d(wood.clone()),
                        Transform::from_xyz(0.55, 0.4, 0.0),
                    ));
                    c.spawn((
                        Mesh3d(spear_tip.clone()),
                        MeshMaterial3d(steel.clone()),
                        Transform::from_xyz(0.55, 1.78, 0.0),
                    ));
                    c.spawn((
                        GuardLantern,
                        Mesh3d(lantern_mesh.clone()),
                        MeshMaterial3d(lantern_mat.clone()),
                        PointLight {
                            color: Color::srgb(1.0, 0.8, 0.5),
                            intensity: 0.0,
                            range: 9.0,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_xyz(-0.55, 0.1, -0.25),
                    ));
                }
            });
    }
}

/// Grafo delle strade per camminare lungo di esse.
pub struct RoadGraph {
    nodes: Vec<Vec2>,
    edges: Vec<Vec<usize>>,
}

impl RoadGraph {
    fn build(roads: &[Road]) -> Self {
        let mut nodes: Vec<Vec2> = Vec::new();
        let mut road_nodes: Vec<Vec<usize>> = Vec::new();
        for r in roads.iter().filter(|r| r.width < 8.0) {
            let ids: Vec<usize> = r
                .pts
                .iter()
                .map(|p| {
                    nodes.push(*p);
                    nodes.len() - 1
                })
                .collect();
            road_nodes.push(ids);
        }
        let mut edges = vec![Vec::new(); nodes.len()];
        let link = |a: usize, b: usize, edges: &mut Vec<Vec<usize>>| {
            edges[a].push(b);
            edges[b].push(a);
        };
        for ids in &road_nodes {
            for w in ids.windows(2) {
                link(w[0], w[1], &mut edges);
            }
        }
        // Strade diverse che si toccano (entro 2,5 m): collegate.
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                if nodes[i].distance(nodes[j]) < 2.5 && !edges[i].contains(&j) {
                    link(i, j, &mut edges);
                }
            }
        }
        Self { nodes, edges }
    }

    fn nearest(&self, p: Vec2) -> Option<usize> {
        (0..self.nodes.len()).min_by(|a, b| {
            self.nodes[*a]
                .distance(p)
                .total_cmp(&self.nodes[*b].distance(p))
        })
    }

    /// Percorso da `a` a `b` lungo le strade (Dijkstra); il punto iniziale non è incluso.
    fn route(&self, a: Vec2, b: Vec2) -> VecDeque<Vec2> {
        let (Some(s), Some(g)) = (self.nearest(a), self.nearest(b)) else {
            return VecDeque::from([b]);
        };
        let n = self.nodes.len();
        let mut dist = vec![f32::MAX; n];
        let mut prev = vec![usize::MAX; n];
        let mut done = vec![false; n];
        dist[s] = 0.0;
        for _ in 0..n {
            let Some(u) = (0..n)
                .filter(|i| !done[*i] && dist[*i] < f32::MAX)
                .min_by(|x, y| dist[*x].total_cmp(&dist[*y]))
            else {
                break;
            };
            done[u] = true;
            if u == g {
                break;
            }
            for &v in &self.edges[u] {
                let nd = dist[u] + self.nodes[u].distance(self.nodes[v]);
                if nd < dist[v] {
                    dist[v] = nd;
                    prev[v] = u;
                }
            }
        }
        let mut path = VecDeque::from([b]);
        let mut cur = g;
        while cur != usize::MAX && cur != s {
            path.push_front(self.nodes[cur]);
            cur = prev[cur];
        }
        path.push_front(self.nodes[s]);
        path
    }
}

/// Quota del piano di un molo se (x, z) ne cade nell'impronta (ponte: x ±1,2, z da -2 a 12).
pub fn pier_deck_at(items: &[crate::debug_place::Placement], x: f32, z: f32) -> Option<f32> {
    for pl in items.iter().filter(|p| p.kind == "molo") {
        for (j, &p) in pl.points.iter().enumerate() {
            let d = Vec3::new(x - p.x, 0.0, z - p.y);
            let l = Quat::from_rotation_y(-pl.yaw(j)) * d;
            if l.x.abs() <= 1.2 && (-2.0..=12.0).contains(&l.z) {
                return Some(crate::terrain::SEA_LEVEL + crate::buildings::PIER_DECK_TOP);
            }
        }
    }
    None
}

/// Quota del suolo: terreno, o piano del molo se più alto.
fn ground_y(field: &TerrainField, items: &[crate::debug_place::Placement], x: f32, z: f32) -> f32 {
    let g = field.height_at(x, z);
    pier_deck_at(items, x, z).map_or(g, |d| d.max(g))
}

/// Sistema: routine degli abitanti (posto di giorno, piazza la sera, via di notte) e lanterne delle guardie.
#[allow(clippy::too_many_arguments)]
pub fn villager_routine(
    mut commands: Commands,
    time: Res<Time>,
    clock: Res<GameClock>,
    field: Option<Res<TerrainField>>,
    dbg: Res<crate::debug_place::DebugPlace>,
    info: Res<VillageInfo>,
    dialog: Res<DialogState>,
    mut graph: Local<Option<RoadGraph>>,
    mut q: Query<(
        Entity,
        &mut Transform,
        &mut Npc,
        &mut Villager,
        &mut Visibility,
    )>,
    mut lanterns: Query<&mut PointLight, With<GuardLantern>>,
) {
    let Some(field) = field else {
        return;
    };
    if graph.is_none() && !info.spec.roads.is_empty() {
        *graph = Some(RoadGraph::build(&info.spec.roads));
    }
    let dt = time.delta_secs();
    let hour = clock.hour;
    // Lanterne accese di notte.
    let dark = !(6.0..19.0).contains(&hour);
    for mut l in &mut lanterns {
        let target = if dark { 60_000.0 } else { 0.0 };
        l.intensity += (target - l.intensity) * (dt * 2.0).min(1.0);
    }
    for (entity, mut tf, mut npc, mut v, mut vis) in &mut q {
        let want = desired_phase(&v.def, hour);
        // Primo frame: si mette direttamente nella fase giusta.
        if !v.placed {
            v.placed = true;
            v.phase = want;
            let (p, look) = spot(&v.def, want);
            tf.translation = Vec3::new(p.x, 0.0, p.y);
            tf.rotation = Quat::from_rotation_y(yaw_towards(look));
            *vis = if want == Phase::Night {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        }
        // Cambio di fase: si parte (o si compare/scompare).
        if want != v.phase && v.path.is_empty() {
            let from = Vec2::new(tf.translation.x, tf.translation.z);
            let (to, _) = spot(&v.def, want);
            if want == Phase::Night {
                *vis = Visibility::Hidden;
                npc.mode = NpcMode::Busy;
                commands.entity(entity).insert(ColliderDisabled);
                v.phase = want;
                continue;
            }
            if v.phase == Phase::Night {
                // Esce di casa: compare al suo posto.
                let (p, _) = spot(&v.def, want);
                tf.translation = Vec3::new(p.x, 0.0, p.y);
                *vis = Visibility::Inherited;
                v.phase = want;
            } else {
                v.path = graph
                    .as_ref()
                    .map_or_else(|| VecDeque::from([to]), |g| g.route(from, to));
                v.phase = want;
            }
        }
        if v.phase == Phase::Night {
            continue;
        }
        let talking = dialog.open && dialog.speaker_entity == Some(entity);
        let here = Vec2::new(tf.translation.x, tf.translation.z);
        if let Some(&next) = v.path.front() {
            npc.mode = NpcMode::Busy;
            commands.entity(entity).insert(ColliderDisabled);
            if !talking {
                let to = next - here;
                if to.length() < 0.15 {
                    v.path.pop_front();
                } else {
                    let step = (WALK_SPEED * dt).min(to.length());
                    let dir = to.normalize();
                    tf.translation.x += dir.x * step;
                    tf.translation.z += dir.y * step;
                    let want_rot = Quat::from_rotation_y(yaw_towards(dir));
                    tf.rotation = tf.rotation.slerp(want_rot, (8.0 * dt).clamp(0.0, 1.0));
                }
            }
        } else {
            let (_, look) = spot(&v.def, v.phase);
            if !talking {
                let want_rot = Quat::from_rotation_y(yaw_towards(look));
                tf.rotation = tf.rotation.slerp(want_rot, (6.0 * dt).clamp(0.0, 1.0));
            }
            if npc.mode == NpcMode::Busy {
                npc.mode = NpcMode::Ready;
                commands.entity(entity).remove::<ColliderDisabled>();
            }
        }
        tf.translation.y =
            ground_y(&field, &dbg.items, tf.translation.x, tf.translation.z) + CENTER_Y;
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, villager_routine);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_place::{PlaceMode, Placement};

    #[test]
    fn pier_deck_is_found_only_on_the_deck() {
        let pl = Placement {
            id: 1,
            kind: "molo".into(),
            mode: PlaceMode::Single,
            a: Vec2::new(8.5, 96.3),
            b: Vec2::ZERO,
            radius: 0.0,
            poly: Vec::new(),
            seed: 0,
            note: String::new(),
            when: String::new(),
            points: vec![Vec2::new(8.5, 96.3)],
            yaws: vec![0.0],
        };
        let items = [pl];
        assert!(pier_deck_at(&items, 8.5, 103.0).is_some());
        assert!(pier_deck_at(&items, 11.0, 103.0).is_none());
        assert!(pier_deck_at(&items, 8.5, 80.0).is_none());
    }

    #[test]
    fn spec_has_villagers_and_guards() {
        let v = parse(SPEC);
        assert!(v.len() >= 8);
        assert!(v.iter().filter(|d| d.guard).count() >= 4);
        assert!(v.iter().any(|d| d.name == "Don Anselmo"));
    }

    #[test]
    fn phases_follow_hours() {
        let d = VillagerDef {
            hours: (6.0, 18.0, 22.0),
            ..Default::default()
        };
        assert_eq!(desired_phase(&d, 10.0), Phase::Work);
        assert_eq!(desired_phase(&d, 19.0), Phase::Evening);
        assert_eq!(desired_phase(&d, 23.0), Phase::Night);
        assert_eq!(desired_phase(&d, 3.0), Phase::Night);
    }

    #[test]
    fn route_walks_along_roads() {
        let roads = vec![Road {
            width: 2.0,
            pts: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 10.0),
            ],
        }];
        let g = RoadGraph::build(&roads);
        let r = g.route(Vec2::new(0.5, 1.0), Vec2::new(9.5, 10.0));
        assert!(r.len() >= 3);
        assert_eq!(*r.back().unwrap(), Vec2::new(9.5, 10.0));
    }
}
