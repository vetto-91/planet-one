//! Le scene delle missioni secondarie che hanno bisogno di persone sul posto (ADR 0052).
//!
//! - **QS-09**: Dino aspetta fuori da Porta Sud per un duello amichevole (tre colpi a segno, senza
//!   ferirlo); poi, al tramonto, due predoni scendono sulla strada del Sud e Aldo guarda.
//! - **QS-14**: dopo la prova dell'arco Silvia aspetta all'imbocco della grotta a est.
//! - **QS-15**: se il cantiere è compiuto, prima dell'alba della partenza il paese scende al molo.
//!
//! Le persone si spostano con il posto «chiamato dalla storia» della routine (`Villager::call_to`):
//! ogni sistema lo rimette a ogni frame finché la scena vale, e lo toglie quando finisce.

use crate::quests::missions::Missions;
use crate::region::Region;
use crate::villagers::Villager;
use avian3d::prelude::*;
use bevy::prelude::*;

/// QS-09: il posto del duello, fuori da Porta Sud (Dino guarda verso la porta).
pub const DUEL_SPOT: (Vec2, Vec2) = (Vec2::new(0.4, 46.5), Vec2::new(0.0, -1.0));
/// Colpi a segno che chiudono il duello.
pub const DUEL_HITS: u8 = 3;
/// QS-09: Dino e Aldo sulla strada del Sud, girati verso il bosco; da dove escono i predoni.
const ROAD_DINO: (Vec2, Vec2) = (Vec2::new(1.4, 49.0), Vec2::new(0.0, 1.0));
const ROAD_ALDO: (Vec2, Vec2) = (Vec2::new(-1.8, 47.5), Vec2::new(0.0, 1.0));
const ROAD_RAIDERS: [(Vec2, Vec2); 2] = [
    (Vec2::new(-7.0, 68.0), Vec2::new(-0.5, 52.0)),
    (Vec2::new(8.0, 68.0), Vec2::new(1.5, 52.0)),
];
/// Fin dove si deve essere perché la difesa cominci.
const ROAD_RANGE: f32 = 30.0;
/// QS-14: Silvia all'imbocco della grotta, girata verso l'arco d'ingresso.
pub const SILVIA_AT_CAVE: (Vec2, Vec2) = (Vec2::new(90.5, 3.2), Vec2::new(0.0, 1.0));
/// QS-15: il paese sul molo, dietro Gaspare, girato verso il mare.
const PIER_CROWD: Vec2 = Vec2::new(8.5, 97.5);

/// Rimette il posto della storia di `name` (o lo toglie, ma solo se è uno dei posti di questa
/// scena, `mine`: quelli dati da altre scene, come il saluto al molo, restano).
fn call(
    villagers: &mut Query<(Entity, &mut Villager)>,
    name: &str,
    spot: Option<(Vec2, Vec2)>,
    mine: &[(Vec2, Vec2)],
) {
    if let Some((_, mut v)) = villagers.iter_mut().find(|(_, v)| v.name() == name)
        && v.called() != spot
        && (spot.is_some() || v.called().is_some_and(|c| mine.contains(&c)))
    {
        v.call_to(spot);
    }
}

/// Uno dei predoni della strada del Sud.
#[derive(Component)]
pub struct SouthRaider;

/// Stato delle scene di Dino (duello e difesa).
#[derive(Resource, Default)]
pub struct DinoScenes {
    /// Colpi a segno nel duello.
    pub hits: u8,
    /// 0 niente, 1 predoni usciti, 2 visti in campo, 3 a terra.
    fight: u8,
    /// Il fendente del frame prima (per contare un colpo per fendente).
    swinging: bool,
    /// Dino ha già detto «in guardia».
    greeted: bool,
    /// Nella difesa, dove Dino e Aldo vanno incontro al predone più vicino.
    dino_chase: Option<(Vec2, Vec2)>,
    aldo_chase: Option<(Vec2, Vec2)>,
    /// Secondi prima del prossimo colpo di Dino e di Aldo.
    dino_cd: f32,
    aldo_cd: f32,
}

/// Fin dove la guardia va incontro a un predone, e a che distanza gli si ferma davanti.
const GUARD_CHASE: f32 = 14.0;
const GUARD_STAND: f32 = 1.5;

/// Dove si mette una guardia che difende `post`: davanti al predone vivo più vicino (entro
/// `GUARD_CHASE`), girata verso di lui; senza predoni, al suo posto. Non si sposta per meno di un
/// metro e mezzo, per non cambiare strada a ogni passo del predone.
pub fn guard_spot(
    post: (Vec2, Vec2),
    raiders: &[Vec2],
    previous: Option<(Vec2, Vec2)>,
) -> (Vec2, Vec2) {
    let Some(r) = raiders
        .iter()
        .filter(|r| r.distance(post.0) <= GUARD_CHASE)
        .min_by(|a, b| a.distance(post.0).total_cmp(&b.distance(post.0)))
    else {
        return post;
    };
    let dir = (*r - post.0).normalize_or(post.1);
    let spot = (*r - dir * GUARD_STAND, dir);
    match previous {
        Some(p) if p.0.distance(spot.0) < 1.5 => p,
        _ => spot,
    }
}

/// Sistema: QS-09, il duello con Dino e la difesa della strada del Sud.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn dino_scenes(
    mut commands: Commands,
    (missions, clock, time): (Res<Missions>, Res<crate::daynight::GameClock>, Res<Time>),
    karma: Res<crate::karma::Karma>,
    swing: Res<crate::combat::SwingTimer>,
    equip: Res<crate::inventory::Equipment>,
    server: Res<AssetServer>,
    field: Option<Res<crate::terrain::TerrainField>>,
    mut state: ResMut<DinoScenes>,
    mut villagers: Query<(Entity, &mut Villager)>,
    places: Query<&GlobalTransform>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    visuals: Query<&GlobalTransform, With<crate::player::PlayerVisual>>,
    mut raiders: Query<
        (Entity, &GlobalTransform, &mut crate::combat::EnemyHealth),
        With<SouthRaider>,
    >,
    mut barks: ResMut<crate::quests::Barks>,
    mut session: ResMut<crate::save::Session>,
) {
    let island = crate::region::current() == Region::Isola && missions.enabled;
    let step = if island {
        missions.step_of("qs09")
    } else {
        None
    };
    let duel = step == Some(3) && !karma.unlocks.contains("duello-dino");
    let dusk = (18.0..22.5).contains(&clock.hour);
    let defense = step == Some(4) && dusk && !karma.unlocks.contains("predoni-sud-vinti");
    // Nella difesa Dino e Aldo vanno incontro ai predoni vivi.
    let live: Vec<Vec2> = raiders
        .iter()
        .filter(|(_, _, hp)| hp.current > 0.0)
        .map(|(_, t, _)| t.translation().xz())
        .collect();
    let old = (state.dino_chase, state.aldo_chase);
    state.dino_chase = defense.then(|| guard_spot(ROAD_DINO, &live, state.dino_chase));
    state.aldo_chase = defense.then(|| guard_spot(ROAD_ALDO, &live, state.aldo_chase));
    let dino_spot = if duel {
        Some(DUEL_SPOT)
    } else {
        state.dino_chase
    };
    let mut dino_mine = vec![DUEL_SPOT, ROAD_DINO];
    dino_mine.extend(old.0);
    call(&mut villagers, "Guardia Dino", dino_spot, &dino_mine);
    let mut aldo_mine = vec![ROAD_ALDO];
    aldo_mine.extend(old.1);
    let aldo_spot = state
        .aldo_chase
        .or((step == Some(4) && dusk || step == Some(5)).then_some(ROAD_ALDO));
    call(&mut villagers, "Guardia Aldo", aldo_spot, &aldo_mine);
    // I loro colpi: come quelli di un compagno (`companions::strike`), al predone più vicino a portata.
    let dt = time.delta_secs();
    state.dino_cd = (state.dino_cd - dt).max(0.0);
    state.aldo_cd = (state.aldo_cd - dt).max(0.0);
    if defense {
        for who in ["Guardia Dino", "Guardia Aldo"] {
            let ready = if who == "Guardia Dino" {
                state.dino_cd == 0.0
            } else {
                state.aldo_cd == 0.0
            };
            let Some(at) = villagers
                .iter()
                .find(|(_, v)| v.name() == who)
                .and_then(|(e, _)| places.get(e).ok().map(|t| t.translation()))
            else {
                continue;
            };
            let (damage, every, reach) = crate::companions::strike(who);
            if !ready {
                continue;
            }
            if let Some((_, _, mut hp)) = raiders
                .iter_mut()
                .filter(|(_, t, hp)| hp.current > 0.0 && t.translation().distance(at) <= reach)
                .min_by(|a, b| {
                    a.1.translation()
                        .distance(at)
                        .total_cmp(&b.1.translation().distance(at))
                })
            {
                hp.current = (hp.current - damage).max(0.0);
                info!("[SCENE] {who} colpisce un predone: vita {:.0}", hp.current);
                if who == "Guardia Dino" {
                    state.dino_cd = every;
                } else {
                    state.aldo_cd = every;
                }
            }
        }
    }
    let Ok(player) = players.single() else {
        return;
    };
    let me = player.translation();
    // Dino conta per il duello quando è arrivato (o quasi) al suo posto fuori dalla porta.
    let dino = villagers
        .iter()
        .find(|(_, v)| v.name() == "Guardia Dino")
        .and_then(|(e, _)| places.get(e).ok().map(|t| (e, t.translation())))
        .filter(|(_, at)| at.xz().distance(DUEL_SPOT.0) < 3.0);

    // Il duello: ogni fendente in mischia che arriva a Dino, davanti a te, è un colpo a segno.
    let swinging = swing.0 > 0.0;
    let new_swing = swinging && !state.swinging;
    state.swinging = swinging;
    if duel && let Some((e, at)) = dino {
        if !state.greeted
            && me.distance(at) < 6.0
            && let Some(line) = crate::quests::bark_line("Guardia Dino", "duello-inizio")
        {
            state.greeted = true;
            barks.say(e, "Guardia Dino", line.to_string());
        }
        let stats = crate::inventory::weapon_stats(equip.weapon);
        let facing = visuals.single().map_or(*player.forward(), |v| *v.forward());
        let to = at - me;
        let reach = to.with_y(0.0).length() <= stats.range + 0.4
            && to.with_y(0.0).normalize_or_zero().dot(facing.with_y(0.0)) > 0.3;
        if new_swing && !stats.ranged && reach {
            state.hits += 1;
            crate::skills::train(crate::skills::Skill::Blade, 4.0);
            info!("[SCENE] duello con Dino: colpo {}", state.hits);
            let key = if state.hits >= DUEL_HITS {
                "duello-fine"
            } else {
                "duello-colpo"
            };
            if let Some(line) = crate::quests::bark_line("Guardia Dino", key) {
                barks.say(e, "Guardia Dino", line.to_string());
            }
            if state.hits >= DUEL_HITS {
                crate::karma::flag("duello-dino");
                session.notify("Duello vinto: Dino ha capito dove sbaglia");
            }
        }
    } else if !duel {
        state.hits = 0;
        state.greeted = false;
    }

    // La difesa: arrivati sulla strada al tramonto, escono due predoni; a terra tutti e due, è fatta.
    let Some(field) = field else {
        return;
    };
    if !defense {
        if state.fight > 0 && state.fight < 3 {
            for (e, ..) in &raiders {
                commands.entity(e).despawn();
            }
        }
        state.fight = 0;
        return;
    }
    match state.fight {
        0 if me.xz().distance(ROAD_DINO.0) <= ROAD_RANGE => {
            for (from, to) in ROAD_RAIDERS {
                let start = Vec3::new(from.x, field.height_at(from.x, from.y) + 1.0, from.y);
                let end = Vec3::new(to.x, field.height_at(to.x, to.y) + 1.0, to.y);
                let e = crate::entities::spawn_enemy(&mut commands, &server, start, end);
                commands
                    .entity(e)
                    .insert((SouthRaider, crate::combat::NoRespawn));
            }
            for who in ["Guardia Dino", "Guardia Aldo"] {
                if let Some((e, _)) = villagers.iter().find(|(_, v)| v.name() == who)
                    && let Some(line) = crate::quests::bark_line(who, "difesa")
                {
                    barks.say(e, who, line.to_string());
                }
            }
            session.notify("Dal bosco scendono due predoni!");
            info!("[SCENE] strada del Sud: arrivano i predoni");
            state.fight = 1;
        }
        1 if !raiders.is_empty() => state.fight = 2,
        2 if raiders.iter().all(|(.., hp)| hp.current <= 0.0) => {
            crate::karma::flag("predoni-sud-vinti");
            info!("[SCENE] strada del Sud: i predoni sono a terra");
            state.fight = 3;
        }
        _ => {}
    }
}

/// QS-01: il mulino in fondo alla strada nord, e da dove escono i due predoni del bosco.
pub const MILL: Vec2 = Vec2::new(0.5, -66.0);
const MILL_RAIDERS: [(Vec2, Vec2); 2] = [
    (Vec2::new(-14.0, -74.0), Vec2::new(-3.0, -68.0)),
    (Vec2::new(13.0, -76.0), Vec2::new(4.0, -69.0)),
];
const MILL_RANGE: f32 = 40.0;

/// Uno dei predoni del bosco attorno al mulino.
#[derive(Component)]
pub struct MillRaider;

/// Sistema: QS-01, i due predoni che tengono Gino chiuso nel mulino. Escono quando arrivi; a terra
/// tutti e due, `predoni-mulino-vinti`. Se la tappa cambia prima, se ne vanno.
#[allow(clippy::too_many_arguments)]
fn mill_raiders(
    mut commands: Commands,
    missions: Res<Missions>,
    karma: Res<crate::karma::Karma>,
    server: Res<AssetServer>,
    field: Option<Res<crate::terrain::TerrainField>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    raiders: Query<(Entity, &crate::combat::EnemyHealth), With<MillRaider>>,
    mut session: ResMut<crate::save::Session>,
    mut fight: Local<u8>,
) {
    let (Some(field), Ok(player)) = (field, players.single()) else {
        return;
    };
    let active = crate::region::current() == Region::Isola
        && missions.enabled
        && missions.step_of("qs01") == Some(2)
        && !karma.unlocks.contains("predoni-mulino-vinti");
    if !active {
        if *fight > 0 && *fight < 3 {
            for (e, _) in &raiders {
                commands.entity(e).despawn();
            }
        }
        *fight = 0;
        return;
    }
    match *fight {
        0 if player.translation().xz().distance(MILL) <= MILL_RANGE => {
            for (from, to) in MILL_RAIDERS {
                let start = Vec3::new(from.x, field.height_at(from.x, from.y) + 1.0, from.y);
                let end = Vec3::new(to.x, field.height_at(to.x, to.y) + 1.0, to.y);
                let e = crate::entities::spawn_enemy(&mut commands, &server, start, end);
                commands
                    .entity(e)
                    .insert((MillRaider, crate::combat::NoRespawn));
            }
            session.notify("Dal bosco attorno al mulino escono due predoni!");
            info!("[SCENE] mulino: arrivano i predoni");
            *fight = 1;
        }
        1 if !raiders.is_empty() => *fight = 2,
        2 if raiders.iter().all(|(_, hp)| hp.current <= 0.0) => {
            crate::karma::flag("predoni-mulino-vinti");
            info!("[SCENE] mulino: i predoni sono a terra");
            *fight = 3;
        }
        _ => {}
    }
}

/// QS-06: la cena davanti alla locanda di Osvaldo, e chi ci viene.
const DINNER: Vec2 = Vec2::new(20.0, 10.6);
const DINNER_GUESTS: [&str; 5] = [
    "Guardia Carlo",
    "Guardia Bruno",
    "Beppe",
    "Gaspare",
    "Marta",
];

/// La tavola della cena, tra Osvaldo e gli invitati.
const DINNER_TABLE: Vec2 = Vec2::new(20.0, 9.4);

/// Chi è a cena da Osvaldo.
#[derive(Resource, Default)]
pub struct Dinner {
    pub guests: Vec<Entity>,
}

/// La tavola apparecchiata della cena dell'anfora (sparisce a cena finita).
#[derive(Component)]
pub struct DinnerTable;

/// Apparecchia la tavola: piano e gambe di legno, l'anfora, il pane di Marta, il pesce di Nina, le
/// mele di Tonio, due brocche e una lanterna accesa.
fn lay_the_table(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    wood: Handle<StandardMaterial>,
    ground: f32,
) {
    let mut paint = |c: Color| {
        materials.add(StandardMaterial {
            base_color: c,
            perceptual_roughness: 0.7,
            ..default()
        })
    };
    let clay = paint(Color::srgb(0.66, 0.38, 0.24));
    let bread = paint(Color::srgb(0.78, 0.55, 0.28));
    let fish = paint(Color::srgb(0.55, 0.62, 0.68));
    let apple = paint(Color::srgb(0.75, 0.12, 0.1));
    let flame = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.8, 0.45),
        emissive: LinearRgba::rgb(8.0, 4.5, 1.4),
        ..default()
    });
    let top = 0.78;
    commands
        .spawn((
            DinnerTable,
            crate::scene::SceneNode,
            Transform::from_xyz(DINNER_TABLE.x, ground, DINNER_TABLE.y),
            Visibility::default(),
            RigidBody::Static,
            Collider::cuboid(3.2, 1.5, 0.9),
        ))
        .with_children(|t| {
            t.spawn((
                Mesh3d(meshes.add(Cuboid::new(3.2, 0.08, 0.9))),
                MeshMaterial3d(wood.clone()),
                Transform::from_xyz(0.0, top, 0.0),
            ));
            for (x, z) in [(-1.45, -0.35), (1.45, -0.35), (-1.45, 0.35), (1.45, 0.35)] {
                t.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.08, top, 0.08))),
                    MeshMaterial3d(wood.clone()),
                    Transform::from_xyz(x, top / 2.0, z),
                ));
            }
            // L'anfora aperta, al centro.
            t.spawn((
                Mesh3d(meshes.add(Capsule3d::new(0.17, 0.3))),
                MeshMaterial3d(clay.clone()),
                Transform::from_xyz(0.0, top + 0.36, 0.0),
            ));
            t.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.07, 0.18))),
                MeshMaterial3d(clay.clone()),
                Transform::from_xyz(0.0, top + 0.72, 0.0),
            ));
            for x in [-1.1f32, -0.55] {
                t.spawn((
                    Mesh3d(meshes.add(Sphere::new(0.14))),
                    MeshMaterial3d(bread.clone()),
                    Transform::from_xyz(x, top + 0.1, 0.1).with_scale(Vec3::new(1.4, 0.6, 1.0)),
                ));
            }
            for z in [-0.18f32, 0.18] {
                t.spawn((
                    Mesh3d(meshes.add(Capsule3d::new(0.06, 0.3))),
                    MeshMaterial3d(fish.clone()),
                    Transform::from_xyz(0.7, top + 0.07, z)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                ));
            }
            for (x, z) in [(1.2, 0.05), (1.32, -0.12), (1.1, -0.16)] {
                t.spawn((
                    Mesh3d(meshes.add(Sphere::new(0.06))),
                    MeshMaterial3d(apple.clone()),
                    Transform::from_xyz(x, top + 0.07, z),
                ));
            }
            for x in [-0.3f32, 0.35] {
                t.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.06, 0.2))),
                    MeshMaterial3d(clay.clone()),
                    Transform::from_xyz(x, top + 0.14, -0.28),
                ));
            }
            t.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.14, 0.2, 0.14))),
                MeshMaterial3d(flame),
                Transform::from_xyz(-1.4, top + 0.14, -0.3),
                PointLight {
                    color: Color::srgb(1.0, 0.72, 0.4),
                    intensity: 35_000.0,
                    range: 7.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
            ));
        });
}

/// Mesh, materiali e materiali PBR per apparecchiare la tavola.
type TableAssets<'w> = (
    ResMut<'w, Assets<Mesh>>,
    ResMut<'w, Assets<StandardMaterial>>,
    Option<Res<'w, crate::pbr::PbrMaterials>>,
);

/// Sistema: QS-06, la sera dell'anfora (dalle 19:30, alla tappa della cena) gli invitati e Marta
/// col pane si mettono davanti alla locanda, girati verso Osvaldo.
#[allow(clippy::too_many_arguments)]
fn dinner_at_the_inn(
    mut commands: Commands,
    (mut meshes, mut materials, pbr): TableAssets,
    tables: Query<Entity, With<DinnerTable>>,
    missions: Res<Missions>,
    clock: Res<crate::daynight::GameClock>,
    field: Option<Res<crate::terrain::TerrainField>>,
    spatial: SpatialQuery,
    grounds: Query<(), With<crate::scene::Ground>>,
    movers: Query<(), crate::lanterns::Movers>,
    mut villagers: Query<(Entity, &mut Villager)>,
    mut dinner: ResMut<Dinner>,
) {
    let Some(field) = field else {
        return;
    };
    let want = crate::region::current() == Region::Isola
        && missions.enabled
        && missions.step_of("qs06") == Some(4)
        && (19.5..24.0).contains(&clock.hour);
    // La tavola c'è finché dura la cena.
    if want
        && tables.is_empty()
        && let Some(pbr) = &pbr
    {
        let ground = field.height_at(DINNER_TABLE.x, DINNER_TABLE.y);
        lay_the_table(
            &mut commands,
            &mut meshes,
            &mut materials,
            pbr.get(crate::pbr::Mat::Planks),
            ground,
        );
    } else if !want {
        for e in &tables {
            commands.entity(e).despawn();
        }
    }
    if want && dinner.guests.is_empty() {
        let who: Vec<Entity> = DINNER_GUESTS
            .iter()
            .filter_map(|name| {
                villagers
                    .iter()
                    .find(|(_, v)| v.name() == *name && v.called().is_none())
                    .map(|(e, _)| e)
            })
            .collect();
        let spots = crowd_spots(DINNER, Vec2::new(0.0, -1.0), who.len(), 3, 1.2, |p| {
            crate::lanterns::spot_is_free(&spatial, &field, &grounds, &movers, p, 0.4)
        });
        for (e, spot) in who.iter().zip(&spots) {
            if let Ok((_, mut v)) = villagers.get_mut(*e) {
                v.call_to(Some(*spot));
                dinner.guests.push(*e);
            }
        }
        info!(
            "[SCENE] cena alla locanda: {} invitati",
            dinner.guests.len()
        );
    } else if !want && !dinner.guests.is_empty() {
        for e in std::mem::take(&mut dinner.guests) {
            if let Ok((_, mut v)) = villagers.get_mut(e) {
                v.call_to(None);
            }
        }
        info!("[SCENE] la cena è finita");
    }
}

/// Sistema: QS-14, Silvia all'imbocco della grotta finché il bracciale non torna a lei.
fn silvia_at_cave(missions: Res<Missions>, mut villagers: Query<(Entity, &mut Villager)>) {
    let island = crate::region::current() == Region::Isola && missions.enabled;
    let waiting = island && matches!(missions.step_of("qs14"), Some(4..=6));
    call(
        &mut villagers,
        "Silvia",
        waiting.then_some(SILVIA_AT_CAVE),
        &[SILVIA_AT_CAVE],
    );
}

/// File di posti dietro `front`, girati come `facing`: righe di `per_row`, a `gap` metri.
pub fn crowd_spots(
    front: Vec2,
    facing: Vec2,
    wanted: usize,
    per_row: usize,
    gap: f32,
    free: impl Fn(Vec2) -> bool,
) -> Vec<(Vec2, Vec2)> {
    let facing = facing.normalize_or(Vec2::Y);
    let side = Vec2::new(facing.y, -facing.x);
    let mut out = Vec::new();
    for row in 0..12 {
        for k in 0..per_row {
            if out.len() >= wanted {
                return out;
            }
            // Sfalsati di mezzo passo a righe alterne, dal centro verso i lati.
            let lateral = (k as f32 - (per_row as f32 - 1.0) / 2.0 + 0.5 * (row % 2) as f32) * gap;
            let p = front - facing * (row as f32 * gap) + side * lateral;
            if free(p)
                && out
                    .iter()
                    .all(|(q, _): &(Vec2, Vec2)| q.distance(p) >= gap * 0.9)
            {
                out.push((p, facing));
            }
        }
    }
    out
}

/// Chi è sceso al molo a salutare.
#[derive(Resource, Default)]
pub struct Farewell {
    pub crowd: Vec<Entity>,
}

/// Sistema: QS-15, prima dell'alba della partenza il paese scende al molo dietro Gaspare (se il
/// cantiere è compiuto). Restano al loro posto Gaspare (è già al molo) ed Ettore (tiene il faro).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn farewell_at_the_pier(
    missions: Res<Missions>,
    clock: Res<crate::daynight::GameClock>,
    field: Option<Res<crate::terrain::TerrainField>>,
    spatial: SpatialQuery,
    grounds: Query<(), With<crate::scene::Ground>>,
    movers: Query<(), crate::lanterns::Movers>,
    mut villagers: Query<(Entity, &mut Villager)>,
    mut farewell: ResMut<Farewell>,
) {
    let Some(field) = field else {
        return;
    };
    let want = crate::region::current() == Region::Isola
        && missions.enabled
        && missions.done("qs15")
        && missions.step_of("mp05") == Some(2)
        && (3.0..9.5).contains(&clock.hour);
    if want && farewell.crowd.is_empty() {
        let mut who: Vec<(Entity, String)> = villagers
            .iter()
            .filter(|(_, v)| {
                v.called().is_none()
                    && !v.on_wall_patrol()
                    && !matches!(v.name(), "Gaspare" | "Ettore")
            })
            .map(|(e, v)| (e, v.name().to_string()))
            .collect();
        who.sort_by(|a, b| a.1.cmp(&b.1));
        let spots = crowd_spots(PIER_CROWD, Vec2::Y, who.len(), 7, 1.2, |p| {
            crate::lanterns::spot_is_free(&spatial, &field, &grounds, &movers, p, 0.4)
        });
        for ((e, _), spot) in who.iter().zip(&spots) {
            if let Ok((_, mut v)) = villagers.get_mut(*e) {
                v.call_to(Some(*spot));
                farewell.crowd.push(*e);
            }
        }
        info!(
            "[SCENE] il paese scende al molo: {} abitanti",
            farewell.crowd.len()
        );
    } else if !want && !farewell.crowd.is_empty() {
        for e in std::mem::take(&mut farewell.crowd) {
            if let Ok((_, mut v)) = villagers.get_mut(e) {
                v.call_to(None);
            }
        }
        info!("[SCENE] il molo si svuota");
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<DinoScenes>()
        .init_resource::<Farewell>()
        .init_resource::<Dinner>()
        .add_systems(
            Update,
            (
                dino_scenes,
                silvia_at_cave,
                farewell_at_the_pier,
                mill_raiders,
                dinner_at_the_inn,
            )
                .run_if(in_state(crate::AppState::Playing)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guards_meet_the_nearest_raider_and_go_back_to_their_post() {
        let post = ROAD_DINO;
        assert_eq!(guard_spot(post, &[], None), post);
        let raider = post.0 + Vec2::new(0.0, 8.0);
        let (spot, look) = guard_spot(post, &[raider], None);
        assert!((spot.distance(raider) - GUARD_STAND).abs() < 1e-3);
        assert!(look.dot(Vec2::Y) > 0.99);
        // Troppo lontano: resta al suo posto.
        assert_eq!(
            guard_spot(post, &[post.0 + Vec2::new(0.0, 30.0)], None),
            post
        );
        // Un passo del predone non gli fa cambiare strada.
        let moved = guard_spot(post, &[raider + Vec2::new(0.5, 0.0)], Some((spot, look)));
        assert_eq!(moved.0, spot);
    }

    #[test]
    fn the_crowd_stands_in_rows_behind_the_front_facing_the_sea() {
        let spots = crowd_spots(PIER_CROWD, Vec2::Y, 15, 7, 1.2, |_| true);
        assert_eq!(spots.len(), 15);
        for (i, (p, look)) in spots.iter().enumerate() {
            assert_eq!(*look, Vec2::Y);
            assert!(
                p.y <= PIER_CROWD.y + 1e-3,
                "nessuno davanti alla prima fila"
            );
            for (q, _) in &spots[i + 1..] {
                assert!(p.distance(*q) >= 1.0);
            }
        }
        // Un ostacolo al centro: lì non ci sta nessuno.
        let spots = crowd_spots(PIER_CROWD, Vec2::Y, 10, 7, 1.2, |p| {
            p.distance(PIER_CROWD) > 1.0
        });
        assert!(spots.iter().all(|(p, _)| p.distance(PIER_CROWD) > 1.0));
    }
}
