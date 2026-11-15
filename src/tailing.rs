//! Pedinare un abitante la sera (ADR 0045, aggiornamento).
//!
//! Alcuni abitanti, in certe sere, vanno di nascosto in un posto legato al loro segreto (`pedina:` in
//! `abitanti.txt`): Marta porta il dolce alla cesta di Gino al mulino, Agnese va ad ascoltare la pietra
//! del santuario. Chi le sta dietro mentre camminano (entro [`FOLLOW_MAX`] metri, per almeno
//! [`NEED`] secondi) senza farsi vedere, e all'arrivo è ancora vicino, ne scopre il segreto.
//! Troppo vicino, o davanti a loro, si viene notati: per quel giorno il pedinamento è fallito.

use bevy::prelude::*;
use std::collections::HashMap;

/// Oltre questa distanza non la si sta seguendo.
pub const FOLLOW_MAX: f32 = 22.0;
/// Secondi di pedinamento che servono.
pub const NEED: f32 = 12.0;
/// All'arrivo bisogna essere almeno così vicini per vedere che cosa fa.
pub const WITNESS: f32 = 16.0;
/// Più vicino di così si viene sentiti, comunque.
pub const TOO_CLOSE: f32 = 2.5;
/// Davanti a lei, entro questa distanza (meno con la Furtività), si viene visti.
pub const FRONT_SIGHT: f32 = 9.0;

/// Il giocatore in `player` viene notato da chi sta in `at` e cammina verso `heading`?
pub fn noticed(at: Vec2, heading: Vec2, player: Vec2, stealth_rank: f32) -> bool {
    let to = player - at;
    let d = to.length();
    if d < TOO_CLOSE {
        return true;
    }
    let sight = FRONT_SIGHT * (1.0 - 0.1 * stealth_rank);
    d < sight && heading.length() > 1e-3 && to.normalize().dot(heading.normalize()) > 0.5
}

#[derive(Default, Debug)]
pub struct Watch {
    /// Secondi passati a seguirla in questa passeggiata.
    pub followed: f32,
    /// Giorno in cui ti ha notato (per quel giorno non si fida).
    pub spotted: Option<u32>,
    last: Option<Vec2>,
    heading: Vec2,
}

#[derive(Resource, Default)]
pub struct Tailing(pub HashMap<String, Watch>);

impl Tailing {
    /// Verso (unitario) in cui sta camminando l'abitante, se lo si sta osservando.
    pub fn heading(&self, name: &str) -> Option<Vec2> {
        self.0
            .get(name)
            .map(|w| w.heading)
            .filter(|h| h.length() > 1e-3)
            .map(Vec2::normalize)
    }
}

/// Sistema: segue i pedinamenti in corso.
#[allow(clippy::too_many_arguments)]
pub fn tail_villagers(
    time: Res<Time>,
    clock: Res<crate::daynight::GameClock>,
    mut tailing: ResMut<Tailing>,
    mut karma: ResMut<crate::karma::Karma>,
    secrets: Res<crate::karma::Secrets>,
    mut session: ResMut<crate::save::Session>,
    mut barks: ResMut<crate::quests::Barks>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    villagers: Query<(Entity, &Transform, &crate::villagers::Villager)>,
    inventory: Res<crate::inventory::Inventory>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let me = player.translation().xz();
    let dt = time.delta_secs();
    let today = clock.date.day_key();
    // Il Mantello del Navigatore (QS-11) vale qualche rango di Furtività in più.
    let stealth = crate::skills::rank(crate::skills::Skill::Stealth)
        + crate::inventory::gear_perk(&inventory, crate::inventory::Relic::NavigatorCloak);
    for (entity, tf, v) in &villagers {
        let Some(goal) = v.secret_walk() else {
            continue;
        };
        let name = v.name().to_string();
        let Some(secret) = secrets.of(&name) else {
            continue;
        };
        if karma.knows_secret(&name) {
            continue;
        }
        let at = tf.translation.xz();
        let w = tailing.0.entry(name.clone()).or_default();
        if let Some(last) = w.last {
            let step = at - last;
            if step.length() > 1e-3 {
                w.heading = step;
            }
        }
        w.last = Some(at);
        if w.spotted == Some(today) {
            continue;
        }
        let d = me.distance(at);
        if v.en_route() {
            if d < FOLLOW_MAX && noticed(at, w.heading, me, stealth) {
                w.spotted = Some(today);
                w.followed = 0.0;
                karma.add_trust(&name, -1);
                barks.say(
                    entity,
                    &name,
                    crate::quests::bark_line(&name, "pedinata")
                        .unwrap_or("Mi stai seguendo?")
                        .to_string(),
                );
                session.notify(format!("{name} ti ha notato"));
                info!("[PEDINA] {name} ti ha notato");
            } else if d < FOLLOW_MAX {
                w.followed += dt;
                crate::skills::train(crate::skills::Skill::Stealth, dt * 0.6);
            }
        } else if at.distance(goal) < 2.0 {
            if w.followed >= NEED && d < WITNESS {
                crate::karma::reveal(&mut karma, secret);
                crate::skills::train(crate::skills::Skill::Stealth, 25.0);
                session.notify(format!(
                    "Hai seguito {name} senza farti vedere: ora conosci il suo segreto"
                ));
                info!("[PEDINA] segreto di {name} scoperto");
            }
            w.followed = 0.0;
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Tailing>().add_systems(
        Update,
        tail_villagers.run_if(in_state(crate::AppState::Playing)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behind_at_a_distance_is_safe_in_front_or_too_close_is_not() {
        let at = Vec2::ZERO;
        let north = Vec2::new(0.0, -1.0);
        // Dietro, a 8 m: non ti vede.
        assert!(!noticed(at, north, Vec2::new(0.0, 8.0), 0.0));
        // Davanti, a 6 m: ti vede; con la Furtività al massimo un po' meno lontano.
        assert!(noticed(at, north, Vec2::new(0.0, -6.0), 0.0));
        assert!(!noticed(at, north, Vec2::new(0.0, -6.0), 5.0));
        // Attaccato alle spalle: ti sente.
        assert!(noticed(at, north, Vec2::new(0.0, 2.0), 0.0));
        // Di fianco, a 7 m: non ti vede.
        assert!(!noticed(at, north, Vec2::new(7.0, 0.0), 0.0));
    }

    #[test]
    fn marta_and_agnese_walk_in_secret_in_their_evening() {
        let defs = crate::villagers::load_defs();
        for name in ["Marta", "Agnese"] {
            let d = defs.iter().find(|d| d.name == name).expect(name);
            let (from, to, ..) = d.secret_walk.expect("pedina:");
            assert!(from < to);
            assert!(
                crate::karma::Secrets::default().of(name).is_some(),
                "{name} ha un segreto"
            );
        }
    }
}
