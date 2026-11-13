//! Scasso delle serrature e borseggio (ADR 0039).
//!
//! - **Scasso**: i bauli sono chiusi a chiave finché non li si apre una prima volta. Con un
//!   grimaldello si gioca il gioco dei perni ([`AppState::Lockpick`]): un ago corre avanti e
//!   indietro, e va fermato sulla tacca d'oro una volta per ogni perno. Uno sbaglio fa ricadere
//!   un perno e può spezzare il grimaldello.
//! - **Borseggio**: alle spalle di un abitante, `F` gli sfila qualche moneta e `G` gliene infila
//!   cinque di nascosto (il «borseggio inverso»). Se va male se ne accorge: multa e diffidenza
//!   fino al giorno dopo.
//!
//! La riuscita dipende dal grado dell'abilità; i tiri sono deterministici (stesso tentativo,
//! stesso esito) così le prove automatiche non ballano.

use std::collections::{BTreeMap, HashMap};

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::AppState;
use crate::containers::{Container, ContainerKind, Containers};
use crate::inventory::{Inventory, ItemKind};
use crate::skills::{self, Skill};

/// Monete infilate a ogni pressione di `G` e multa quando si è scoperti.
pub const SLIP_COINS: u32 = 5;
pub const FINE: u32 = 10;
/// Distanza (m) entro cui si arriva alle tasche di qualcuno.
const POCKET_RANGE: f32 = 1.7;

/// Ciò che il personaggio ha combinato con serrature e tasche.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct Thievery {
    /// Serrature aperte col grimaldello.
    pub picked: u32,
    /// Tentativi fatti finora (seme dei tiri).
    pub attempts: u32,
    /// Monete infilate di nascosto a ciascuno (nome in minuscolo).
    pub slipped: BTreeMap<String, u32>,
    /// Giorno in cui la tasca di qualcuno è stata vuotata, e in cui si è fatto scoprire.
    emptied: HashMap<String, u32>,
    wary: HashMap<String, u32>,
}

impl Thievery {
    /// Contatori per le condizioni delle missioni (`quando: conta <chiave> <n>`).
    pub fn counters(&self) -> Vec<(String, u32)> {
        let mut out = vec![("serrature".to_string(), self.picked)];
        out.extend(
            self.slipped
                .iter()
                .map(|(name, n)| (format!("infilate:{name}"), *n)),
        );
        out
    }

    fn next_roll(&mut self, key: &str) -> f32 {
        self.attempts += 1;
        crate::containers::roll(key, self.attempts)
    }
}

// --- Scasso ------------------------------------------------------------------------------

/// Durezza della serratura di un baule: 0 = aperta, poi 1 semplice, 2 robusta, 3 antica.
pub fn lock_grade(kind: ContainerKind, building: &str) -> u8 {
    if kind != ContainerKind::Chest {
        return 0;
    }
    match building {
        // Il baule del Viandante è quello di casa: lo tiene aperto per l'ospite.
        "casetta" => 0,
        "caserma" => 2,
        "grotta" => 3,
        _ => 1,
    }
}

pub fn lock_name(grade: u8) -> &'static str {
    match grade {
        0 => "aperta",
        1 => "semplice",
        2 => "robusta",
        _ => "antica",
    }
}

/// Larghezza della tacca (frazione della corsa dell'ago): cresce col grado, cala con la durezza.
pub fn zone_width(lock: u8, grade: u8, perk: f32) -> f32 {
    let base = 0.24 - 0.045 * f32::from(lock) + 0.0016 * f32::from(grade);
    (base * (1.0 + 0.1 * perk)).clamp(0.06, 0.45)
}

/// Probabilità che il grimaldello si spezzi a uno sbaglio.
pub fn break_chance(lock: u8, grade: u8) -> f32 {
    (0.35 + 0.1 * f32::from(lock) - 0.006 * f32::from(grade)).clamp(0.05, 0.9)
}

/// Esito di un colpo sul perno.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Push {
    /// Perno a posto; ne restano altri.
    Pin,
    /// Ultimo perno: la serratura cede.
    Open,
    /// Sbagliato: un perno ricade.
    Slip,
    /// Sbagliato e il grimaldello si spezza.
    Broke,
}

/// Serratura su cui si sta lavorando.
#[derive(Resource, Default, Debug)]
pub struct Lockpicking {
    pub key: String,
    pub building: String,
    pub lock: u8,
    pub pins: u8,
    pub done: u8,
    /// Posizione dell'ago (0..1) e verso.
    pub needle: f32,
    dir: f32,
    /// Inizio della tacca (0..1-larghezza).
    pub zone: f32,
    pub width: f32,
    pub message: String,
    /// Il pilota automatico spinge da solo quando l'ago è sulla tacca.
    pub auto: bool,
}

impl Lockpicking {
    pub fn begin(&mut self, c: &Container, lock: u8, grade: u8) {
        *self = Self {
            key: c.key.clone(),
            building: c.building.clone(),
            lock,
            pins: 2 + lock,
            done: 0,
            needle: 0.0,
            dir: 1.0,
            zone: 0.0,
            width: zone_width(lock, grade, skills::rank(Skill::Lockpick)),
            message: String::new(),
            auto: false,
        };
        self.place_zone();
    }

    fn place_zone(&mut self) {
        let r = crate::containers::roll(&self.key, 100 + u32::from(self.done));
        self.zone = r * (1.0 - self.width);
    }

    pub fn on_zone(&self) -> bool {
        (self.zone..=self.zone + self.width).contains(&self.needle)
    }

    /// L'ago corre avanti e indietro, più svelto sulle serrature dure.
    pub fn sweep(&mut self, dt: f32) {
        let speed = 0.55 + 0.2 * f32::from(self.lock);
        self.needle += self.dir * speed * dt;
        if self.needle >= 1.0 {
            self.needle = 1.0;
            self.dir = -1.0;
        } else if self.needle <= 0.0 {
            self.needle = 0.0;
            self.dir = 1.0;
        }
    }

    /// Un colpo sul perno. `luck` (0..1) decide se, sbagliando, il grimaldello si spezza.
    pub fn push(&mut self, grade: u8, luck: f32) -> Push {
        if self.on_zone() {
            self.done += 1;
            if self.done >= self.pins {
                return Push::Open;
            }
            self.place_zone();
            return Push::Pin;
        }
        self.done = self.done.saturating_sub(1);
        self.place_zone();
        if luck < break_chance(self.lock, grade) {
            Push::Broke
        } else {
            Push::Slip
        }
    }
}

/// Apre un mobile, o avvia lo scasso se è un baule chiuso a chiave. Rende lo stato in cui andare
/// (`None`: resta in gioco, con un avviso).
pub fn open_or_pick(
    c: &Container,
    containers: &mut Containers,
    pick: &mut Lockpicking,
    inventory: &Inventory,
    session: &mut crate::save::Session,
    grade: u8,
) -> Option<AppState> {
    let lock = lock_grade(c.kind, &c.building);
    if lock == 0 || containers.was_opened(&c.key) || inventory.qty(ItemKind::MasterKey) > 0 {
        containers.open(c);
        return Some(AppState::Container);
    }
    if inventory.qty(ItemKind::Lockpick) == 0 {
        session.notify("È chiuso a chiave: serve un grimaldello");
        info!("[SCASSO] {}: chiuso, nessun grimaldello", c.key);
        return None;
    }
    pick.begin(c, lock, grade);
    info!(
        "[SCASSO] {}: serratura {} ({} perni)",
        c.key,
        lock_name(lock),
        pick.pins
    );
    Some(AppState::Lockpick)
}

/// Ciò che serve a `hut::handle_prop_toggle` per aprire o forzare un mobile.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PickParams<'w> {
    pick: ResMut<'w, Lockpicking>,
    session: ResMut<'w, crate::save::Session>,
    progress: Res<'w, skills::Progress>,
}

impl PickParams<'_> {
    pub fn open(
        &mut self,
        c: &Container,
        containers: &mut Containers,
        inventory: &Inventory,
    ) -> Option<AppState> {
        let grade = self.progress.grade(Skill::Lockpick);
        open_or_pick(
            c,
            containers,
            &mut self.pick,
            inventory,
            &mut self.session,
            grade,
        )
    }
}

/// Sistema (stato `Lockpick`): l'ago, i colpi sul perno e la finestra.
#[allow(clippy::too_many_arguments)]
pub fn lockpick_window(
    mut contexts: EguiContexts,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut pick: ResMut<Lockpicking>,
    mut thievery: ResMut<Thievery>,
    mut containers: ResMut<Containers>,
    mut inventory: ResMut<Inventory>,
    progress: Res<skills::Progress>,
    mut session: ResMut<crate::save::Session>,
    mut next: ResMut<NextState<AppState>>,
) {
    pick.sweep(time.delta_secs());
    let centred = (pick.needle - (pick.zone + pick.width / 2.0)).abs() < pick.width * 0.3;
    let pushed = keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::KeyE)
        || mouse.just_pressed(MouseButton::Left)
        || (pick.auto && centred);
    if pushed {
        let grade = progress.grade(Skill::Lockpick);
        let luck = thievery.next_roll(&pick.key);
        let lock = f32::from(pick.lock);
        match pick.push(grade, luck) {
            Push::Pin => {
                skills::train(Skill::Lockpick, 3.0 * lock);
                pick.message = "Un perno è a posto.".into();
            }
            Push::Open => {
                skills::train(Skill::Lockpick, 10.0 * lock);
                thievery.picked += 1;
                info!("[SCASSO] {}: aperta", pick.key);
                containers.open(&Container {
                    key: pick.key.clone(),
                    kind: ContainerKind::Chest,
                    building: pick.building.clone(),
                });
                session.notify("La serratura cede");
                next.set(AppState::Container);
                return;
            }
            Push::Slip => {
                skills::train(Skill::Lockpick, 1.0);
                pick.message = "Il perno ricade.".into();
            }
            Push::Broke => {
                skills::train(Skill::Lockpick, 1.0);
                inventory.consume(ItemKind::Lockpick, 1);
                info!("[SCASSO] {}: grimaldello spezzato", pick.key);
                if inventory.qty(ItemKind::Lockpick) == 0 {
                    session.notify("L'ultimo grimaldello si è spezzato");
                    next.set(AppState::Playing);
                    return;
                }
                pick.message = "Il grimaldello si spezza!".into();
            }
        }
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let frame = egui::Frame::default()
        .fill(crate::ui_style::PANEL_FILL)
        .stroke(egui::Stroke::new(2.0, crate::ui_style::GOLD))
        .inner_margin(egui::Margin::symmetric(24, 16))
        .corner_radius(egui::CornerRadius::same(10));
    let mut close = false;
    egui::Area::new(egui::Id::new("lockpick_panel"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.set_width(460.0);
                ui.label(
                    egui::RichText::new(format!("Serratura {}", lock_name(pick.lock)))
                        .strong()
                        .size(24.0)
                        .color(crate::ui_style::GOLD),
                );
                ui.label(format!(
                    "Perni: {} di {} · Grimaldelli: {}",
                    pick.done,
                    pick.pins,
                    inventory.qty(ItemKind::Lockpick)
                ));
                ui.add_space(8.0);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(460.0, 34.0), egui::Sense::hover());
                let p = ui.painter();
                p.rect_filled(
                    rect,
                    egui::CornerRadius::same(5),
                    egui::Color32::from_rgb(28, 32, 44),
                );
                let x = |f: f32| rect.min.x + rect.width() * f;
                let zone = egui::Rect::from_min_max(
                    egui::pos2(x(pick.zone), rect.min.y),
                    egui::pos2(x(pick.zone + pick.width), rect.max.y),
                );
                p.rect_filled(
                    zone,
                    egui::CornerRadius::same(3),
                    crate::ui_style::GOLD.linear_multiply(0.75),
                );
                p.rect_stroke(
                    rect,
                    egui::CornerRadius::same(5),
                    egui::Stroke::new(1.0, crate::ui_style::GOLD_EDGE),
                    egui::StrokeKind::Inside,
                );
                p.line_segment(
                    [
                        egui::pos2(x(pick.needle), rect.min.y - 4.0),
                        egui::pos2(x(pick.needle), rect.max.y + 4.0),
                    ],
                    egui::Stroke::new(3.0, crate::ui_style::TEXT),
                );
                ui.add_space(8.0);
                ui.label(&pick.message);
                ui.label(
                    egui::RichText::new(
                        "[Spazio] spingi il perno quando l'ago è sulla tacca · [Esc] lascia",
                    )
                    .size(14.0)
                    .color(egui::Color32::from_rgb(170, 178, 196)),
                );
                if ui.button("Lascia").clicked() {
                    close = true;
                }
            });
        });
    if close {
        next.set(AppState::Playing);
    }
}

// --- Borseggio ---------------------------------------------------------------------------

/// Probabilità di non farsi scoprire. Infilare è più facile che sfilare; di notte aiuta il buio.
pub fn pocket_chance(grade: u8, perk: f32, agility: u8, luck: u8, night: bool, slip: bool) -> f32 {
    let mut p = 0.30 + 0.007 * f32::from(grade) + 0.05 * perk;
    p += 0.01 * (f32::from(agility) - 10.0) + 0.01 * (f32::from(luck) - 10.0);
    if night {
        p += 0.10;
    }
    if slip {
        p += 0.20;
    }
    p.clamp(0.05, 1.0)
}

/// Monete nella tasca di qualcuno, oggi.
pub fn pocket_coins(name: &str, day: u32) -> u32 {
    2 + (crate::containers::roll(name, day) * 7.0) as u32
}

/// Si è alle spalle di chi guarda verso `forward`?
pub fn behind(npc: Vec3, forward: Vec3, player: Vec3) -> bool {
    let to = (player - npc).with_y(0.0);
    to.length() <= POCKET_RANGE && to.normalize_or_zero().dot(forward.with_y(0.0)) < -0.2
}

/// Sistema: tasche degli abitanti. `F` sfila, `G` infila; solo alle spalle e a un passo.
#[allow(clippy::too_many_arguments)]
pub fn pickpocket(
    keys: Res<ButtonInput<KeyCode>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    npcs: Query<(Entity, &GlobalTransform, &crate::entities::Npc)>,
    dialog: Res<crate::entities::DialogState>,
    clock: Res<crate::daynight::GameClock>,
    character: Res<crate::character::Character>,
    progress: Res<skills::Progress>,
    mut thievery: ResMut<Thievery>,
    mut inventory: ResMut<Inventory>,
    mut nearest: ResMut<crate::entities::NearestInteract>,
    mut barks: ResMut<crate::quests::Barks>,
    mut session: ResMut<crate::save::Session>,
) {
    if dialog.open {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();
    // Il modello degli abitanti guarda verso +Z locale.
    let target = npcs
        .iter()
        .filter(|(_, t, _)| behind(t.translation(), t.rotation() * Vec3::Z, p))
        .min_by(|a, b| {
            a.1.translation()
                .distance(p)
                .total_cmp(&b.1.translation().distance(p))
        });
    let Some((entity, _, npc)) = target else {
        return;
    };
    let name = npc.name.to_lowercase();
    let day = u32::from(clock.date.month) * 100 + u32::from(clock.date.day);
    let wary = thievery.wary.get(&name) == Some(&day);
    if wary {
        return;
    }
    let coins = inventory.qty(ItemKind::Coin);
    let can_slip = coins >= SLIP_COINS;
    let hint = if can_slip {
        format!("[F] Borseggia · [G] Infila {SLIP_COINS} monete")
    } else {
        "[F] Borseggia".to_string()
    };
    nearest.prompt = Some(match nearest.prompt.take() {
        Some(prompt) => format!("{prompt} · {hint}"),
        None => hint,
    });
    let steal = keys.just_pressed(KeyCode::KeyF);
    let slip = keys.just_pressed(KeyCode::KeyG) && can_slip;
    if !steal && !slip {
        return;
    }
    let night = !(5.0..21.0).contains(&clock.hour);
    let chance = pocket_chance(
        progress.grade(Skill::Pickpocket),
        skills::rank(Skill::Pickpocket),
        character.stats.get(crate::character::Stat::Agility),
        character.stats.get(crate::character::Stat::Luck),
        night,
        slip,
    );
    if thievery.next_roll(&name) >= chance {
        let fine = FINE.min(coins);
        inventory.consume(ItemKind::Coin, fine);
        thievery.wary.insert(name.clone(), day);
        skills::train(Skill::Pickpocket, 3.0);
        barks.say(
            entity,
            npc.name,
            "Ehi! Giù le mani dalle mie tasche!".into(),
        );
        session.notify(if fine > 0 {
            format!("{} se n'è accorto: {fine} monete di multa", npc.name)
        } else {
            format!("{} se n'è accorto", npc.name)
        });
        info!("[BORSEGGIO] {}: scoperto (multa {fine})", npc.name);
        return;
    }
    if slip {
        inventory.consume(ItemKind::Coin, SLIP_COINS);
        *thievery.slipped.entry(name).or_default() += SLIP_COINS;
        skills::train(Skill::Pickpocket, 10.0);
        session.notify(format!(
            "Infili {SLIP_COINS} monete nella tasca di {}",
            npc.name
        ));
        info!("[BORSEGGIO] {}: infilate {SLIP_COINS} monete", npc.name);
    } else if thievery.emptied.get(&name) == Some(&day) {
        skills::train(Skill::Pickpocket, 2.0);
        session.notify(format!("Le tasche di {} sono vuote", npc.name));
    } else {
        let loot = pocket_coins(&name, day);
        inventory.add(ItemKind::Coin, loot);
        thievery.emptied.insert(name, day);
        skills::train(Skill::Pickpocket, 12.0);
        session.notify(format!("Sfili {loot} monete a {}", npc.name));
        info!("[BORSEGGIO] {}: sfilate {loot} monete", npc.name);
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Thievery>()
        .init_resource::<Lockpicking>();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chest(building: &str) -> Container {
        Container {
            key: "prova:baule".into(),
            kind: ContainerKind::Chest,
            building: building.into(),
        }
    }

    #[test]
    fn only_chests_are_locked_and_not_the_wanderers() {
        assert_eq!(lock_grade(ContainerKind::Crate, "caserma"), 0);
        assert_eq!(lock_grade(ContainerKind::Chest, "casetta"), 0);
        assert_eq!(lock_grade(ContainerKind::Chest, "casa"), 1);
        assert_eq!(lock_grade(ContainerKind::Chest, "caserma"), 2);
        assert_eq!(lock_grade(ContainerKind::Chest, "grotta"), 3);
    }

    #[test]
    fn skill_widens_the_notch_and_saves_the_picks() {
        assert!(zone_width(1, 50, 0.0) > zone_width(1, 1, 0.0));
        assert!(zone_width(3, 1, 0.0) < zone_width(1, 1, 0.0));
        assert!(zone_width(1, 1, 3.0) > zone_width(1, 1, 0.0));
        assert!(break_chance(1, 100) < break_chance(1, 1));
        assert!(break_chance(3, 1) > break_chance(1, 1));
    }

    #[test]
    fn a_lock_opens_one_pin_at_a_time() {
        let mut pick = Lockpicking::default();
        pick.begin(&chest("casa"), 1, 1);
        assert_eq!((pick.pins, pick.done), (3, 0));
        // Fuori dalla tacca: sbagliato (con fortuna alta il grimaldello regge).
        pick.needle = if pick.zone > 0.5 { 0.0 } else { 1.0 };
        assert_eq!(pick.push(1, 0.99), Push::Slip);
        assert_eq!(pick.push(1, 0.0), Push::Broke);
        for expected in [Push::Pin, Push::Pin, Push::Open] {
            pick.needle = pick.zone + pick.width / 2.0;
            assert_eq!(pick.push(1, 0.5), expected);
        }
    }

    #[test]
    fn the_needle_sweeps_back_and_forth() {
        let mut pick = Lockpicking::default();
        pick.begin(&chest("casa"), 1, 1);
        let mut seen = (1.0_f32, 0.0_f32);
        for _ in 0..400 {
            pick.sweep(0.02);
            seen = (seen.0.min(pick.needle), seen.1.max(pick.needle));
            assert!((0.0..=1.0).contains(&pick.needle));
        }
        assert_eq!(seen, (0.0, 1.0));
    }

    #[test]
    fn pockets_are_reached_only_from_behind() {
        let npc = Vec3::ZERO;
        assert!(behind(npc, Vec3::Z, Vec3::new(0.0, 0.0, -1.0)));
        assert!(!behind(npc, Vec3::Z, Vec3::new(0.0, 0.0, 1.0)));
        assert!(!behind(npc, Vec3::Z, Vec3::new(0.0, 0.0, -3.0)));
    }

    #[test]
    fn practice_and_darkness_help_the_light_hand() {
        let day = pocket_chance(1, 0.0, 10, 10, false, false);
        assert!(pocket_chance(1, 0.0, 10, 10, true, false) > day);
        assert!(pocket_chance(1, 0.0, 10, 10, false, true) > day);
        assert!(pocket_chance(60, 0.0, 10, 10, false, false) > day);
        assert_eq!(pocket_chance(100, 0.0, 10, 10, false, false), 1.0);
        let coins = pocket_coins("beppe", 412);
        assert!((2..=8).contains(&coins));
        assert_eq!(coins, pocket_coins("beppe", 412));
    }

    #[test]
    fn counters_feed_the_missions() {
        let mut t = Thievery {
            picked: 2,
            ..Default::default()
        };
        t.slipped.insert("beppe".into(), 15);
        assert_eq!(
            t.counters(),
            vec![
                ("serrature".to_string(), 2),
                ("infilate:beppe".to_string(), 15)
            ]
        );
    }
}
