//! Il mondo dell'isola che si muove da sé (ADR 0046).
//!
//! - **Casse delle mareggiate**: bauli col lucchetto di bronzo che il mare spinge sulle spiagge. Non
//!   hanno padrone: si scassinano senza pesare sul karma.
//! - **Bottino che torna**: nei luoghi fuori dalle mura (e nelle casse delle mareggiate) un mobile
//!   svuotato si riempie di nuovo, una notte a caso.
//! - **Il pedaggio del ponte** (evento EC-05): al tramonto e di notte due banditi sbarrano il ponte di
//!   legno sul ruscello. Si paga, si convincono col Carisma o si combatte: la scelta è nel dialogo
//!   (`eventi.txt`, missione `ec05`), qui si mettono in scena.
//! - **Il carretto nella nebbia** (EC-02): al crepuscolo Lino il carrettiere ha la ruota nel fango
//!   sulla strada del nord; mentre lo aiuti escono dal bosco due predoni.
//! - **La disputa del mercato** (EC-08): in tarda mattinata Tonio e Nina litigano ad alta voce in
//!   piazza finché non fai da giudice.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::containers::{ContainerKind, Containers};
use crate::region::Region;

/// Edificio delle casse delle mareggiate (per contenuto e serratura).
pub const WRECK: &str = "mareggiata";
const WRECK_CHESTS: usize = 4;
/// Distanza minima fra due casse delle mareggiate.
const WRECK_SPACING: f32 = 45.0;

/// Un mobile può riempirsi di nuovo con questa probabilità, ogni notte.
const REFILL_CHANCE: f32 = 0.4;

/// Il ponte sul ruscello (centro) e i punti dove stanno i due banditi del pedaggio.
pub const BRIDGE: Vec2 = Vec2::new(81.0, -8.0);
const TOLL_POSTS: [Vec2; 2] = [Vec2::new(79.4, -4.6), Vec2::new(80.4, -5.2)];
pub const TOLLER: &str = "Bandito del ponte";
/// Da quanto lontano si vedono (e si mettono al loro posto).
const TOLL_RANGE: f32 = 45.0;

/// EC-02: dove sta il carretto, dove sta Lino, da dove escono i predoni (e dove vanno).
const CART: Vec2 = Vec2::new(1.7, -24.0);
const CARTER_AT: Vec2 = Vec2::new(-0.7, -22.8);
/// Coseno dell'angolo dallo sguardo oltre cui il carretto è fuori vista (circa 75°).
const CART_UNSEEN_COS: f32 = 0.26;

pub const CARTER: &str = "Lino il carrettiere";
const RAIDERS: [(Vec2, Vec2); 2] = [
    (Vec2::new(15.0, -31.0), Vec2::new(3.0, -25.0)),
    (Vec2::new(-13.0, -29.0), Vec2::new(-1.5, -24.5)),
];
/// Da quanto lontano compare la scena del carretto.
const CART_RANGE: f32 = 45.0;

/// EC-08: secondi fra una battuta della lite e l'altra (battute `lite-<n>` in `battute.txt`).
const QUARREL_EVERY: f32 = 4.5;

/// Una cassa delle mareggiate.
#[derive(Component)]
pub struct WreckChest;

/// La scena del carretto (Lino e il carretto) e i suoi predoni.
#[derive(Component)]
pub struct CartScene;
#[derive(Component)]
pub struct Raider;

/// Uno dei banditi del pedaggio, finché si può parlare con loro.
#[derive(Component)]
pub struct Toller;

/// Punti di spiaggia dove il mare lascia le casse: lungo la costa, a quota di sabbia, distanti fra
/// loro. Sempre gli stessi per lo stesso terreno.
pub fn wreck_spots(height: impl Fn(f32, f32) -> f32) -> Vec<Vec2> {
    let sea = crate::terrain::SEA_LEVEL;
    let mut spots: Vec<Vec2> = Vec::new();
    for k in 0..48 {
        let a = (k as f32 * 7.5 + 20.0).to_radians();
        let dir = Vec2::new(a.cos(), a.sin());
        // Dal largo verso l'interno, il primo punto asciutto e basso.
        let mut r = 148.0;
        while r > 50.0 {
            let p = dir * r;
            let h = height(p.x, p.y);
            if h > sea + 0.15 {
                let flat = (height(p.x + 1.0, p.y) - h).abs() < 0.35
                    && (height(p.x, p.y + 1.0) - h).abs() < 0.35;
                if h < sea + 1.2 && flat && spots.iter().all(|s| s.distance(p) >= WRECK_SPACING) {
                    spots.push(p - dir * 2.0);
                }
                break;
            }
            r -= 1.0;
        }
        if spots.len() >= WRECK_CHESTS {
            break;
        }
    }
    spots
}

/// Sistema: le casse delle mareggiate sulle spiagge dell'isola (una volta per scena).
pub fn spawn_wreck_chests(
    mut commands: Commands,
    field: Option<Res<crate::terrain::TerrainField>>,
    pbr: Res<crate::pbr::PbrMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    chests: Query<(), With<WreckChest>>,
) {
    let Some(field) = field else {
        return;
    };
    if crate::region::current() != Region::Isola || !chests.is_empty() {
        return;
    }
    let wood = pbr.get(crate::pbr::Mat::Planks);
    let iron = pbr.get(crate::pbr::Mat::Iron);
    let body = meshes.add(crate::pbr::with_tangents(Cuboid::new(0.95, 0.55, 0.6)));
    let band = meshes.add(crate::pbr::with_tangents(Cuboid::new(0.99, 0.08, 0.64)));
    for (i, p) in wreck_spots(|x, z| field.height_at(x, z))
        .into_iter()
        .enumerate()
    {
        let y = field.height_at(p.x, p.y);
        let yaw = crate::world_props::hash01(p, 3.0) * std::f32::consts::TAU;
        commands
            .spawn((
                WreckChest,
                crate::scene::SceneNode,
                RigidBody::Static,
                Collider::cuboid(0.95, 0.55, 0.6),
                Mesh3d(body.clone()),
                MeshMaterial3d(wood.clone()),
                Transform::from_xyz(p.x, y + 0.22, p.y)
                    .with_rotation(Quat::from_rotation_y(yaw) * Quat::from_rotation_z(0.08)),
                Visibility::default(),
            ))
            .with_children(|c| {
                c.spawn((
                    Mesh3d(band.clone()),
                    MeshMaterial3d(iron.clone()),
                    Transform::from_xyz(0.0, 0.12, 0.0),
                ));
                c.spawn(crate::containers::bundle(
                    ContainerKind::Chest,
                    Vec3::new(0.0, 0.3, 0.0),
                    format!("{WRECK}:{i}"),
                    WRECK,
                ));
            });
        info!("[MONDO] cassa delle mareggiate a ({:.0},{:.0})", p.x, p.y);
    }
}

/// Il mobile `key` sta in un luogo fuori dalle mura (o è una cassa delle mareggiate)? Allora il suo
/// bottino può tornare. Rende l'edificio, per sapere che cosa rimetterci.
fn outdoor_building(dbg: &crate::debug_place::DebugPlace, key: &str) -> Option<String> {
    if key.starts_with(&format!("{WRECK}:")) {
        return Some(WRECK.to_string());
    }
    let id: u32 = key.split(':').next()?.parse().ok()?;
    let kind = &dbg.items.iter().find(|p| p.id == id)?.kind;
    crate::places::is_place(kind).then(|| kind.clone())
}

/// Sistema: a ogni notte nuova, qualche mobile svuotato fuori dalle mura si riempie di nuovo.
pub fn refill_loot(
    clock: Res<crate::daynight::GameClock>,
    dbg: Res<crate::debug_place::DebugPlace>,
    mut containers: ResMut<Containers>,
    mut last: Local<Option<i64>>,
) {
    let night = crate::places::night_index(&clock);
    let night_hours = clock.hour >= 21.5 || clock.hour < 4.5;
    if !night_hours || *last == Some(night) {
        return;
    }
    // La prima notte vista dopo l'avvio fa solo da riferimento.
    let first = last.is_none();
    *last = Some(night);
    if first {
        return;
    }
    for (key, kind) in containers.emptied() {
        let Some(building) = outdoor_building(&dbg, &key) else {
            continue;
        };
        if crate::containers::roll(&key, night as u32) >= REFILL_CHANCE {
            continue;
        }
        let items =
            crate::containers::starting_items(kind, &building, "", &format!("{key}#{night}"));
        if !items.is_empty() {
            containers.refill(&key, &items);
            info!("[MONDO] bottino tornato in {key}");
        }
    }
}

/// Sistema: i banditi del pedaggio, al tramonto e di notte, finché l'evento non è chiuso; poi se ne
/// vanno (pagati o convinti) o diventano nemici (se li si sfida).
#[allow(clippy::too_many_arguments)]
pub fn bridge_toll(
    mut commands: Commands,
    server: Res<AssetServer>,
    clock: Res<crate::daynight::GameClock>,
    missions: Res<crate::quests::missions::Missions>,
    field: Option<Res<crate::terrain::TerrainField>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    tollers: Query<(Entity, &GlobalTransform), With<Toller>>,
    mut session: ResMut<crate::save::Session>,
) {
    let (Some(field), Ok(player)) = (field, players.single()) else {
        return;
    };
    if crate::region::current() != Region::Isola {
        return;
    }
    let done = missions.done("ec05");
    if done {
        if tollers.is_empty() {
            return;
        }
        let fight = missions.unlocked("pedaggio-lotta");
        for (e, t) in &tollers {
            let at = t.translation();
            commands.entity(e).despawn();
            if fight {
                let to = at + Vec3::new(2.5, 0.0, -2.0);
                let e = crate::entities::spawn_enemy(&mut commands, &server, at, to);
                commands.entity(e).insert(crate::combat::NoRespawn);
            }
        }
        if fight {
            session.notify("I banditi del ponte sfoderano i coltelli!");
            info!("[MONDO] pedaggio: si combatte");
        } else {
            info!("[MONDO] pedaggio: i banditi se ne vanno");
        }
        return;
    }
    let evening = clock.hour >= 18.0 || clock.hour < 5.0;
    let near = player.translation().xz().distance(BRIDGE) <= TOLL_RANGE;
    if !missions.enabled || !evening || !near {
        for (e, _) in &tollers {
            commands.entity(e).despawn();
        }
        return;
    }
    if !tollers.is_empty() {
        return;
    }
    for post in TOLL_POSTS {
        let y = field.height_at(post.x, post.y);
        let e = spawn_talker(
            &mut commands,
            &server,
            TOLLER,
            "ladro",
            crate::quests::talker_lines(TOLLER),
            Vec3::new(post.x, y, post.y),
            BRIDGE - post,
        );
        commands.entity(e).insert(Toller);
    }
    info!("[MONDO] pedaggio: due banditi sul ponte");
}

/// Un personaggio di passaggio con cui si parla (banditi del ponte, Lino).
fn spawn_talker(
    commands: &mut Commands,
    server: &AssetServer,
    name: &'static str,
    model: &'static str,
    lines: &'static [&'static str],
    at: Vec3,
    face: Vec2,
) -> Entity {
    commands
        .spawn((
            crate::scene::SceneNode,
            crate::entities::Npc {
                name,
                mode: crate::entities::NpcMode::Ready,
                lines,
                shop: None,
            },
            crate::entities::Interactable {
                prompt: "[E] Parla".to_string(),
                range: 3.0,
                aim: Some(0.5),
                use_from: None,
            },
            crate::figure::ModelName(model),
            crate::hud::NameAnchor(1.1),
            RigidBody::Static,
            Collider::capsule(0.35, 1.0),
            Transform::from_xyz(at.x, at.y + crate::figure::CENTER_Y + 0.05, at.z)
                .with_rotation(Quat::from_rotation_y((-face.x).atan2(-face.y))),
            Visibility::default(),
        ))
        .with_children(|c| crate::figure::spawn_character(c, server, model))
        .id()
}

/// Sistema: EC-02, il carretto nella nebbia. Lino compare al crepuscolo vicino al carretto finché
/// l'evento è aperto; dopo il primo dialogo escono due predoni dal bosco (`predoni-vinti` quando
/// sono a terra); a evento chiuso Lino e il carretto se ne vanno.
#[allow(clippy::too_many_arguments)]
pub fn cart_in_the_mud(
    mut commands: Commands,
    server: Res<AssetServer>,
    clock: Res<crate::daynight::GameClock>,
    missions: Res<crate::quests::missions::Missions>,
    karma: Res<crate::karma::Karma>,
    field: Option<Res<crate::terrain::TerrainField>>,
    pbr: Res<crate::pbr::PbrMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    scene: Query<Entity, With<CartScene>>,
    raiders: Query<&crate::combat::EnemyHealth, With<Raider>>,
    mut fight: Local<u8>,
    mut session: ResMut<crate::save::Session>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
) {
    let (Some(field), Ok(player)) = (field, players.single()) else {
        return;
    };
    let step = missions.step_of("ec02").unwrap_or(0);
    let open =
        missions.enabled && crate::region::current() == Region::Isola && !missions.done("ec02");
    let near = player.translation().xz().distance(CART) <= CART_RANGE;
    let dusk = (18.0..20.5).contains(&clock.hour);
    let wanted = open && near && (step > 0 || dusk);
    if !wanted {
        // Lino e il carretto se ne vanno solo quando non li guardi: prima sparivano davanti agli
        // occhi appena chiuso l'evento (segnalazione 2026-11-19_21-41-04).
        let seen = near
            && cameras.iter().any(|c| {
                let to = Vec3::new(CART.x, c.translation().y, CART.y) - c.translation();
                to.normalize_or_zero().dot(*c.forward()) > CART_UNSEEN_COS
            });
        if seen {
            return;
        }
        for e in &scene {
            commands.entity(e).despawn();
        }
        *fight = 0;
        return;
    }
    if scene.is_empty() {
        let ground = |p: Vec2| Vec3::new(p.x, field.height_at(p.x, p.y), p.y);
        let lino = spawn_talker(
            &mut commands,
            &server,
            CARTER,
            "civile-m-ocra",
            crate::quests::talker_lines(CARTER),
            ground(CARTER_AT),
            CART - CARTER_AT,
        );
        commands.entity(lino).insert(CartScene);
        let model = crate::buildings::model("carretto", 3);
        commands
            .spawn((
                CartScene,
                crate::scene::SceneNode,
                Transform::from_translation(ground(CART) - Vec3::Y * 0.12)
                    .with_rotation(Quat::from_rotation_y(0.35) * Quat::from_rotation_z(0.08)),
                Visibility::default(),
                RigidBody::Static,
            ))
            .with_children(|c| {
                for (m, mesh) in model.parts {
                    c.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(pbr.get(m))));
                }
                for (center, size) in model.colliders {
                    c.spawn((
                        Collider::cuboid(size.x, size.y, size.z),
                        Transform::from_translation(center),
                    ));
                }
            });
        info!("[MONDO] carretto nel fango sulla strada del nord");
    }
    if step == 1 && !karma.unlocks.contains("predoni-vinti") {
        match *fight {
            0 => {
                for (from, to) in RAIDERS {
                    let start = Vec3::new(from.x, field.height_at(from.x, from.y) + 1.0, from.y);
                    let end = Vec3::new(to.x, field.height_at(to.x, to.y) + 1.0, to.y);
                    let e = crate::entities::spawn_enemy(&mut commands, &server, start, end);
                    commands
                        .entity(e)
                        .insert((Raider, CartScene, crate::combat::NoRespawn));
                }
                session.notify("Dal bosco escono due predoni!");
                info!("[MONDO] carretto: arrivano i predoni");
                *fight = 1;
            }
            1 if !raiders.is_empty() => *fight = 2,
            2 if raiders.iter().all(|hp| hp.current <= 0.0) => {
                crate::karma::flag("predoni-vinti");
                info!("[MONDO] carretto: i predoni sono a terra");
                *fight = 3;
            }
            _ => {}
        }
    }
}

/// Sistema: EC-08, la disputa del mercato. Finché non hai fatto da giudice, in tarda mattinata
/// Tonio e Nina si rimbeccano ad alta voce quando sei in piazza.
#[allow(clippy::too_many_arguments)]
pub fn market_quarrel(
    time: Res<Time>,
    clock: Res<crate::daynight::GameClock>,
    missions: Res<crate::quests::missions::Missions>,
    dialog: Res<crate::entities::DialogState>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    npcs: Query<(Entity, &GlobalTransform, &crate::entities::Npc)>,
    mut barks: ResMut<crate::quests::Barks>,
    mut beat: Local<(f32, usize)>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let active = missions.enabled
        && crate::region::current() == Region::Isola
        && missions.step_of("ec08") == Some(0)
        && (10.0..13.0).contains(&clock.hour)
        && !dialog.open;
    if !active {
        return;
    }
    beat.0 -= time.delta_secs();
    if beat.0 > 0.0 {
        return;
    }
    let quarrel = crate::quests::numbered_lines("lite");
    if quarrel.is_empty() {
        return;
    }
    let (who, line) = quarrel[beat.1 % quarrel.len()];
    let Some((e, t, npc)) = npcs.iter().find(|(.., n)| n.name == who) else {
        return;
    };
    if t.translation().distance(player.translation()) > 25.0 {
        return;
    }
    barks.say(e, npc.name, line.to_string());
    beat.0 = QUARREL_EVERY;
    beat.1 += 1;
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, spawn_wreck_chests).add_systems(
        Update,
        (refill_loot, bridge_toll, cart_in_the_mud, market_quarrel)
            .run_if(in_state(crate::AppState::Playing)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un'isola rotonda: piana a 2 m fino a 100 m dal centro, poi scende in mare.
    fn island(x: f32, z: f32) -> f32 {
        let r = Vec2::new(x, z).length();
        2.0 - (r - 100.0).max(0.0) * 0.1
    }

    #[test]
    fn wreck_chests_lie_on_the_beach_far_apart() {
        let spots = wreck_spots(island);
        assert_eq!(spots.len(), WRECK_CHESTS);
        for (i, a) in spots.iter().enumerate() {
            let h = island(a.x, a.y);
            assert!(
                h > crate::terrain::SEA_LEVEL && h < crate::terrain::SEA_LEVEL + 1.6,
                "{h}"
            );
            for b in &spots[i + 1..] {
                assert!(a.distance(*b) >= WRECK_SPACING);
            }
        }
        assert_eq!(spots, wreck_spots(island), "sempre gli stessi");
    }

    #[test]
    fn only_outdoor_furniture_gets_its_loot_back() {
        let mut dbg = crate::debug_place::DebugPlace::default();
        dbg.set_runtime("rovine", Some(Vec2::ZERO));
        dbg.set_runtime("casa-piccola", Some(Vec2::ONE));
        let ruins = dbg.items.iter().find(|p| p.kind == "rovine").unwrap().id;
        assert_eq!(
            outdoor_building(&dbg, &format!("{ruins}:0:0")),
            Some("rovine".into())
        );
        assert_eq!(outdoor_building(&dbg, "mareggiata:2"), Some(WRECK.into()));
        assert_eq!(outdoor_building(&dbg, "casetta:baule"), None);
    }
}
