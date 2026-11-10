//! Animali dell'isola (ADR 0028). Non si salvano: si ricreano secondo ora e stagione.
//!
//! Mucche e pecore sono modelli CC0 animati; gabbiani, uccelli, galline, pesci e farfalle sono
//! fatti nel codice. Dove stanno lo dice `assets/world/isola.txt` (`gregge`, `pollaio`, `gabbiani`).

use crate::ambience::daylight;
use crate::daynight::GameClock;
use crate::debug_place::DebugPlace;
use crate::nature::{Blocked, blocked_map};
use crate::player::Player;
use crate::scene::SceneNode;
use crate::seasons::SeasonLook;
use crate::terrain::{SEA_LEVEL, TerrainField};
use crate::village::VillageInfo;
use crate::water::IslandSpec;
use crate::weather::{Noise, WeatherState};
use crate::world_props::hash01;
use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use std::f32::consts::{PI, TAU};
use std::time::Duration;

/// Clip nei `.glb`, in ordine alfabetico come le scrive `scripts/build_animals_trees.py`.
pub const CLIPS: [&str; 6] = ["Death", "Idle", "Jump", "Run", "Walk", "WalkSlow"];
const IDLE: usize = 1;
const RUN: usize = 3;
const WALK: usize = 5;

/// (file, scala, velocità al passo, di corsa)
const GRAZERS: [(&str, f32, f32, f32); 2] = [("cow", 0.3, 0.75, 3.2), ("sheep", 0.24, 0.65, 3.4)];
/// Sotto questa distanza (m) gli animali si allontanano dal giocatore.
const SHY: f32 = 3.2;
/// Sotto questa distanza (m) gli uccelli a terra si alzano in volo.
const SCARE: f32 = 7.5;
const BUTTERFLY_ROAM: f32 = 26.0;
const BUTTERFLIES: usize = 14;

/// Ogni animale: serve a ricrearli dopo un reset.
#[derive(Component)]
struct Animal;

#[derive(Resource)]
struct AnimalKit {
    graphs: Vec<(Handle<AnimationGraph>, Vec<AnimationNodeIndex>)>,
    scenes: Vec<Handle<WorldAsset>>,
    hen_parts: Vec<(Handle<Mesh>, usize, Transform)>,
    hen_colors: Vec<Handle<StandardMaterial>>,
    hen_fixed: Vec<Handle<StandardMaterial>>,
    body: Handle<Mesh>,
    wing: Handle<Mesh>,
    gull: [Handle<StandardMaterial>; 2],
    songbird: [Handle<StandardMaterial>; 3],
    fish: Handle<StandardMaterial>,
    butterfly: [Handle<StandardMaterial>; 4],
}

/// Dove gli animali non possono andare.
#[derive(Resource)]
struct Pasture(Blocked);

/// Mucca o pecora.
#[derive(Component)]
struct Grazer {
    kind: usize,
    home: Vec2,
    radius: f32,
    pos: Vec2,
    yaw: f32,
    target: Option<Vec2>,
    wait: f32,
    clip: usize,
    player: Option<Entity>,
    rng: u32,
}

/// Gallina che razzola attorno a un punto.
#[derive(Component)]
struct Hen {
    home: Vec2,
    radius: f32,
    pos: Vec2,
    yaw: f32,
    target: Option<Vec2>,
    wait: f32,
    rng: u32,
}

#[derive(Component)]
struct Gull {
    center: Vec2,
    radius: f32,
    height: f32,
    speed: f32,
    phase: f32,
    /// Quota in più per scavalcare faro e tetti.
    lift: f32,
}

/// Uccellino a terra: si alza quando ci si avvicina e torna dopo un po'.
#[derive(Component)]
struct Songbird {
    home: Vec3,
    flight: Option<Vec3>,
    away: f32,
    rng: u32,
}

#[derive(Component)]
struct Fish {
    center: Vec2,
    radius: f32,
    level: f32,
    speed: f32,
    phase: f32,
}

#[derive(Component)]
struct Butterfly {
    home: Option<Vec3>,
    seed: f32,
}

/// Ampiezza e frequenza del battito d'ali di chi ha delle [`Wing`].
#[derive(Component)]
struct Flap {
    amp: f32,
    rate: f32,
    rest: f32,
}

#[derive(Component)]
struct Wing {
    side: f32,
}

// --- Regole ------------------------------------------------------------------------------------

/// Un animale può mettere le zampe qui?
fn walkable(field: &TerrainField, blocked: &Blocked, p: Vec2) -> bool {
    field.height_at(p.x, p.y) > SEA_LEVEL + 0.6
        && !field.in_fresh_water(p.x, p.y)
        && !blocked.is(p)
        && (field.height_at(p.x + 1.0, p.y) - field.height_at(p.x - 1.0, p.y)).abs() < 1.0
        && (field.height_at(p.x, p.y + 1.0) - field.height_at(p.x, p.y - 1.0)).abs() < 1.0
}

fn uniform(rng: &mut u32) -> f32 {
    (Noise::step(rng) + 1.0) * 0.5
}

/// Prova qualche punto a caso attorno a casa e restituisce il primo buono.
fn pick_spot(rng: &mut u32, home: Vec2, radius: f32, ok: impl Fn(Vec2) -> bool) -> Option<Vec2> {
    (0..10).find_map(|_| {
        let a = uniform(rng) * TAU;
        let r = radius * uniform(rng).sqrt();
        let p = home + Vec2::from_angle(a) * r;
        ok(p).then_some(p)
    })
}

/// Punto più lontano dal giocatore in direzione opposta, se ci si può andare.
fn flee_spot(pos: Vec2, from: Vec2, ok: impl Fn(Vec2) -> bool) -> Option<Vec2> {
    let away = (pos - from).normalize_or(Vec2::X);
    [0.0f32, 0.6, -0.6, 1.2, -1.2].into_iter().find_map(|turn| {
        let p = pos + Vec2::from_angle(turn).rotate(away) * 7.0;
        ok(p).then_some(p)
    })
}

/// Avvicina l'angolo `yaw` a `want` senza fare il giro lungo.
fn turn_to(yaw: f32, want: f32, step: f32) -> f32 {
    let delta = (want - yaw + PI).rem_euclid(TAU) - PI;
    yaw + delta.clamp(-step, step)
}

/// Un ostacolo solido (muro, albero, altro animale, giocatore) sta davanti a `from` entro `dist`?
fn blocked_ahead(
    spatial: &SpatialQuery,
    grounds: &Query<(), With<crate::scene::Ground>>,
    me: Entity,
    from: Vec3,
    dir: Vec2,
    dist: f32,
) -> bool {
    let Ok(d) = Dir3::new(Vec3::new(dir.x, 0.0, dir.y)) else {
        return false;
    };
    spatial
        .cast_ray_predicate(
            from,
            d,
            dist,
            true,
            &SpatialQueryFilter::from_excluded_entities([me]),
            &|e| !grounds.contains(e),
        )
        .is_some()
}

// --- Mesh e materiali --------------------------------------------------------------------------

/// Ala: un quadrilatero che parte dal perno e si estende su +X, a punta.
fn wing_mesh() -> Mesh {
    let pos = vec![
        [0.0, 0.0, -0.5],
        [1.0, 0.0, -0.18],
        [1.0, 0.0, 0.22],
        [0.0, 0.0, 0.5],
    ];
    let nor = vec![[0.0, 1.0, 0.0]; 4];
    let uv = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nor);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2]));
    m
}

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut models = Vec::new();
    let mut scenes = Vec::new();
    for (file, ..) in GRAZERS {
        let path = format!("models/animals/{file}.glb");
        let clips: Vec<Handle<AnimationClip>> = (0..CLIPS.len())
            .map(|i| server.load(GltfAssetLabel::Animation(i).from_asset(path.clone())))
            .collect();
        let (graph, nodes) = AnimationGraph::from_clips(clips);
        models.push((graphs.add(graph), nodes));
        scenes.push(server.load(GltfAssetLabel::Scene(0).from_asset(path)));
    }
    let matte = |materials: &mut Assets<StandardMaterial>, r, g, b| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            perceptual_roughness: 0.85,
            double_sided: true,
            cull_mode: None,
            ..default()
        })
    };
    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(10, 7));
    let block = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let t = |x: f32, y: f32, z: f32, s: Vec3| Transform::from_xyz(x, y, z).with_scale(s);
    // Parte, colore (0 = piume, 1 = cresta, 2 = becco e zampe), posizione.
    let hen_parts = vec![
        (
            sphere.clone(),
            0,
            t(0.0, 0.24, 0.0, Vec3::new(0.12, 0.13, 0.19)),
        ),
        (sphere.clone(), 0, t(0.0, 0.4, 0.15, Vec3::splat(0.07))),
        (
            sphere.clone(),
            0,
            t(0.0, 0.33, -0.17, Vec3::new(0.05, 0.11, 0.08)),
        ),
        (
            block.clone(),
            1,
            t(0.0, 0.48, 0.16, Vec3::new(0.02, 0.06, 0.08)),
        ),
        (
            block.clone(),
            2,
            t(0.0, 0.39, 0.24, Vec3::new(0.03, 0.025, 0.06)),
        ),
        (
            block.clone(),
            2,
            t(-0.05, 0.07, 0.0, Vec3::new(0.02, 0.15, 0.02)),
        ),
        (
            block.clone(),
            2,
            t(0.05, 0.07, 0.0, Vec3::new(0.02, 0.15, 0.02)),
        ),
    ];
    let wing = meshes.add(wing_mesh());
    let kit = AnimalKit {
        graphs: models,
        scenes,
        hen_parts,
        hen_colors: vec![
            matte(&mut materials, 0.86, 0.82, 0.74),
            matte(&mut materials, 0.5, 0.3, 0.15),
            matte(&mut materials, 0.16, 0.14, 0.13),
        ],
        hen_fixed: vec![
            matte(&mut materials, 0.75, 0.13, 0.1),
            matte(&mut materials, 0.82, 0.58, 0.16),
        ],
        body: sphere,
        wing,
        gull: [
            matte(&mut materials, 0.92, 0.92, 0.9),
            matte(&mut materials, 0.55, 0.57, 0.6),
        ],
        songbird: [
            matte(&mut materials, 0.38, 0.27, 0.18),
            matte(&mut materials, 0.25, 0.3, 0.42),
            matte(&mut materials, 0.62, 0.42, 0.2),
        ],
        fish: matte(&mut materials, 0.35, 0.33, 0.3),
        butterfly: [
            matte(&mut materials, 0.95, 0.6, 0.15),
            matte(&mut materials, 0.95, 0.92, 0.75),
            matte(&mut materials, 0.35, 0.5, 0.95),
            matte(&mut materials, 0.9, 0.35, 0.55),
        ],
    };
    commands.insert_resource(kit);
}

/// Modello pronto: collega il riproduttore e parte dalla clip di riposo.
fn on_grazer_ready(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    kit: Res<AnimalKit>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    mut grazers: Query<&mut Grazer>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
) {
    let root = ready.entity;
    let Some(body) = parents.get(root).ok().map(|p| p.parent()) else {
        return;
    };
    let Ok(mut grazer) = grazers.get_mut(body) else {
        return;
    };
    let (graph, nodes) = &kit.graphs[grazer.kind];
    for e in children.iter_descendants(root) {
        if let Ok((entity, mut player)) = players.get_mut(e) {
            let mut transitions = AnimationTransitions::new();
            transitions
                .play(&mut player, nodes[IDLE], Duration::ZERO)
                .repeat()
                .seek_to(uniform(&mut grazer.rng));
            commands
                .entity(entity)
                .insert((AnimationGraphHandle(graph.clone()), transitions));
            grazer.player = Some(entity);
            grazer.clip = IDLE;
        }
    }
}

fn spawn_flyer(
    commands: &mut Commands,
    kit: &AnimalKit,
    root: impl Bundle,
    material: Handle<StandardMaterial>,
    body: Vec3,
    span: f32,
) {
    commands
        .spawn((SceneNode, Animal, Visibility::Hidden, NotShadowCaster, root))
        .with_children(|p| {
            p.spawn((
                Mesh3d(kit.body.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_scale(body),
                NotShadowCaster,
            ));
            for side in [-1.0, 1.0] {
                p.spawn((
                    Wing { side },
                    Mesh3d(kit.wing.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_scale(Vec3::new(side * span, 1.0, span * 0.55)),
                    NotShadowCaster,
                ));
            }
        });
}

/// Crea tutti gli animali dai dati dell'isola.
fn populate(
    commands: &mut Commands,
    kit: &AnimalKit,
    spec: &IslandSpec,
    field: &TerrainField,
    blocked: &Blocked,
) {
    let mut seed = 0x51ED_270B_u32;
    for herd in &spec.herds {
        let (file, scale, ..) = GRAZERS[herd.kind];
        for _ in 0..herd.count {
            let pos = pick_spot(&mut seed, herd.center, herd.radius, |p| {
                walkable(field, blocked, p)
            });
            let Some(pos) = pos else {
                warn!(
                    "[ANIMALI] nessun posto per un animale «{file}» attorno a {}",
                    herd.center
                );
                continue;
            };
            let yaw = uniform(&mut seed) * TAU;
            commands
                .spawn((
                    SceneNode,
                    Animal,
                    Grazer {
                        kind: herd.kind,
                        home: herd.center,
                        radius: herd.radius,
                        pos,
                        yaw,
                        target: None,
                        wait: uniform(&mut seed) * 8.0,
                        clip: IDLE,
                        player: None,
                        rng: seed.wrapping_add(1) | 1,
                    },
                    Transform::from_xyz(pos.x, field.height_at(pos.x, pos.y), pos.y),
                    Visibility::default(),
                    RigidBody::Kinematic,
                ))
                .with_children(|p| {
                    let (w, l) = if herd.kind == 0 {
                        (0.9, 1.9)
                    } else {
                        (0.75, 1.3)
                    };
                    p.spawn((
                        Collider::cuboid(w, 1.1, l),
                        Transform::from_xyz(0.0, 0.6, 0.0),
                    ));
                    p.spawn((
                        WorldAssetRoot(kit.scenes[herd.kind].clone()),
                        Transform::from_scale(Vec3::splat(scale)),
                    ))
                    .observe(on_grazer_ready);
                });
        }
    }
    for coop in &spec.coops {
        for i in 0..coop.count {
            let pos = coop.center + Vec2::from_angle(i as f32 * 2.4) * coop.radius * 0.5;
            let feathers = kit.hen_colors[i % kit.hen_colors.len()].clone();
            commands
                .spawn((
                    SceneNode,
                    Animal,
                    Hen {
                        home: coop.center,
                        radius: coop.radius,
                        pos,
                        yaw: i as f32,
                        target: None,
                        wait: uniform(&mut seed) * 4.0,
                        rng: seed.wrapping_add(i as u32 * 2 + 1) | 1,
                    },
                    Transform::from_xyz(pos.x, field.height_at(pos.x, pos.y), pos.y),
                    Visibility::Hidden,
                    RigidBody::Kinematic,
                ))
                .with_children(|p| {
                    p.spawn((
                        Collider::cuboid(0.3, 0.5, 0.45),
                        Transform::from_xyz(0.0, 0.28, 0.0),
                    ));
                    for (mesh, color, at) in &kit.hen_parts {
                        let material = match color {
                            0 => feathers.clone(),
                            c => kit.hen_fixed[c - 1].clone(),
                        };
                        p.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material), *at));
                    }
                });
        }
    }
    if let Some((center, count)) = spec.gulls {
        for i in 0..count {
            let k = i as f32;
            spawn_flyer(
                commands,
                kit,
                (
                    Gull {
                        center: center + Vec2::from_angle(k * 2.1) * (2.0 + k),
                        radius: 11.0 + k * 3.0,
                        height: 7.0 + (k * 1.7) % 7.0,
                        speed: 5.5 + (k * 0.7) % 2.0,
                        phase: k * 1.9,
                        lift: 0.0,
                    },
                    Flap {
                        amp: 0.35,
                        rate: 4.5,
                        rest: 0.15,
                    },
                    Transform::from_xyz(center.x, 10.0, center.y),
                ),
                kit.gull[i % 2].clone(),
                Vec3::new(0.09, 0.08, 0.24),
                0.55,
            );
        }
    }
    // Un piccolo stormo al margine di ogni bosco.
    for (f, forest) in spec.forests.iter().enumerate() {
        let edge = forest.center + Vec2::from_angle(f as f32 * 1.7) * forest.radius * 0.8;
        for i in 0..4 {
            let at = edge
                + Vec2::new(
                    hash01(edge, i as f32) * 5.0 - 2.5,
                    hash01(edge, i as f32 + 9.0) * 5.0 - 2.5,
                );
            if !walkable(field, blocked, at) {
                continue;
            }
            let home = at.extend(field.height_at(at.x, at.y) + 0.06).xzy();
            spawn_flyer(
                commands,
                kit,
                (
                    Songbird {
                        home,
                        flight: None,
                        away: 0.0,
                        rng: seed.wrapping_add(f as u32 * 17 + i) | 1,
                    },
                    Flap {
                        amp: 0.0,
                        rate: 42.0,
                        rest: 1.2,
                    },
                    Transform::from_translation(home),
                ),
                kit.songbird[(f + i as usize) % 3].clone(),
                Vec3::new(0.035, 0.035, 0.07),
                0.09,
            );
        }
    }
    if let Some((center, radius, level)) = field.lake() {
        for i in 0..7 {
            let k = i as f32;
            commands
                .spawn((
                    SceneNode,
                    Animal,
                    Fish {
                        center,
                        radius: radius * (0.3 + 0.08 * k),
                        level: level - 0.14 - k * 0.02,
                        speed: 0.35 + k * 0.06,
                        phase: k * 2.4,
                    },
                    Mesh3d(kit.body.clone()),
                    MeshMaterial3d(kit.fish.clone()),
                    Transform::from_xyz(center.x, level - 0.3, center.y),
                    NotShadowCaster,
                ))
                .with_children(|p| {
                    p.spawn((
                        Mesh3d(kit.wing.clone()),
                        MeshMaterial3d(kit.fish.clone()),
                        Transform::from_xyz(0.0, 0.0, -1.0)
                            .with_rotation(Quat::from_rotation_y(PI * 0.5))
                            .with_scale(Vec3::new(0.9, 1.0, 1.1)),
                        NotShadowCaster,
                    ));
                });
        }
    }
    for i in 0..BUTTERFLIES {
        spawn_flyer(
            commands,
            kit,
            (
                Butterfly {
                    home: None,
                    seed: i as f32 * 3.7,
                },
                Flap {
                    amp: 1.0,
                    rate: 22.0,
                    rest: 0.3,
                },
                Transform::default(),
            ),
            kit.butterfly[i % 4].clone(),
            Vec3::new(0.01, 0.01, 0.035),
            0.075,
        );
    }
}

/// Sistema: (ri)crea gli animali quando il mondo è pronto.
#[allow(clippy::too_many_arguments)]
fn sync_animals(
    mut commands: Commands,
    field: Option<Res<TerrainField>>,
    dbg: Res<DebugPlace>,
    village: Res<VillageInfo>,
    kit: Option<Res<AnimalKit>>,
    existing: Query<Entity, With<Animal>>,
    mut built: Local<Option<u32>>,
) {
    let (Some(field), Some(kit)) = (field, kit) else {
        return;
    };
    if village.poly.len() < 3 {
        return;
    }
    let want = dbg.revision();
    if *built == Some(want) && !existing.is_empty() {
        return;
    }
    *built = Some(want);
    for e in &existing {
        commands.entity(e).despawn();
    }
    let spec = crate::water::load_spec();
    let blocked = blocked_map(&dbg, &village, &spec);
    populate(&mut commands, &kit, &spec, &field, &blocked);
    info!(
        "[ANIMALI] {} mandrie, {} pollai, stormi in {} boschi",
        spec.herds.len(),
        spec.coops.len(),
        spec.forests.len()
    );
    commands.insert_resource(Pasture(blocked));
}

/// Sistema: mucche e pecore pascolano, di notte stanno ferme, si scostano se ci si avvicina.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn move_grazers(
    time: Res<Time>,
    clock: Res<GameClock>,
    field: Option<Res<TerrainField>>,
    pasture: Option<Res<Pasture>>,
    kit: Option<Res<AnimalKit>>,
    player: Query<&Transform, (With<Player>, Without<Grazer>)>,
    spatial: SpatialQuery,
    grounds: Query<(), With<crate::scene::Ground>>,
    mut grazers: Query<(Entity, &mut Grazer, &mut Transform)>,
    mut anim: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    let (Some(field), Some(pasture), Some(kit), Ok(me)) = (field, pasture, kit, player.single())
    else {
        return;
    };
    let dt = time.delta_secs();
    let me = me.translation.xz();
    let awake = daylight(clock.hour, clock.sunrise, clock.sunset) > 0.3;
    for (entity, mut g, mut t) in &mut grazers {
        let (_, _, walk, run) = GRAZERS[g.kind];
        let ok = |p: Vec2| walkable(&field, &pasture.0, p);
        let scared = g.pos.distance(me) < SHY;
        if scared && g.clip != RUN {
            let from = g.pos;
            g.target = flee_spot(from, me, ok);
        }
        let mut clip = IDLE;
        if let Some(target) = g.target {
            let to = target - g.pos;
            let running = g.pos.distance(me) < SHY * 2.0 && (scared || g.clip == RUN);
            let speed = if running { run } else { walk };
            clip = if running { RUN } else { WALK };
            if to.length() < 0.3 {
                g.target = None;
                g.wait = 4.0 + uniform(&mut g.rng) * 11.0;
                clip = IDLE;
            } else {
                let step = to.normalize() * speed * dt;
                let next = g.pos + step;
                let from = Vec3::new(g.pos.x, t.translation.y + 0.7, g.pos.y);
                let stuck = blocked_ahead(&spatial, &grounds, entity, from, step, 1.3);
                if ok(next) && !stuck {
                    g.pos = next;
                    let yaw = g.yaw;
                    g.yaw = turn_to(yaw, to.x.atan2(to.y), dt * 2.2);
                } else {
                    g.target = None;
                    g.wait = 2.0;
                    clip = IDLE;
                }
            }
        } else {
            g.wait -= dt;
            if g.wait <= 0.0 && awake {
                let (home, radius) = (g.home, g.radius);
                g.target = pick_spot(&mut g.rng, home, radius, ok);
                g.wait = 3.0;
            }
        }
        t.translation = Vec3::new(g.pos.x, field.height_at(g.pos.x, g.pos.y), g.pos.y);
        t.rotation = Quat::from_rotation_y(g.yaw);
        if clip != g.clip
            && let Some(entity) = g.player
            && let Ok((mut p, mut tr)) = anim.get_mut(entity)
        {
            tr.play(
                &mut p,
                kit.graphs[g.kind].1[clip],
                Duration::from_millis(250),
            )
            .repeat();
            g.clip = clip;
        }
    }
}

/// Sistema: galline in giro di giorno, al chiuso di notte.
#[allow(clippy::type_complexity)]
fn move_hens(
    time: Res<Time>,
    clock: Res<GameClock>,
    field: Option<Res<TerrainField>>,
    player: Query<&Transform, (With<Player>, Without<Hen>)>,
    spatial: SpatialQuery,
    grounds: Query<(), With<crate::scene::Ground>>,
    mut hens: Query<(Entity, &mut Hen, &mut Transform, &mut Visibility)>,
) {
    let (Some(field), Ok(me)) = (field, player.single()) else {
        return;
    };
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let me = me.translation.xz();
    let out = daylight(clock.hour, clock.sunrise, clock.sunset) > 0.25;
    for (entity, mut h, mut t, mut vis) in &mut hens {
        vis.set_if_neq(if out {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if !out {
            continue;
        }
        let mut peck = 0.0;
        if h.pos.distance(me) < 1.4 {
            let (home, radius, pos) = (h.home, h.radius, h.pos);
            h.target = flee_spot(pos, me, |p| p.distance(home) <= radius);
        }
        if let Some(target) = h.target {
            let to = target - h.pos;
            if to.length() < 0.1 {
                h.target = None;
                h.wait = 1.0 + uniform(&mut h.rng) * 5.0;
            } else {
                let step = to.normalize() * 0.55 * dt;
                let from = Vec3::new(h.pos.x, t.translation.y + 0.3, h.pos.y);
                if blocked_ahead(&spatial, &grounds, entity, from, step, 0.55) {
                    h.target = None;
                    h.wait = 0.5;
                } else {
                    h.pos += step;
                    h.yaw = turn_to(h.yaw, to.x.atan2(to.y), dt * 6.0);
                }
            }
        } else {
            peck = ((now * 7.0 + h.home.x).sin().max(0.0)) * 0.55;
            h.wait -= dt;
            if h.wait <= 0.0 {
                let (home, radius) = (h.home, h.radius);
                h.target = pick_spot(&mut h.rng, home, radius, |_| true);
                h.wait = 2.0;
            }
        }
        t.translation = Vec3::new(h.pos.x, field.height_at(h.pos.x, h.pos.y), h.pos.y);
        t.rotation = Quat::from_rotation_y(h.yaw) * Quat::from_rotation_x(peck);
    }
}

/// Sistema: gabbiani che girano sul porto, di giorno e senza tempesta.
fn move_gulls(
    time: Res<Time>,
    clock: Res<GameClock>,
    weather: Res<WeatherState>,
    spatial: SpatialQuery,
    grounds: Query<(), With<crate::scene::Ground>>,
    mut gulls: Query<(&mut Gull, &mut Transform, &mut Visibility)>,
) {
    let now = time.elapsed_secs();
    let out = daylight(clock.hour, clock.sunrise, clock.sunset) > 0.2 && weather.rain < 0.6;
    for (mut g, mut t, mut vis) in &mut gulls {
        vis.set_if_neq(if out {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let a = now * g.speed / g.radius + g.phase;
        let r = g.radius * (1.0 + 0.25 * (a * 0.37).sin());
        let (x, z) = (g.center.x + r * a.cos(), g.center.y + r * a.sin());
        // Ciò che sta sotto (faro, tetti, torri): il gabbiano sale per passarci sopra.
        let ceiling = spatial
            .cast_ray_predicate(
                Vec3::new(x, 60.0, z),
                Dir3::NEG_Y,
                70.0,
                true,
                &SpatialQueryFilter::default(),
                &|e| !grounds.contains(e),
            )
            .map_or(f32::MIN, |h| 60.0 - h.distance);
        let want = (ceiling + 3.0 - g.height).max(0.0);
        let k = 1.0 - (-time.delta_secs() * 1.2).exp();
        g.lift += (want - g.lift) * k;
        t.translation = Vec3::new(x, g.height + g.lift + (a * 0.8).sin() * 1.3, z);
        // Tangente all'orbita e ali inclinate nella virata.
        t.rotation = Quat::from_rotation_y(-a + PI) * Quat::from_rotation_z(0.28);
    }
}

/// Sistema: uccellini che zampettano e si alzano in volo se il giocatore si avvicina.
#[allow(clippy::type_complexity)]
fn move_songbirds(
    time: Res<Time>,
    clock: Res<GameClock>,
    player: Query<&Transform, (With<Player>, Without<Songbird>)>,
    mut birds: Query<(&mut Songbird, &mut Flap, &mut Transform, &mut Visibility)>,
) {
    let Ok(me) = player.single() else {
        return;
    };
    let dt = time.delta_secs();
    let day = daylight(clock.hour, clock.sunrise, clock.sunset) > 0.2;
    for (mut b, mut flap, mut t, mut vis) in &mut birds {
        if b.away > 0.0 {
            b.away -= dt;
            vis.set_if_neq(Visibility::Hidden);
            if b.away <= 0.0 {
                b.flight = None;
                t.translation = b.home;
            }
            continue;
        }
        vis.set_if_neq(if day {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        match b.flight {
            Some(dir) => {
                t.translation += dir * dt;
                flap.amp = 0.9;
                if t.translation.y > b.home.y + 22.0 {
                    b.away = 25.0 + uniform(&mut b.rng) * 40.0;
                }
            }
            None if day && t.translation.distance(me.translation) < SCARE => {
                let away = (t.translation - me.translation).xz().normalize_or(Vec2::X);
                let turn = uniform(&mut b.rng) - 0.5;
                let side = Vec2::from_angle(turn).rotate(away) * 6.0;
                let dir = Vec3::new(side.x, 5.0 + uniform(&mut b.rng) * 2.0, side.y);
                b.flight = Some(dir);
                t.rotation = Quat::from_rotation_y(dir.x.atan2(dir.z));
            }
            None => {
                flap.amp = 0.0;
                // Ogni tanto un saltello sul posto.
                let hop = (time.elapsed_secs() * 1.3 + b.home.x * 3.0).sin();
                t.translation.y = b.home.y + (hop - 0.93).max(0.0) * 0.8;
            }
        }
    }
}

/// Sistema: pesci che girano nel laghetto muovendo la coda.
fn move_fish(time: Res<Time>, mut fish: Query<(&Fish, &mut Transform)>) {
    let now = time.elapsed_secs();
    for (f, mut t) in &mut fish {
        let a = now * f.speed + f.phase;
        let r = f.radius * (1.0 + 0.2 * (a * 0.6).sin());
        t.translation = Vec3::new(f.center.x + r * a.cos(), f.level, f.center.y + r * a.sin());
        let wiggle = (now * 9.0 + f.phase).sin() * 0.12;
        t.rotation = Quat::from_rotation_y(-a + PI + wiggle);
        t.scale = Vec3::new(0.09, 0.08, 0.26);
    }
}

/// Sistema: farfalle attorno al giocatore quando ci sono i fiori, di giorno e col bello.
#[allow(clippy::type_complexity)]
fn move_butterflies(
    time: Res<Time>,
    clock: Res<GameClock>,
    weather: Res<WeatherState>,
    season: Res<SeasonLook>,
    field: Option<Res<TerrainField>>,
    player: Query<&Transform, (With<Player>, Without<Butterfly>)>,
    mut flies: Query<(&mut Butterfly, &mut Transform, &mut Visibility)>,
) {
    let (Some(field), Ok(me)) = (field, player.single()) else {
        return;
    };
    let now = time.elapsed_secs();
    let on = season.0.flowers > 0.3
        && daylight(clock.hour, clock.sunrise, clock.sunset) > 0.6
        && weather.rain < 0.05
        && weather.wind < 7.0
        && weather.sheltered < 0.5;
    let center = me.translation.xz();
    for (mut b, mut t, mut vis) in &mut flies {
        if !on {
            vis.set_if_neq(Visibility::Hidden);
            b.home = None;
            continue;
        }
        let far = b
            .home
            .is_none_or(|h| h.xz().distance(center) > BUTTERFLY_ROAM);
        if far {
            let k = now.floor() * 0.17 + b.seed;
            let p = center
                + Vec2::from_angle(k * 2.3)
                    * (BUTTERFLY_ROAM * 0.35 + (k * 7.1).sin().abs() * 12.0);
            if field.surface_at(p.x, p.y) != crate::terrain::Surface::Grass {
                vis.set_if_neq(Visibility::Hidden);
                b.home = None;
                continue;
            }
            b.home = Some(Vec3::new(p.x, field.height_at(p.x, p.y) + 0.7, p.y));
        }
        vis.set_if_neq(Visibility::Inherited);
        let Some(home) = b.home else {
            continue;
        };
        let s = b.seed;
        let offset = Vec3::new(
            (now * 0.7 + s).sin() + (now * 1.9 + s * 2.0).sin() * 0.35,
            (now * 2.3 + s).sin() * 0.3,
            (now * 0.9 + s * 3.0).cos() + (now * 1.5 + s).cos() * 0.35,
        ) * 1.3;
        t.translation = home + offset;
        t.rotation = Quat::from_rotation_y(now * 0.8 + s);
    }
}

/// Sistema: battito d'ali.
fn flap_wings(
    time: Res<Time>,
    flaps: Query<&Flap>,
    mut wings: Query<(&Wing, &ChildOf, &mut Transform)>,
) {
    let now = time.elapsed_secs();
    for (wing, parent, mut t) in &mut wings {
        let Ok(f) = flaps.get(parent.parent()) else {
            continue;
        };
        let angle = f.rest + f.amp * (now * f.rate + parent.parent().index_u32() as f32).sin();
        t.rotation = Quat::from_rotation_z(wing.side * angle);
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup).add_systems(
        Update,
        (
            sync_animals,
            move_grazers,
            move_hens,
            move_gulls,
            move_songbirds,
            move_fish,
            move_butterflies,
            flap_wings,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_are_in_the_models_in_order() {
        for (file, ..) in GRAZERS {
            let path = format!(
                "{}/assets/models/animals/{file}.glb",
                env!("CARGO_MANIFEST_DIR")
            );
            let data = std::fs::read(path).unwrap();
            let len = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
            let json: serde_json::Value = serde_json::from_slice(&data[20..20 + len]).unwrap();
            let names: Vec<&str> = json["animations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a["name"].as_str().unwrap())
                .collect();
            assert_eq!(names, CLIPS, "{file}");
        }
    }

    #[test]
    fn turning_takes_the_short_way() {
        assert!((turn_to(0.1, TAU - 0.1, 1.0) - (-0.1)).abs() < 1e-4);
        assert!((turn_to(0.0, 3.0, 0.5) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn pick_spot_stays_home_and_gives_up_when_nothing_fits() {
        let mut rng = 7u32;
        let home = Vec2::new(3.0, -2.0);
        for _ in 0..50 {
            let p = pick_spot(&mut rng, home, 4.0, |_| true).unwrap();
            assert!(p.distance(home) <= 4.0 + 1e-3);
        }
        assert!(pick_spot(&mut rng, home, 4.0, |_| false).is_none());
    }

    #[test]
    fn fleeing_moves_away_from_the_player() {
        let spot = flee_spot(Vec2::ZERO, Vec2::new(0.0, 2.0), |_| true).unwrap();
        assert!(spot.y < -5.0);
        assert!(flee_spot(Vec2::ZERO, Vec2::new(0.0, 2.0), |_| false).is_none());
    }
}
