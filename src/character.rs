//! Il personaggio (ADR 0024): statistiche, classi, portata e sovraccarico.
//!
//! Le formule sono funzioni pure: 10 è il valore neutro di ogni statistica.

use crate::AppState;
use crate::containers::PLAYER_CARRY_KG;
use crate::inventory::{Equipment, Inventory};
use crate::player::{Player, PlayerHealth};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

/// Valore neutro e limiti di una statistica.
pub const STAT_NEUTRAL: u8 = 10;
pub const STAT_MIN: u8 = 1;
pub const STAT_MAX: u8 = 20;
/// Sotto questa portata non si scende, qualunque sia la forza.
const MIN_CARRY_KG: f32 = 15.0;
/// Oltre il limite si cammina a questa frazione della velocità.
pub const OVERLOAD_SPEED: f32 = 0.5;

/// Le statistiche del personaggio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stat {
    Strength,
    Constitution,
    Agility,
    Intelligence,
    Wisdom,
    Charisma,
    Luck,
}

impl Stat {
    pub const ALL: [Stat; 7] = [
        Stat::Strength,
        Stat::Constitution,
        Stat::Agility,
        Stat::Intelligence,
        Stat::Wisdom,
        Stat::Charisma,
        Stat::Luck,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Stat::Strength => "Forza",
            Stat::Constitution => "Costituzione",
            Stat::Agility => "Agilità",
            Stat::Intelligence => "Intelligenza",
            Stat::Wisdom => "Saggezza",
            Stat::Charisma => "Carisma",
            Stat::Luck => "Fortuna",
        }
    }

    /// Cosa fa, per il pannello del personaggio.
    pub fn effect(self) -> &'static str {
        match self {
            Stat::Strength => "portata e danno in mischia",
            Stat::Constitution => "vita massima",
            Stat::Agility => "velocità",
            Stat::Intelligence | Stat::Wisdom => "servirà per la magia",
            Stat::Charisma => "prezzi nei negozi",
            Stat::Luck => "monete dai nemici",
        }
    }

    /// Cerca una statistica per nome, senza badare a maiuscole e accenti.
    pub fn parse(text: &str) -> Option<Stat> {
        let wanted = crate::inventory_ui::normalize(text);
        Stat::ALL
            .into_iter()
            .find(|s| crate::inventory_ui::normalize(s.name()) == wanted)
    }

    fn index(self) -> usize {
        Stat::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// Valori delle sette statistiche.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats([u8; 7]);

impl Default for Stats {
    fn default() -> Self {
        Self([STAT_NEUTRAL; 7])
    }
}

impl Stats {
    pub fn new(values: [u8; 7]) -> Self {
        Self(values.map(|v| v.clamp(STAT_MIN, STAT_MAX)))
    }

    pub fn get(self, stat: Stat) -> u8 {
        self.0[stat.index()]
    }

    pub fn set(&mut self, stat: Stat, value: u8) {
        self.0[stat.index()] = value.clamp(STAT_MIN, STAT_MAX);
    }

    fn delta(self, stat: Stat) -> f32 {
        f32::from(self.get(stat)) - f32::from(STAT_NEUTRAL)
    }

    /// Chili in più (o in meno) dovuti alla forza.
    pub fn strength_carry(self) -> f32 {
        self.delta(Stat::Strength)
    }

    pub fn melee_factor(self) -> f32 {
        1.0 + 0.04 * self.delta(Stat::Strength)
    }

    pub fn max_health(self) -> f32 {
        100.0 + 5.0 * self.delta(Stat::Constitution)
    }

    pub fn speed_factor(self) -> f32 {
        1.0 + 0.01 * self.delta(Stat::Agility)
    }

    /// Prezzo in negozio: il carisma lo abbassa, mai sotto una moneta.
    pub fn price(self, base: u32) -> u32 {
        let factor = 1.0 - 0.02 * self.delta(Stat::Charisma);
        ((base as f32 * factor).round() as u32).max(1)
    }

    /// Monete lasciate da un nemico comune.
    pub fn enemy_coins(self) -> u32 {
        (4.0 + (self.delta(Stat::Luck) / 2.0).floor()).max(1.0) as u32
    }
}

/// Classi di partenza (si scelgono dalla «Nuova partita»).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Warrior,
    Explorer,
    Scholar,
    Merchant,
    Custom,
}

impl Class {
    pub const ALL: [Class; 5] = [
        Class::Warrior,
        Class::Explorer,
        Class::Scholar,
        Class::Merchant,
        Class::Custom,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Class::Warrior => "Guerriero",
            Class::Explorer => "Esploratore",
            Class::Scholar => "Studioso",
            Class::Merchant => "Mercante",
            Class::Custom => "Personalizzato",
        }
    }

    /// Statistiche iniziali, nell'ordine di [`Stat::ALL`].
    pub fn stats(self) -> Stats {
        Stats::new(match self {
            Class::Warrior => [14, 13, 10, 8, 8, 8, 9],
            Class::Explorer => [10, 10, 14, 9, 10, 8, 9],
            Class::Scholar => [8, 8, 9, 15, 13, 9, 8],
            Class::Merchant => [8, 9, 9, 11, 10, 14, 9],
            Class::Custom => [8; 7],
        })
    }
}

/// Punti da distribuire nella creazione personalizzata, a partire da 8 in tutto.
pub const CUSTOM_POINTS: u8 = 14;

/// Il personaggio in gioco.
#[derive(Resource, Clone, Debug)]
pub struct Character {
    pub name: String,
    pub class: Class,
    pub stats: Stats,
}

impl Default for Character {
    fn default() -> Self {
        Self {
            name: "Viaggiatore".to_string(),
            class: Class::Custom,
            stats: Stats::default(),
        }
    }
}

/// Un effetto sulla portata: pozione, magia lanciata o altro.
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub source: String,
    pub carry_kg: f32,
    /// Secondi rimasti (`None` = finché non viene tolto).
    pub remaining: Option<f32>,
}

/// Effetti attivi.
#[derive(Resource, Default, Clone, Debug)]
pub struct ActiveEffects(pub Vec<Effect>);

impl ActiveEffects {
    /// Aggiunge un effetto; la stessa fonte non si somma, si rinnova.
    pub fn add(&mut self, effect: Effect) {
        self.0.retain(|e| e.source != effect.source);
        self.0.push(effect);
    }

    pub fn carry_kg(&self) -> f32 {
        self.0.iter().map(|e| e.carry_kg).sum::<f32>().abs_zero()
    }

    /// Fa scorrere il tempo e toglie ciò che è scaduto.
    pub fn tick(&mut self, dt: f32) {
        for e in &mut self.0 {
            if let Some(t) = &mut e.remaining {
                *t -= dt;
            }
        }
        self.0.retain(|e| e.remaining.is_none_or(|t| t > 0.0));
    }
}

/// La somma vuota di `f32` vale -0.0: la riporta a zero.
trait AbsZero {
    fn abs_zero(self) -> Self;
}

impl AbsZero for f32 {
    fn abs_zero(self) -> Self {
        if self == 0.0 { 0.0 } else { self }
    }
}

/// Da dove vengono i chili che si possono portare.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Carry {
    pub base: f32,
    pub backpack: f32,
    pub strength: f32,
    pub enchanted: f32,
    pub effects: f32,
}

impl Carry {
    pub fn of(stats: Stats, equip: &Equipment, effects: &ActiveEffects) -> Self {
        Self {
            base: PLAYER_CARRY_KG,
            backpack: equip.backpack.and_then(|b| b.backpack_kg()).unwrap_or(0.0),
            strength: stats.strength_carry(),
            enchanted: equip.charm.and_then(|c| c.charm_kg()).unwrap_or(0.0),
            effects: effects.carry_kg(),
        }
    }

    pub fn total(self) -> f32 {
        (self.base + self.backpack + self.strength + self.enchanted + self.effects)
            .max(MIN_CARRY_KG)
    }
}

/// Valori ricalcolati a ogni frame, per movimento e interfaccia.
#[derive(Resource, Debug, Clone, Copy)]
pub struct Load {
    pub carry: Carry,
    pub capacity: f32,
    pub weight: f32,
    pub overloaded: bool,
    pub speed_factor: f32,
}

impl Default for Load {
    fn default() -> Self {
        Self {
            carry: Carry::default(),
            capacity: PLAYER_CARRY_KG,
            weight: 0.0,
            overloaded: false,
            speed_factor: 1.0,
        }
    }
}

/// Sistema: peso portato e portata.
pub fn update_load(
    character: Res<Character>,
    inventory: Res<Inventory>,
    equip: Res<Equipment>,
    effects: Res<ActiveEffects>,
    mut load: ResMut<Load>,
) {
    let carry = Carry::of(character.stats, &equip, &effects);
    let capacity = carry.total();
    let weight = inventory.total_weight();
    *load = Load {
        carry,
        capacity,
        weight,
        overloaded: weight > capacity + 1e-3,
        speed_factor: character.stats.speed_factor(),
    };
}

/// Sistema: la vita massima segue la costituzione.
pub fn apply_stats(character: Res<Character>, mut players: Query<&mut PlayerHealth, With<Player>>) {
    let max = character.stats.max_health();
    for mut hp in &mut players {
        if hp.max != max {
            hp.max = max;
            hp.current = hp.current.min(max);
        }
    }
}

/// Sistema: le pozioni e gli altri effetti a tempo scadono.
pub fn tick_effects(time: Res<Time>, mut effects: ResMut<ActiveEffects>) {
    if !effects.0.is_empty() {
        effects.tick(time.delta_secs());
    }
}

/// Sistema (egui): pannello del personaggio, tasto C.
pub fn character_window(
    mut contexts: EguiContexts,
    character: Res<Character>,
    load: Res<Load>,
    effects: Res<ActiveEffects>,
    players: Query<&PlayerHealth, With<Player>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgba_unmultiplied(10, 12, 20, 250))
        .stroke(egui::Stroke::new(2.0, crate::ui_style::GOLD))
        .inner_margin(egui::Margin::symmetric(24, 16))
        .corner_radius(egui::CornerRadius::same(10));
    let mut close = false;
    egui::Area::new(egui::Id::new("character_panel"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.set_width(520.0_f32.min(ctx.viewport_rect().width() - 80.0));
                ui.label(
                    egui::RichText::new(&character.name)
                        .strong()
                        .size(26.0)
                        .color(crate::ui_style::GOLD),
                );
                ui.label(character.class.name());
                if let Ok(hp) = players.single() {
                    ui.label(format!("Vita: {:.0} / {:.0}", hp.current, hp.max));
                }
                ui.separator();
                for stat in Stat::ALL {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [130.0, 22.0],
                            egui::Label::new(egui::RichText::new(stat.name()).strong()),
                        );
                        ui.add_sized(
                            [34.0, 22.0],
                            egui::Label::new(character.stats.get(stat).to_string()),
                        );
                        ui.label(
                            egui::RichText::new(stat.effect())
                                .size(14.0)
                                .color(egui::Color32::from_rgb(170, 178, 196)),
                        );
                    });
                }
                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "Peso: {:.1} / {:.1} kg{}",
                        load.weight,
                        load.capacity,
                        if load.overloaded {
                            " — sovraccarico"
                        } else {
                            ""
                        }
                    ))
                    .strong(),
                );
                let c = load.carry;
                let mut part = |name: &str, kg: f32| {
                    if kg != 0.0 {
                        ui.label(format!("  {name}: {kg:+.1} kg"));
                    }
                };
                part("Base", c.base);
                part("Zaino", c.backpack);
                part("Forza", c.strength);
                part("Oggetti incantati", c.enchanted);
                part("Effetti", c.effects);
                for e in &effects.0 {
                    ui.label(match e.remaining {
                        Some(t) => format!("  • {} ({t:.0} s)", e.source),
                        None => format!("  • {}", e.source),
                    });
                }
                ui.add_space(8.0);
                if ui.button("Chiudi").clicked() {
                    close = true;
                }
            });
        });
    if close {
        next.set(AppState::Playing);
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Character>()
        .init_resource::<ActiveEffects>()
        .init_resource::<Load>()
        .add_systems(
            Update,
            (
                update_load,
                apply_stats,
                tick_effects.run_if(in_state(AppState::Playing)),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ItemKind;

    fn stats(stat: Stat, v: u8) -> Stats {
        let mut s = Stats::default();
        s.set(stat, v);
        s
    }

    #[test]
    fn carry_is_30_and_60_with_the_biggest_backpack() {
        let none = ActiveEffects::default();
        let equip = Equipment::default();
        assert_eq!(Carry::of(Stats::default(), &equip, &none).total(), 30.0);
        let equip = Equipment {
            backpack: Some(ItemKind::ExpeditionPack),
            ..Default::default()
        };
        assert_eq!(Carry::of(Stats::default(), &equip, &none).total(), 60.0);
    }

    #[test]
    fn every_source_adds_up() {
        let mut fx = ActiveEffects::default();
        fx.add(Effect {
            source: "Pozione".into(),
            carry_kg: 20.0,
            remaining: Some(10.0),
        });
        let equip = Equipment {
            backpack: Some(ItemKind::CanvasPack),
            charm: Some(ItemKind::CarryBelt),
            ..Default::default()
        };
        let c = Carry::of(stats(Stat::Strength, 15), &equip, &fx);
        assert_eq!(c.total(), 30.0 + 10.0 + 5.0 + 10.0 + 20.0);
    }

    #[test]
    fn carry_never_drops_below_the_minimum() {
        let c = Carry::of(
            stats(Stat::Strength, 1),
            &Equipment::default(),
            &ActiveEffects(vec![Effect {
                source: "Maledizione".into(),
                carry_kg: -50.0,
                remaining: None,
            }]),
        );
        assert_eq!(c.total(), MIN_CARRY_KG);
    }

    #[test]
    fn effects_expire_and_the_same_source_renews() {
        let mut fx = ActiveEffects::default();
        for _ in 0..2 {
            fx.add(Effect {
                source: "Pozione".into(),
                carry_kg: 20.0,
                remaining: Some(5.0),
            });
        }
        assert_eq!(fx.carry_kg(), 20.0);
        fx.tick(4.0);
        assert_eq!(fx.carry_kg(), 20.0);
        fx.tick(1.5);
        assert_eq!(fx.carry_kg(), 0.0);
        assert!(fx.0.is_empty());
    }

    #[test]
    fn stats_change_what_they_should() {
        assert_eq!(stats(Stat::Constitution, 14).max_health(), 120.0);
        assert!((stats(Stat::Strength, 15).melee_factor() - 1.2).abs() < 1e-5);
        assert!((stats(Stat::Agility, 5).speed_factor() - 0.95).abs() < 1e-5);
        assert_eq!(stats(Stat::Charisma, 15).price(100), 90);
        assert_eq!(stats(Stat::Charisma, 1).price(100), 118);
        assert_eq!(stats(Stat::Charisma, 20).price(1), 1);
        assert_eq!(stats(Stat::Luck, 14).enemy_coins(), 6);
        assert_eq!(stats(Stat::Luck, 1).enemy_coins(), 1);
        assert_eq!(Stats::default().enemy_coins(), 4);
    }

    #[test]
    fn stat_names_are_found_without_accents() {
        assert_eq!(Stat::parse("AGILITA"), Some(Stat::Agility));
        assert_eq!(Stat::parse("forza"), Some(Stat::Strength));
        assert_eq!(Stat::parse("destrezza"), None);
    }

    #[test]
    fn values_stay_in_range() {
        let mut s = Stats::default();
        s.set(Stat::Luck, 99);
        assert_eq!(s.get(Stat::Luck), STAT_MAX);
        s.set(Stat::Luck, 0);
        assert_eq!(s.get(Stat::Luck), STAT_MIN);
    }

    #[test]
    fn classes_spend_a_comparable_budget() {
        for class in [
            Class::Warrior,
            Class::Explorer,
            Class::Scholar,
            Class::Merchant,
        ] {
            let total: u32 = Stat::ALL
                .iter()
                .map(|s| u32::from(class.stats().get(*s)))
                .sum();
            assert_eq!(total, 70, "{}", class.name());
        }
        let custom: u32 = Stat::ALL
            .iter()
            .map(|s| u32::from(Class::Custom.stats().get(*s)))
            .sum();
        assert_eq!(custom + u32::from(CUSTOM_POINTS), 70);
    }
}
