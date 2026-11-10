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
/// Passo di ronda sulle mura (m/s).
const PATROL_SPEED: f32 = 1.3;
const CENTER_Y: f32 = 0.85;

/// Dati di un abitante letti da `abitanti.txt`.
#[derive(Clone, Debug, Default)]
pub struct VillagerDef {
    pub name: String,
    pub role: String,
    /// Aspetto: file in `assets/models/characters/`.
    pub model: String,
    pub guard: bool,
    /// Assortimento del negozio (chiave di `shop.rs`), se è un venditore.
    pub shop: Option<String>,
    /// Cancello (nome) e turno (1..=3) delle guardie di turno.
    pub gate: Option<String>,
    pub shift: Option<u8>,
    /// Camminamento (etichetta) delle guardie di ronda sulle mura.
    pub wall: Option<String>,
    /// Posto di lavoro e direzione dello sguardo (dx, dz).
    pub work: (Vec2, Vec2),
    /// Posto serale (se diverso) e sguardo.
    pub evening: Option<(Vec2, Vec2)>,
    /// Inizio lavoro, fine lavoro (va in piazza), fine serata (torna a casa).
    pub hours: (f32, f32, f32),
    /// Casa: tipo di edificio, quale tra quelli di quel tipo, posto letto.
    pub home: Option<(String, usize, usize)>,
    /// Chiavi in più (edifici dove lavora): nomi come `chiesa 0`.
    pub extra_keys: Vec<String>,
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
            "modello" => d.model = val.trim().to_string(),
            "guardia" => d.guard = val.trim() == "si",
            "negozio" => d.shop = Some(val.trim().to_string()),
            "cancello" => d.gate = Some(val.trim().to_string()),
            "turno" => d.shift = val.trim().parse().ok().filter(|t| (1..=3).contains(t)),
            "muro" => d.wall = Some(val.trim().to_lowercase()),
            "lavoro" => {
                if let Some(p) = pos(&nums) {
                    d.work = p;
                }
            }
            "sera" => d.evening = pos(&nums),
            "orari" if nums.len() >= 3 => d.hours = (nums[0], nums[1], nums[2]),
            "chiave" => {
                let key = val.split_whitespace().collect::<Vec<_>>().join(" ");
                if !key.is_empty() {
                    d.extra_keys.push(key);
                }
            }
            "casa" => {
                let mut t = val.split_whitespace();
                if let (Some(kind), Some(n), Some(bed)) = (t.next(), t.next(), t.next())
                    && let (Ok(n), Ok(bed)) = (n.parse(), bed.parse())
                {
                    d.home = Some((kind.to_string(), n, bed));
                }
            }
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
    /// Guardia fuori turno: va alla casa delle guardie.
    Rest,
}

/// Sosta a una finestra per aprire o chiudere gli scuri.
struct Chore {
    stand: Vec3,
    face: Vec2,
    state: Entity,
    /// Scuri da aprire (mattina) o da chiudere (sera).
    open: bool,
    timer: f32,
    /// Il gesto è a metà: `villager_chores` cambia lo stato degli scuri.
    fire: bool,
    done: bool,
}

/// Componente di un abitante (routine).
#[derive(Component)]
pub struct Villager {
    def: VillagerDef,
    phase: Phase,
    path: VecDeque<Vec2>,
    placed: bool,
    /// Nascosto (in casa o alla caserma).
    hidden: bool,
    /// Distanza percorsa sull'anello delle mura (guardie di ronda).
    patrol: f32,
    /// Percorso dentro casa (mondo, piedi): ha la precedenza sulla strada.
    indoor: VecDeque<Vec3>,
    /// È già entrato: quando `indoor` finisce si mette a letto.
    entered: bool,
    /// Appena letto da un salvataggio: la routine riprende da dove era.
    resume: bool,
    /// Sosta alla finestra in corso (apre o chiude gli scuri).
    chore: Option<Chore>,
}

/// Stato di un abitante nel salvataggio.
#[derive(Clone, Debug, PartialEq)]
pub struct VillagerSave {
    pub name: String,
    pub phase: String,
    pub hidden: bool,
    pub pos: Vec3,
    pub rot: Quat,
    pub patrol: f32,
}

impl Villager {
    /// Sta facendo il gesto alla finestra (animazione `Interact`).
    pub fn interacting(&self) -> bool {
        self.chore.as_ref().is_some_and(|c| c.timer > 0.0)
    }

    pub fn name(&self) -> &str {
        &self.def.name
    }

    /// Fotografia: chi è a metà di una scala viene considerato già arrivato in fondo al percorso.
    pub fn snapshot(&self, tf: &Transform) -> VillagerSave {
        let pos = self
            .indoor
            .back()
            .map_or(tf.translation, |p| *p + Vec3::Y * CENTER_Y);
        VillagerSave {
            name: self.def.name.clone(),
            phase: format!("{:?}", self.phase),
            hidden: self.hidden,
            pos,
            rot: tf.rotation,
            patrol: self.patrol,
        }
    }

    /// Riprende da un salvataggio; la routine ricalcola il percorso al frame dopo.
    pub fn restore(&mut self, saved: &VillagerSave, tf: &mut Transform) {
        self.phase = match saved.phase.as_str() {
            "Evening" => Phase::Evening,
            "Night" => Phase::Night,
            "Rest" => Phase::Rest,
            _ => Phase::Work,
        };
        self.hidden = saved.hidden;
        self.patrol = saved.patrol;
        self.placed = true;
        self.resume = true;
        self.entered = false;
        self.path.clear();
        self.indoor.clear();
        tf.translation = saved.pos;
        tf.rotation = saved.rot;
    }
}

/// Inventario di un abitante: le chiavi che porta con sé.
#[derive(Component, Default, Debug)]
pub struct NpcInventory {
    pub keys: Vec<String>,
}

impl NpcInventory {
    pub fn has(&self, key: &str) -> bool {
        self.keys.iter().any(|k| k == key)
    }

    fn of(def: &VillagerDef) -> Self {
        let mut keys: Vec<String> = def
            .home
            .iter()
            .map(|(kind, n, _)| format!("{kind} {n}"))
            .collect();
        keys.extend(def.extra_keys.iter().cloned());
        Self { keys }
    }
}

/// Lanterna di una guardia (accesa di notte).
#[derive(Component)]
pub struct GuardLantern;

/// Lancia e lanterna: si posano quando la guardia va a letto.
#[derive(Component)]
pub struct GuardGear;

type GearItem = (Entity, &'static mut Visibility);
type GearOnly = (With<GuardGear>, Without<Villager>);

/// Turni delle guardie (ore di gioco, fine esclusa; il terzo attraversa la mezzanotte).
const SHIFTS: [(f32, f32); 3] = [(6.0, 14.0), (14.0, 22.0), (22.0, 30.0)];
/// Anticipo (h) con cui il cambio turno raggiunge il posto: la caserma sta fuori dal cancello nord: la camminata fino a Porta Sud dura ~45 min.
const SHIFT_EARLY: f32 = 1.0;

/// La guardia del turno `shift` (1..=3) è in servizio a quest'ora?
pub fn on_duty(shift: u8, hour: f32) -> bool {
    let (a, b) = SHIFTS[(shift as usize).clamp(1, 3) - 1];
    let a = a - SHIFT_EARLY;
    (a..b).contains(&hour) || (a..b).contains(&(hour + 24.0))
}

fn desired_phase(def: &VillagerDef, hour: f32, shelter: bool) -> Phase {
    if def.guard {
        return match def.shift {
            Some(t) if !on_duty(t, hour) => Phase::Rest,
            _ => Phase::Work,
        };
    }
    // Con pioggia o neve forti gli abitanti si riparano in casa.
    if shelter {
        return Phase::Night;
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

/// Posto di una guardia di cancello: subito oltre la punta dell'anta aperta (che si apre verso
/// l'interno lungo il lato del vano), ai bordi della strada; un posto diverso per ogni turno.
pub fn gate_post(gate: &crate::village::Gate, shift: u8) -> (Vec2, Vec2) {
    let (lateral, dist) = match shift {
        1 => (-1.5, 3.2),
        2 => (1.5, 3.2),
        _ => (-1.5, 4.6),
    };
    (
        gate.pos + gate.inward * dist + gate.tangent * lateral,
        -gate.inward,
    )
}

/// Posizione iniziale sull'anello di una guardia di ronda (frazione del giro).
fn patrol_start(wall: &str) -> f32 {
    match wall {
        "nord" => 0.0,
        "est" => 0.25,
        "sud" => 0.5,
        _ => 0.75,
    }
}

fn spot(def: &VillagerDef, phase: Phase, info: &VillageInfo) -> (Vec2, Vec2) {
    match phase {
        Phase::Evening => def.evening.unwrap_or(def.work),
        Phase::Rest => info.barracks.map_or(def.work, |(door, out)| (door, -out)),
        _ => {
            if let (Some(name), Some(shift)) = (&def.gate, def.shift)
                && let Some(g) = info
                    .gates
                    .iter()
                    .find(|g| g.name.eq_ignore_ascii_case(name))
            {
                return gate_post(g, shift);
            }
            def.work
        }
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
    let steel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.56, 0.6),
        metallic: 0.5,
        perceptual_roughness: 0.5,
        ..default()
    });
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.27, 0.15),
        perceptual_roughness: 0.85,
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
        let model = def.model.clone();
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
                NpcInventory::of(&def),
                Villager {
                    def,
                    phase: Phase::Work,
                    path: VecDeque::new(),
                    placed: false,
                    hidden: false,
                    patrol: 0.0,
                    indoor: VecDeque::new(),
                    entered: false,
                    resume: false,
                    chore: None,
                },
                RigidBody::Kinematic,
                Collider::capsule(0.35, 1.0),
                Transform::from_xyz(pos.x, CENTER_Y, pos.y)
                    .with_rotation(Quat::from_rotation_y(yaw_towards(look))),
                Visibility::default(),
            ))
            .with_children(|c| {
                crate::figure::spawn_character(c, asset_server, &model);
                if guard {
                    // Lancia nella destra: l'asse dell'osso `hand_r` punta verso le dita (in basso a riposo).
                    c.spawn((
                        GuardGear,
                        crate::figure::AttachToHand { right: true },
                        // Impugnata a un terzo dell'asta, dritta e con la punta in alto: un bastone d'arma.
                        Transform::from_xyz(-0.02, -0.15, 0.04)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
                        Visibility::default(),
                    ))
                    .with_children(|s| {
                        s.spawn((Mesh3d(spear.clone()), MeshMaterial3d(wood.clone())));
                        s.spawn((
                            Mesh3d(spear_tip.clone()),
                            MeshMaterial3d(steel.clone()),
                            Transform::from_xyz(0.0, 1.38, 0.0),
                        ));
                    });
                    c.spawn((
                        GuardLantern,
                        GuardGear,
                        crate::figure::AttachToHand { right: false },
                        Mesh3d(lantern_mesh.clone()),
                        MeshMaterial3d(lantern_mat.clone()),
                        PointLight {
                            color: Color::srgb(1.0, 0.8, 0.5),
                            intensity: 0.0,
                            range: 9.0,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_xyz(0.0, 0.26, 0.0),
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

/// Letto di un abitante: serve la chiave di casa, altrimenti non può entrare.
struct HomeRef<'a> {
    door: Vec2,
    bed: &'a crate::world_props::HomeBed,
    windows: &'a [crate::world_props::HomeWindow],
}

/// Sosta alla finestra più vicina al letto, se ce n'è una sullo stesso piano.
fn window_chore(h: &HomeRef, open: bool) -> Option<Chore> {
    let w = &h.windows[crate::world_props::HomeWindow::pick(h.windows, h.bed)?];
    Some(Chore {
        stand: w.stand,
        face: w.face,
        state: w.state?,
        open,
        timer: 0.0,
        fire: false,
        done: false,
    })
}

fn home_of<'a>(
    def: &VillagerDef,
    keys: &NpcInventory,
    homes: &'a crate::world_props::Homes,
) -> Option<HomeRef<'a>> {
    let (kind, n, bed) = def.home.as_ref()?;
    let home = homes.find(kind, *n)?;
    keys.has(&home.key()).then_some(())?;
    Some(HomeRef {
        door: home.door,
        bed: home.beds.get(*bed)?,
        windows: &home.windows,
    })
}

/// Mette l'abitante a letto, supino con la testa verso la testata.
fn lie_in_bed(tf: &mut Transform, pos: Vec3, head: Vec3) {
    tf.translation = pos + Vec3::Y * 0.18;
    tf.rotation = Quat::from_rotation_y(head.x.atan2(head.z))
        * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
}

/// Sistema: le lanterne delle guardie vicine al giocatore fanno ombra, così la luce non attraversa i muri.
pub fn lantern_shadows(
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    mut lanterns: Query<(&mut PointLight, &GlobalTransform), With<GuardLantern>>,
) {
    let Ok(me) = players.single() else {
        return;
    };
    for (mut l, g) in &mut lanterns {
        let near = me.translation().distance(g.translation()) < 22.0;
        if l.shadow_maps_enabled != near {
            l.shadow_maps_enabled = near;
        }
    }
}

/// Sistema: routine degli abitanti (posto di giorno, piazza la sera, a letto di notte) e lanterne delle guardie.
#[allow(clippy::too_many_arguments)]
pub fn villager_routine(
    mut commands: Commands,
    time: Res<Time>,
    clock: Res<GameClock>,
    weather: Res<crate::weather::WeatherState>,
    field: Option<Res<TerrainField>>,
    dbg: Res<crate::debug_place::DebugPlace>,
    info: Res<VillageInfo>,
    dialog: Res<DialogState>,
    chat: Res<crate::ai::AiChat>,
    app_state: Res<State<crate::AppState>>,
    homes: Res<crate::world_props::Homes>,
    parents: Query<&ChildOf>,
    mut graph: Local<Option<RoadGraph>>,
    mut q: Query<(
        Entity,
        &mut Transform,
        &mut Npc,
        &NpcInventory,
        &mut Villager,
        &mut Visibility,
    )>,
    mut lanterns: Query<(Entity, &mut PointLight), With<GuardLantern>>,
    mut gear: Query<GearItem, GearOnly>,
) {
    let Some(field) = field else {
        return;
    };
    if graph.is_none() && !info.spec.roads.is_empty() {
        *graph = Some(RoadGraph::build(&info.spec.roads));
    }
    let dt = time.delta_secs();
    let hour = clock.hour;
    // Lanterne accese di notte, spente per chi dorme.
    let dark = clock.guard_lanterns_on();
    for (entity, mut l) in &mut lanterns {
        let asleep = parents
            .iter_ancestors(entity)
            .any(|a| q.get(a).is_ok_and(|(.., v, _)| v.hidden));
        let target = if dark && !asleep { 60_000.0 } else { 0.0 };
        l.intensity += (target - l.intensity) * (dt * 2.0).min(1.0);
    }
    for (entity, mut vis) in &mut gear {
        let asleep = parents
            .iter_ancestors(entity)
            .any(|a| q.get(a).is_ok_and(|(.., v, _)| v.hidden));
        // Le lanterne si portano soltanto al buio: di giorno restano riposte.
        let stowed = lanterns.contains(entity) && !dark;
        let wanted = if asleep || stowed {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *vis != wanted {
            *vis = wanted;
        }
    }
    for (entity, mut tf, mut npc, keys, mut v, mut vis) in &mut q {
        let want = desired_phase(&v.def, hour, weather.shelter);
        let here = Vec2::new(tf.translation.x, tf.translation.z);
        // Guardie di ronda: percorrono l'anello delle mura senza fermarsi, salvo quando parlano.
        if let Some(wall) = v.def.wall.clone() {
            if info.patrol.len() < 2 {
                *vis = Visibility::Hidden;
                continue;
            }
            let length = crate::village::loop_length(&info.patrol);
            if !v.placed {
                v.placed = true;
                v.patrol = patrol_start(&wall) * length;
                *vis = Visibility::Inherited;
            }
            // Si ferma anche mentre si chiacchiera con la IA (stato Chat), non solo nel dialogo scritto.
            let chatting = *app_state.get() == crate::AppState::Chat && chat.npc_name == v.def.name;
            let talking = (dialog.open && dialog.speaker_entity == Some(entity)) || chatting;
            if !talking {
                v.patrol = (v.patrol + PATROL_SPEED * dt).rem_euclid(length);
            }
            let (p, dir) = crate::village::point_on_loop(&info.patrol, v.patrol);
            tf.translation = p + Vec3::Y * CENTER_Y;
            if !talking {
                let want_rot = Quat::from_rotation_y(yaw_towards(dir));
                tf.rotation = tf.rotation.slerp(want_rot, (6.0 * dt).clamp(0.0, 1.0));
            }
            continue;
        }
        // Aspetta che gli edifici esistano: i letti servono per piazzarli.
        if !homes.built {
            continue;
        }
        let home = home_of(&v.def, keys, &homes);
        // Primo frame: si mette direttamente nella fase giusta.
        if !v.placed {
            v.placed = true;
            v.phase = want;
            let (p, look) = spot(&v.def, want, &info);
            tf.translation = Vec3::new(p.x, 0.0, p.y);
            tf.rotation = Quat::from_rotation_y(yaw_towards(look));
            v.hidden = matches!(want, Phase::Night | Phase::Rest);
            if v.hidden {
                npc.mode = NpcMode::Busy;
                commands.entity(entity).insert(ColliderDisabled);
                if let Some(h) = &home {
                    lie_in_bed(&mut tf, h.bed.pos, h.bed.head);
                    *vis = Visibility::Inherited;
                } else {
                    *vis = Visibility::Hidden;
                }
            } else {
                *vis = Visibility::Inherited;
            }
        }
        // Appena caricato: chi non è a letto riprende la strada verso il suo posto.
        if v.resume {
            v.resume = false;
            if v.hidden {
                npc.mode = NpcMode::Busy;
                commands.entity(entity).insert(ColliderDisabled);
                *vis = if home.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            } else {
                *vis = Visibility::Inherited;
                let (to, _) = spot(&v.def, v.phase, &info);
                let dest = match (&home, v.phase) {
                    (Some(h), Phase::Night | Phase::Rest) => h.door,
                    _ => to,
                };
                if here.distance(dest) > 0.5 {
                    v.path = graph
                        .as_ref()
                        .map_or_else(|| VecDeque::from([dest]), |g| g.route(here, dest));
                }
            }
        }
        // Cambio di fase: si parte (o si compare/scompare).
        if want != v.phase && v.path.is_empty() && v.indoor.is_empty() {
            let (to, _) = spot(&v.def, want, &info);
            let going_to_bed = matches!(want, Phase::Night | Phase::Rest);
            if want == Phase::Night && home.is_none() {
                *vis = Visibility::Hidden;
                v.hidden = true;
                npc.mode = NpcMode::Busy;
                commands.entity(entity).insert(ColliderDisabled);
                v.phase = want;
                continue;
            }
            if v.hidden {
                if going_to_bed {
                    // Da riposo a riposo (riparo dalla pioggia finito di notte): resta dov'è.
                    v.phase = want;
                    continue;
                }
                // Si alza, scende le scale ed esce; senza casa compare al lavoro o alla caserma.
                let from_rest = v.phase == Phase::Rest;
                let appear = match &home {
                    Some(h) => h.door,
                    None if from_rest => spot(&v.def, Phase::Rest, &info).0,
                    None => to,
                };
                tf.translation = Vec3::new(appear.x, 0.0, appear.y);
                tf.rotation = Quat::from_rotation_y(tf.rotation.to_euler(EulerRot::YXZ).0);
                if let Some(h) = &home {
                    let mut steps: VecDeque<Vec3> = h.bed.path.iter().rev().copied().collect();
                    if let Some(start) = steps.pop_front() {
                        tf.translation = start + Vec3::Y * CENTER_Y;
                    }
                    // Prima di scendere apre gli scuri della sua stanza.
                    v.chore = window_chore(h, true);
                    if let Some(c) = &v.chore {
                        steps.push_front(c.stand);
                    }
                    v.indoor = steps;
                    info!("[VILLAGERS] {} si alza", v.def.name);
                }
                *vis = Visibility::Inherited;
                v.hidden = false;
                if home.is_some() || from_rest {
                    v.path = graph
                        .as_ref()
                        .map_or_else(|| VecDeque::from([to]), |g| g.route(appear, to));
                }
                v.phase = want;
            } else {
                let dest = match &home {
                    Some(h) if going_to_bed => h.door,
                    _ if want == Phase::Rest => spot(&v.def, Phase::Rest, &info).0,
                    _ => to,
                };
                v.path = graph
                    .as_ref()
                    .map_or_else(|| VecDeque::from([dest]), |g| g.route(here, dest));
                v.phase = want;
            }
        }
        if v.hidden {
            continue;
        }
        let talking = dialog.open && dialog.speaker_entity == Some(entity);
        // Dentro casa si cammina in 3D, sulle rampe e sui pavimenti.
        if let Some(&next) = v.indoor.front() {
            npc.mode = NpcMode::Busy;
            commands.entity(entity).insert(ColliderDisabled);
            if !talking {
                let to = next + Vec3::Y * CENTER_Y - tf.translation;
                if to.length() < 0.1 {
                    if let Some(c) = v.chore.as_mut().filter(|c| c.stand.distance(next) < 0.01) {
                        // Sosta alla finestra: si volta, fa il gesto, a metà cambia gli scuri.
                        c.timer += dt;
                        let want_rot = Quat::from_rotation_y(yaw_towards(c.face));
                        tf.rotation = tf.rotation.slerp(want_rot, (8.0 * dt).clamp(0.0, 1.0));
                        if c.timer > 0.6 && !c.done {
                            c.done = true;
                            c.fire = true;
                        }
                        if c.timer > 1.4 {
                            v.chore = None;
                            v.indoor.pop_front();
                        }
                    } else {
                        v.indoor.pop_front();
                    }
                } else {
                    let step = (WALK_SPEED * dt).min(to.length());
                    tf.translation += to.normalize() * step;
                    let flat = Vec2::new(to.x, to.z);
                    if flat.length() > 0.05 {
                        let want_rot = Quat::from_rotation_y(yaw_towards(flat.normalize()));
                        tf.rotation = tf.rotation.slerp(want_rot, (8.0 * dt).clamp(0.0, 1.0));
                    }
                }
            }
            continue;
        }
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
        } else if matches!(v.phase, Phase::Night | Phase::Rest) {
            // Arrivato alla porta: entra e sale; in fondo al percorso si sdraia. Senza casa, sparisce.
            if let Some(h) = &home {
                if !v.entered {
                    v.entered = true;
                    let mut path: VecDeque<Vec3> = h.bed.path.iter().copied().collect();
                    // Prima di coricarsi chiude gli scuri della sua stanza.
                    v.chore = window_chore(h, false);
                    if let Some(c) = &v.chore
                        && let Some(last) = path.pop_back()
                    {
                        path.push_back(c.stand);
                        path.push_back(last);
                    }
                    v.indoor = path;
                    info!("[VILLAGERS] {} entra in casa", v.def.name);
                    continue;
                }
                lie_in_bed(&mut tf, h.bed.pos, h.bed.head);
                info!("[VILLAGERS] {} è a letto", v.def.name);
            } else {
                *vis = Visibility::Hidden;
            }
            v.entered = false;
            v.hidden = true;
            npc.mode = NpcMode::Busy;
            commands.entity(entity).insert(ColliderDisabled);
            continue;
        } else {
            let (_, look) = spot(&v.def, v.phase, &info);
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

/// Sistema: a metà del gesto alla finestra gli scuri si aprono o si chiudono.
pub fn villager_chores(
    mut villagers: Query<&mut Villager>,
    mut shutters: Query<&mut crate::hut::PropState>,
) {
    for mut v in &mut villagers {
        let name = v.def.name.clone();
        let Some(c) = v.chore.as_mut().filter(|c| c.fire) else {
            continue;
        };
        c.fire = false;
        if let Ok(mut st) = shutters.get_mut(c.state) {
            st.on = c.open;
            info!(
                "[VILLAGERS] {name} {} gli scuri",
                if c.open { "apre" } else { "chiude" }
            );
        }
    }
}

/// Sistema: la porta si apre davanti a chi ne ha la chiave e si richiude dopo; alla chiusura la bottega si chiude.
pub fn villager_doors(
    time: Res<Time>,
    clock: Res<GameClock>,
    mut homes: ResMut<crate::world_props::Homes>,
    walkers: Query<(&Transform, &Villager, &NpcInventory)>,
    mut doors: Query<(&mut crate::hut::PropState, &crate::hut::Lock)>,
) {
    /// Distanza dalla porta a cui si apre e tempo prima di richiuderla.
    const OPEN_NEAR: f32 = 2.0;
    const CLOSE_AFTER: f32 = 1.5;
    for home in &mut homes.list {
        let Some(Ok((mut state, lock))) = home.state.map(|e| doors.get_mut(e)) else {
            continue;
        };
        let key = home.key();
        let near = walkers.iter().any(|(t, v, keys)| {
            !v.indoor.is_empty()
                && keys.has(&key)
                && (t.translation - home.door_point).length() < OPEN_NEAR
        });
        if near {
            home.idle = 0.0;
            if !state.on {
                state.on = true;
                home.npc_open = true;
            }
        } else if home.npc_open {
            home.idle += time.delta_secs();
            if home.idle > CLOSE_AFTER {
                state.on = false;
                home.npc_open = false;
            }
        }
        let open = lock.open_to_all(clock.hour);
        if home.was_open && !open && !near && state.on {
            state.on = false;
        }
        home.was_open = open;
    }
}

/// Sistema: all'avvio segnala chi lavora davanti a una porta (anche dopo spostamenti con F4).
pub fn warn_blocked_doors(homes: Res<crate::world_props::Homes>, mut done: Local<bool>) {
    /// Distanza minima dal punto davanti alla porta (m).
    const MIN_FROM_DOOR: f32 = 2.5;
    if *done || !homes.built {
        return;
    }
    *done = true;
    for def in load_defs() {
        let spots = std::iter::once(def.work.0).chain(def.evening.map(|e| e.0));
        for spot in spots {
            if def.gate.is_none()
                && def.wall.is_none()
                && let Some(h) = homes
                    .list
                    .iter()
                    .find(|h| h.door.distance(spot) < MIN_FROM_DOOR)
            {
                warn!(
                    "[VILLAGERS] {} sta davanti alla porta di {}",
                    def.name,
                    h.key()
                );
            }
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            (
                villager_routine,
                villager_chores,
                villager_doors,
                warn_blocked_doors,
            )
                .chain(),
            lantern_shadows,
        ),
    );
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
        assert_eq!(desired_phase(&d, 10.0, false), Phase::Work);
        assert_eq!(desired_phase(&d, 19.0, false), Phase::Evening);
        assert_eq!(desired_phase(&d, 23.0, false), Phase::Night);
        assert_eq!(desired_phase(&d, 3.0, false), Phase::Night);
    }

    #[test]
    fn route_walks_along_roads() {
        let roads = vec![Road {
            dirt: false,
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

    #[test]
    fn villagers_shelter_from_bad_weather_but_guards_stay() {
        let d = VillagerDef {
            hours: (6.0, 18.0, 22.0),
            ..Default::default()
        };
        assert_eq!(desired_phase(&d, 10.0, true), Phase::Night);
        let g = VillagerDef {
            guard: true,
            ..Default::default()
        };
        assert_eq!(desired_phase(&g, 10.0, true), Phase::Work);
    }

    #[test]
    fn every_gate_is_covered_at_every_hour() {
        for step in 0..(24 * 12) {
            let hour = step as f32 / 12.0;
            assert!(
                (1..=3).any(|t| on_duty(t, hour)),
                "nessuna guardia in servizio alle {hour}"
            );
        }
        // Il turno di notte attraversa la mezzanotte.
        assert!(on_duty(3, 23.0) && on_duty(3, 2.0) && !on_duty(3, 12.0));
        assert!(on_duty(1, 6.5) && !on_duty(1, 15.0));
    }

    #[test]
    fn gate_posts_are_past_the_open_leaf_and_distinct_per_shift() {
        let gate = crate::village::Gate {
            name: "Nord".into(),
            pos: Vec2::new(0.0, -8.0),
            tangent: Vec2::X,
            inward: Vec2::Y,
            edge: 0,
            along: 0.0,
        };
        let posts: Vec<Vec2> = (1..=3).map(|s| gate_post(&gate, s).0).collect();
        for p in &posts {
            // L'anta aperta si estende per 2,2 m verso l'interno: il posto sta oltre.
            assert!(p.y - gate.pos.y > 2.6, "{p}");
            assert!((p.x - gate.pos.x).abs() < 2.1, "dentro la strada");
        }
        assert!(posts[0].distance(posts[1]) > 1.0 && posts[0].distance(posts[2]) > 1.0);
    }

    #[test]
    fn spec_has_gate_and_wall_guards() {
        let v = parse(SPEC);
        let gate_guards: Vec<_> = v.iter().filter(|d| d.gate.is_some()).collect();
        assert_eq!(gate_guards.len(), 6);
        for gate in ["Nord", "Sud"] {
            for shift in 1..=3u8 {
                assert!(
                    gate_guards
                        .iter()
                        .any(|d| d.gate.as_deref() == Some(gate) && d.shift == Some(shift)),
                    "manca {gate} turno {shift}"
                );
            }
        }
        assert_eq!(v.iter().filter(|d| d.wall.is_some()).count(), 4);
    }

    #[test]
    fn every_home_has_someone_with_the_key() {
        let v = parse(SPEC);
        for d in v.iter().filter(|d| d.home.is_some()) {
            let (kind, n, _) = d.home.clone().unwrap();
            assert!(
                NpcInventory::of(d).has(&format!("{kind} {n}")),
                "{}: niente chiave",
                d.name
            );
        }
        let anselmo = v.iter().find(|d| d.name == "Don Anselmo").unwrap();
        assert!(NpcInventory::of(anselmo).has("chiesa 0"));
    }

    #[test]
    fn every_civilian_and_gate_guard_has_a_real_bed() {
        let v = parse(SPEC);
        let mut used = std::collections::HashSet::new();
        for d in v.iter().filter(|d| d.name != "Ettore" && d.wall.is_none()) {
            let (kind, _, bed) = d
                .home
                .clone()
                .unwrap_or_else(|| panic!("{} senza casa", d.name));
            let beds = crate::buildings::model(&kind, 1).interior.beds.len();
            assert!(bed < beds, "{}: letto {bed} su {beds}", d.name);
            assert!(
                used.insert(d.home.clone()),
                "{}: letto già occupato",
                d.name
            );
        }
    }
}
