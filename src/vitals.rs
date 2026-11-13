//! Vigore e magia del personaggio (ADR 0039).
//!
//! - Il **vigore** si consuma correndo, saltando e colpendo; torna da solo dopo un attimo di
//!   respiro. A zero si resta senza fiato: niente corsa finché non se ne recupera un quarto.
//! - La **magia** è la riserva per gli incantesimi (che arriveranno): per ora si riempie da sola.
//!
//! Chi consuma vigore non deve conoscere la risorsa: [`spend`] scrive in un contatore globale,
//! svuotato una volta per frame da [`tick`].

use std::sync::Mutex;

use bevy::prelude::*;

use crate::character::{Stat, Stats};

/// Vigore al secondo consumato dalla corsa, e costi di un salto e di un colpo.
pub const SPRINT_DRAIN: f32 = 9.0;
pub const JUMP_COST: f32 = 7.0;
pub const SWING_COST: f32 = 5.0;
/// Recupero al secondo e pausa (s) dopo l'ultimo sforzo prima che ricominci.
const VIGOR_REGEN: f32 = 16.0;
const REST_DELAY: f32 = 1.0;
/// Frazione da recuperare, dopo essere rimasti senza fiato, per tornare a correre.
const WINDED_UNTIL: f32 = 0.25;
const MAGIC_REGEN: f32 = 0.6;

pub fn max_vigor(stats: Stats) -> f32 {
    let con = f32::from(stats.get(Stat::Constitution)) - 10.0;
    let agi = f32::from(stats.get(Stat::Agility)) - 10.0;
    (100.0 + 5.0 * con + 2.0 * agi).max(40.0)
}

pub fn max_magic(stats: Stats) -> f32 {
    let int = f32::from(stats.get(Stat::Intelligence)) - 10.0;
    let wis = f32::from(stats.get(Stat::Wisdom)) - 10.0;
    (50.0 + 5.0 * int + 3.0 * wis).max(10.0)
}

/// Riserve del personaggio oltre alla vita.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct Vitals {
    pub vigor: f32,
    pub vigor_max: f32,
    pub magic: f32,
    pub magic_max: f32,
    /// Secondi da aspettare prima che il vigore torni a salire.
    rest: f32,
    /// Senza fiato: non si corre finché il vigore non risale.
    pub winded: bool,
}

impl Default for Vitals {
    fn default() -> Self {
        let stats = Stats::default();
        Self {
            vigor: max_vigor(stats),
            vigor_max: max_vigor(stats),
            magic: max_magic(stats),
            magic_max: max_magic(stats),
            rest: 0.0,
            winded: false,
        }
    }
}

impl Vitals {
    /// Pieno di vigore e magia per queste statistiche (partita nuova, rinascita).
    pub fn full(stats: Stats) -> Self {
        Self {
            vigor: max_vigor(stats),
            vigor_max: max_vigor(stats),
            magic: max_magic(stats),
            magic_max: max_magic(stats),
            rest: 0.0,
            winded: false,
        }
    }

    pub fn can_sprint(&self) -> bool {
        !self.winded && self.vigor > 0.0
    }

    /// Consuma vigore (mai sotto zero) e ferma per un attimo il recupero.
    pub fn use_vigor(&mut self, amount: f32) {
        if amount <= 0.0 {
            return;
        }
        self.vigor = (self.vigor - amount).max(0.0);
        self.rest = REST_DELAY;
        if self.vigor <= 0.0 {
            self.winded = true;
        }
    }

    /// Passa il tempo: i massimi seguono le statistiche, le riserve si riempiono.
    pub fn update(&mut self, stats: Stats, dt: f32) {
        self.vigor_max = max_vigor(stats);
        self.magic_max = max_magic(stats);
        self.rest = (self.rest - dt).max(0.0);
        if self.rest <= 0.0 {
            self.vigor += VIGOR_REGEN * dt;
        }
        self.vigor = self.vigor.clamp(0.0, self.vigor_max);
        if self.winded && self.vigor >= self.vigor_max * WINDED_UNTIL {
            self.winded = false;
        }
        self.magic = (self.magic + MAGIC_REGEN * dt).clamp(0.0, self.magic_max);
    }
}

static SPENT: Mutex<f32> = Mutex::new(0.0);

/// Vigore speso in questo frame da un sistema qualsiasi (salto, colpo).
pub fn spend(amount: f32) {
    if let Ok(mut s) = SPENT.lock() {
        *s += amount;
    }
}

/// Sistema: applica gli sforzi del frame e fa recuperare. Con `god` il vigore non cala.
pub fn tick(
    time: Res<Time>,
    character: Res<crate::character::Character>,
    cheats: Res<crate::shell::Cheats>,
    mut vitals: ResMut<Vitals>,
) {
    let spent = SPENT
        .lock()
        .map(|mut s| std::mem::take(&mut *s))
        .unwrap_or(0.0);
    if cheats.god {
        vitals.vigor = vitals.vigor_max;
        vitals.winded = false;
    } else {
        vitals.use_vigor(spent);
    }
    vitals.update(character.stats, time.delta_secs());
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Vitals>()
        .add_systems(Update, tick.run_if(in_state(crate::AppState::Playing)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_out_of_vigor_stops_the_sprint_until_it_recovers() {
        let stats = Stats::default();
        let mut v = Vitals::full(stats);
        assert!(v.can_sprint());
        v.use_vigor(500.0);
        assert_eq!(v.vigor, 0.0);
        assert!(!v.can_sprint());
        // Per un secondo non recupera, poi sì; torna a correre a un quarto.
        v.update(stats, 0.5);
        assert_eq!(v.vigor, 0.0);
        v.update(stats, 0.6);
        assert!(v.vigor > 5.0 && !v.can_sprint());
        v.update(stats, 1.0);
        assert!(v.can_sprint());
        v.update(stats, 60.0);
        assert_eq!(v.vigor, v.vigor_max);
    }

    #[test]
    fn stats_set_the_size_of_the_reserves() {
        let mut stats = Stats::default();
        assert_eq!((max_vigor(stats), max_magic(stats)), (100.0, 50.0));
        stats.set(Stat::Constitution, 14);
        stats.set(Stat::Intelligence, 16);
        assert_eq!((max_vigor(stats), max_magic(stats)), (120.0, 80.0));
        let mut v = Vitals::full(Stats::default());
        v.update(stats, 0.0);
        assert_eq!((v.vigor_max, v.magic_max), (120.0, 80.0));
    }
}
