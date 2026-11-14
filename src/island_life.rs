//! Il mondo dell'isola che si muove da sé (ADR 0046).
//!
//! - **Casse delle mareggiate**: bauli col lucchetto di bronzo che il mare spinge sulle spiagge. Non
//!   hanno padrone: si scassinano senza pesare sul karma.
//! - **Bottino che torna**: nei luoghi fuori dalle mura (e nelle casse delle mareggiate) un mobile
//!   svuotato si riempie di nuovo, una notte a caso.
//! - **Il pedaggio del ponte** (evento EC-05): al tramonto e di notte due banditi sbarrano il ponte di
//!   legno sul ruscello. Si paga, si convincono col Carisma o si combatte: la scelta è nel dialogo
//!   (`eventi.txt`, missione `ec05`), qui si mettono in scena.

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

/// Una cassa delle mareggiate.
#[derive(Component)]
pub struct WreckChest;

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
                crate::entities::spawn_enemy(&mut commands, &server, at, to);
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
        let face = (BRIDGE - post).normalize_or_zero();
        commands
            .spawn((
                Toller,
                crate::scene::SceneNode,
                crate::entities::Npc {
                    name: TOLLER,
                    mode: crate::entities::NpcMode::Ready,
                    lines: &["Il ponte è nostro, stanotte."],
                    shop: None,
                },
                crate::entities::Interactable {
                    prompt: "[E] Parla".to_string(),
                    range: 3.0,
                    aim: Some(0.5),
                    use_from: None,
                },
                crate::figure::ModelName("ladro"),
                crate::hud::NameAnchor(1.1),
                RigidBody::Static,
                Collider::capsule(0.35, 1.0),
                Transform::from_xyz(post.x, y + crate::figure::CENTER_Y + 0.05, post.y)
                    .with_rotation(Quat::from_rotation_y((-face.x).atan2(-face.y))),
                Visibility::default(),
            ))
            .with_children(|c| crate::figure::spawn_character(c, &server, "ladro"));
    }
    info!("[MONDO] pedaggio: due banditi sul ponte");
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, spawn_wreck_chests).add_systems(
        Update,
        (refill_loot, bridge_toll).run_if(in_state(crate::AppState::Playing)),
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
