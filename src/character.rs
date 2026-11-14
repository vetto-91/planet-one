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

/// Età del personaggio, scelta guardandosi nello «Specchio della Marea» (ADR 0047).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Age {
    /// 18–26 anni: «il passo di Telemaco».
    Young,
    /// 27–45 anni: «il vigore di Odisseo».
    #[default]
    Adult,
    /// 46–70 anni: «la memoria di Nestore».
    Elder,
}

impl Age {
    pub const ALL: [Age; 3] = [Age::Young, Age::Adult, Age::Elder];

    pub fn name(self) -> &'static str {
        match self {
            Age::Young => "Giovane",
            Age::Adult => "Adulto",
            Age::Elder => "Maturo",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Age::Young => "giovane",
            Age::Adult => "adulto",
            Age::Elder => "maturo",
        }
    }

    pub fn parse(text: &str) -> Option<Age> {
        Age::ALL.into_iter().find(|a| a.key() == text.trim())
    }

    /// Ciò che si vede nell'acqua ferma.
    pub fn mirror(self) -> &'static str {
        match self {
            Age::Young => {
                "Un volto giovane, occhi inquieti, mani con più futuro che cicatrici. (18–26 anni: +1 Agilità, −1 Saggezza)"
            }
            Age::Adult => {
                "Lineamenti temprati, lo sguardo di chi ha già attraversato tempeste che non ricorda. (27–45 anni)"
            }
            Age::Elder => {
                "Rughe scavate dal sole e fili d'argento tra i capelli: il corpo ricorda una lunga vita. (46–70 anni: +1 Saggezza, +1 Intelligenza, −1 Agilità, −1 Costituzione)"
            }
        }
    }

    /// Le statistiche di partenza con il segno degli anni.
    pub fn apply(self, mut stats: Stats) -> Stats {
        let mut add = |s: Stat, n: i8| {
            let v = i16::from(stats.get(s)) + i16::from(n);
            stats.set(s, v.clamp(0, 255) as u8);
        };
        match self {
            Age::Young => {
                add(Stat::Agility, 1);
                add(Stat::Wisdom, -1);
            }
            Age::Adult => {}
            Age::Elder => {
                add(Stat::Wisdom, 1);
                add(Stat::Intelligence, 1);
                add(Stat::Agility, -1);
                add(Stat::Constitution, -1);
            }
        }
        stats
    }
}

/// Fasi del risveglio sulla spiaggia: età scelta, classe scelta, nome detto a Gaspare.
pub const AWAKE: u8 = 3;

/// Il personaggio in gioco.
#[derive(Resource, Clone, Debug)]
pub struct Character {
    pub name: String,
    pub class: Class,
    pub stats: Stats,
    pub age: Age,
    /// A che punto è il risveglio sulla spiaggia (0..=[`AWAKE`]); le partite vecchie sono già sveglie.
    pub awakening: u8,
}

impl Default for Character {
    fn default() -> Self {
        Self {
            name: "Viaggiatore".to_string(),
            class: Class::Custom,
            stats: Stats::default(),
            age: Age::Adult,
            awakening: AWAKE,
        }
    }
}

impl Character {
    /// Il naufrago appena arrivato: senza nome, senza età né indole (le sceglie sulla spiaggia).
    pub fn castaway() -> Self {
        Self {
            name: "Nessuno".to_string(),
            awakening: 0,
            ..Default::default()
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
    /// Talento «Spalle d'Atlante» (ADR 0038).
    pub skill: f32,
}

impl Carry {
    pub fn of(stats: Stats, equip: &Equipment, effects: &ActiveEffects) -> Self {
        Self {
            base: PLAYER_CARRY_KG,
            backpack: equip.backpack.and_then(|b| b.backpack_kg()).unwrap_or(0.0),
            strength: stats.strength_carry(),
            enchanted: equip.charm.and_then(|c| c.charm_kg()).unwrap_or(0.0),
            effects: effects.carry_kg(),
            skill: 3.0 * crate::skills::rank(crate::skills::Skill::Carrying),
        }
    }

    pub fn total(self) -> f32 {
        (self.base + self.backpack + self.strength + self.enchanted + self.effects + self.skill)
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
        // L'armatura a piastre pesa anche sul passo (ADR 0049).
        speed_factor: character.stats.speed_factor()
            * if equip.armor == Some(crate::inventory::ItemKind::PlateArmor) {
                0.9
            } else {
                1.0
            },
    };
}

/// Sistema: la vita massima segue la costituzione.
pub fn apply_stats(
    character: Res<Character>,
    progress: Res<crate::skills::Progress>,
    mut players: Query<&mut PlayerHealth, With<Player>>,
) {
    let max = character.stats.max_health() + progress.health_bonus();
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

/// Sistema (egui): pannello del personaggio, tasto C. A sinistra statistiche e peso, a destra
/// livello, abilità e talenti (ADR 0038).
pub fn character_window(
    mut contexts: EguiContexts,
    mut character: ResMut<Character>,
    mut progress: ResMut<crate::skills::Progress>,
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
    let grey = egui::Color32::from_rgb(170, 178, 196);
    let view = ctx.viewport_rect();
    let mut close = false;
    let mut raise: Option<Stat> = None;
    let mut activate: Option<crate::skills::Skill> = None;
    egui::Area::new(egui::Id::new("character_panel"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.set_width(1000.0_f32.min(view.width() - 80.0));
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&character.name)
                            .strong()
                            .size(26.0)
                            .color(crate::ui_style::GOLD),
                    );
                    ui.label(format!(
                        "{} · livello {}",
                        character.class.name(),
                        progress.level
                    ));
                    ui.add(
                        egui::ProgressBar::new(progress.level_fraction())
                            .desired_width(160.0)
                            .desired_height(10.0),
                    );
                });
                if let Ok(hp) = players.single() {
                    ui.label(format!("Vita: {:.0} / {:.0}", hp.current, hp.max));
                }
                ui.separator();
                ui.columns(2, |cols| {
                    {
                        let ui = &mut cols[0];
                        if progress.stat_points > 0 {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Punti statistica da spendere: {}",
                                    progress.stat_points
                                ))
                                .color(crate::ui_style::GOLD),
                            );
                        }
                        for stat in Stat::ALL {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [120.0, 22.0],
                                    egui::Label::new(egui::RichText::new(stat.name()).strong()),
                                );
                                ui.add_sized(
                                    [30.0, 22.0],
                                    egui::Label::new(character.stats.get(stat).to_string()),
                                );
                                if progress.stat_points > 0
                                    && character.stats.get(stat) < STAT_MAX
                                    && ui.small_button("+").clicked()
                                {
                                    raise = Some(stat);
                                }
                                ui.label(egui::RichText::new(stat.effect()).size(14.0).color(grey));
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
                        part("Tempra", c.skill);
                        for e in &effects.0 {
                            ui.label(match e.remaining {
                                Some(t) => format!("  • {} ({t:.0} s)", e.source),
                                None => format!("  • {}", e.source),
                            });
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Incantesimi — [Z] scegli · [Q] lancia").strong(),
                        );
                        let magic_grade = progress.grade(crate::skills::Skill::Magic);
                        for spell in crate::magic::Spell::ALL {
                            let known = magic_grade >= spell.grade()
                                && (spell.basic() || crate::magic::taught());
                            let text = if known {
                                format!(
                                    "  {} ({:.0}): {}",
                                    spell.name(),
                                    spell.cost(),
                                    spell.effect()
                                )
                            } else {
                                format!(
                                    "  {} — al grado {} di Magie{}",
                                    spell.name(),
                                    spell.grade(),
                                    if spell.basic() {
                                        ""
                                    } else {
                                        ", insegnato da Agnese"
                                    }
                                )
                            };
                            ui.label(egui::RichText::new(text).size(14.0).color(if known {
                                crate::ui_style::TEXT
                            } else {
                                grey
                            }));
                        }
                    }
                    {
                        let ui = &mut cols[1];
                        ui.label(
                            egui::RichText::new(format!(
                                "Abilità — Nodi d'Oro: {}",
                                progress.nodes
                            ))
                            .strong()
                            .color(crate::ui_style::GOLD),
                        );
                        egui::ScrollArea::vertical()
                            .max_height((view.height() - 260.0).max(200.0))
                            .show(ui, |ui| {
                                for skill in crate::skills::Skill::ALL {
                                    skill_row(ui, skill, &progress, grey, &mut activate);
                                }
                            });
                    }
                });
                ui.add_space(8.0);
                if ui.button("Chiudi").clicked() {
                    close = true;
                }
            });
        });
    if let Some(stat) = raise {
        let value = character.stats.get(stat);
        character.stats.set(stat, value + 1);
        progress.stat_points -= 1;
        info!("[ABILITA] {} sale a {}", stat.name(), value + 1);
    }
    if let Some(skill) = activate
        && progress.activate(skill)
    {
        info!(
            "[ABILITA] talento di {}: rango {}",
            skill.name(),
            progress.perk_rank(skill)
        );
    }
    if close {
        next.set(AppState::Playing);
    }
}

/// Una riga del pannello: abilità, grado, avanzamento e talento.
fn skill_row(
    ui: &mut egui::Ui,
    skill: crate::skills::Skill,
    progress: &crate::skills::Progress,
    grey: egui::Color32,
    activate: &mut Option<crate::skills::Skill>,
) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [200.0, 20.0],
            egui::Label::new(egui::RichText::new(skill.name()).strong()),
        );
        ui.add_sized(
            [30.0, 20.0],
            egui::Label::new(progress.grade(skill).to_string()),
        );
        ui.add(
            egui::ProgressBar::new(progress.grade_fraction(skill))
                .desired_width(90.0)
                .desired_height(8.0),
        );
        if let Some((_, name)) = progress.next_rank(skill)
            && ui.small_button(format!("Attiva {name}")).clicked()
        {
            *activate = Some(skill);
        }
    });
    let detail = match skill.perk() {
        Some((name, effect)) => {
            let rank = progress.perk_rank(skill);
            let next = crate::skills::RANKS
                .get(usize::from(rank))
                .map_or(String::new(), |(g, _)| format!(" · prossimo al grado {g}"));
            format!(
                "   {name} {rank}/5 ({effect} per rango){next} — cresce {}",
                skill.how()
            )
        }
        None => "   non ancora nel gioco".to_string(),
    };
    ui.label(egui::RichText::new(detail).size(13.0).color(grey));
    // Talenti attivi (ADR 0049): sbloccati dal grado di Apprendista.
    for (owner, name, what) in crate::skills::ACTIVE_TALENTS {
        if owner == skill {
            let open = progress.grade(skill) >= crate::skills::ACTIVE_TALENT;
            let text = if open {
                format!("   ★ {name}: {what}")
            } else {
                format!("   {name} — al grado {}", crate::skills::ACTIVE_TALENT)
            };
            ui.label(egui::RichText::new(text).size(13.0).color(if open {
                crate::ui_style::GOLD
            } else {
                grey
            }));
        }
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
