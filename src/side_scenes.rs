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
}

/// Sistema: QS-09, il duello con Dino e la difesa della strada del Sud.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn dino_scenes(
    mut commands: Commands,
    missions: Res<Missions>,
    clock: Res<crate::daynight::GameClock>,
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
    raiders: Query<(Entity, &crate::combat::EnemyHealth), With<SouthRaider>>,
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
    let dino_spot = if duel {
        Some(DUEL_SPOT)
    } else if defense {
        Some(ROAD_DINO)
    } else {
        None
    };
    call(
        &mut villagers,
        "Guardia Dino",
        dino_spot,
        &[DUEL_SPOT, ROAD_DINO],
    );
    call(
        &mut villagers,
        "Guardia Aldo",
        (step == Some(4) && dusk || step == Some(5)).then_some(ROAD_ALDO),
        &[ROAD_ALDO],
    );
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
            for (e, _) in &raiders {
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
        2 if raiders.iter().all(|(_, hp)| hp.current <= 0.0) => {
            crate::karma::flag("predoni-sud-vinti");
            info!("[SCENE] strada del Sud: i predoni sono a terra");
            state.fight = 3;
        }
        _ => {}
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
        .add_systems(
            Update,
            (dino_scenes, silvia_at_cave, farewell_at_the_pier)
                .run_if(in_state(crate::AppState::Playing)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

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
