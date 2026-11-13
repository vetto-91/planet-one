//! Karma e segreti degli abitanti (ADR 0040).
//!
//! - **Karma** («la Bilancia di Ma'at»): un numero solo per l'impronta del personaggio nel mondo.
//!   Sale con la pietà, la parola mantenuta e l'aiuto dato; scende con i furti e l'avidità.
//! - **Fiducia**: un numero per ogni abitante. Sale parlandogli (una volta al giorno), facendogli
//!   un regalo gradito, comprando da lui, aiutandolo nelle missioni; scende se lo si deruba.
//! - **Segreti**: ogni abitante ne ha uno (`assets/world/segreti.txt`). Lo confida quando la
//!   fiducia arriva ad «Amico», o un po' prima a chi ha occhio (Carisma, Saggezza, Intelligenza).
//!
//! Chi compie un'azione non deve conoscere la risorsa: [`deed`] e [`trust`] scrivono in una coda
//! globale, svuotata una volta per frame da [`apply_deeds`].

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Mutex;

use bevy::prelude::*;

use crate::inventory::ItemKind;

pub const ACQUAINTANCE: i32 = 10;
pub const FRIEND: i32 = 30;
pub const KINDRED: i32 = 60;
pub const TRUST_MIN: i32 = -50;
pub const TRUST_MAX: i32 = 100;
/// Soglia della confidenza per chi ha occhio (una fra Carisma, Saggezza, Intelligenza a 14).
pub const INSIGHT_TRUST: i32 = 22;
pub const INSIGHT_STAT: u8 = 14;
/// Fiducia guadagnata: prima chiacchierata del giorno, regalo gradito, tappa di missione, acquisto.
pub const TALK_TRUST: i32 = 3;
pub const GIFT_TRUST: i32 = 6;
pub const MISSION_TRUST: i32 = 5;
pub const TRADE_TRUST: i32 = 1;
const GIFT_RANGE: f32 = 2.2;

/// Il legame con un abitante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bond {
    Wary,
    Stranger,
    Acquaintance,
    Friend,
    Kindred,
}

impl Bond {
    pub fn of(trust: i32) -> Bond {
        match trust {
            t if t < 0 => Bond::Wary,
            t if t < ACQUAINTANCE => Bond::Stranger,
            t if t < FRIEND => Bond::Acquaintance,
            t if t < KINDRED => Bond::Friend,
            _ => Bond::Kindred,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Bond::Wary => "Diffidente",
            Bond::Stranger => "Sconosciuto",
            Bond::Acquaintance => "Conoscente",
            Bond::Friend => "Amico",
            Bond::Kindred => "Anima affine",
        }
    }
}

/// Come pesa il cuore sulla bilancia.
pub fn heart(global: i32) -> &'static str {
    match global {
        g if g >= 40 => "Cuore leggero come la piuma",
        g if g >= 15 => "Cuore leggero",
        g if g > -15 => "Cuore in bilico",
        g if g > -40 => "Cuore pesante",
        _ => "Cuore pesante come il piombo",
    }
}

/// Il segreto di un abitante, dal file di testo.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Secret {
    pub name: String,
    /// Che cosa apprezza.
    pub dear: String,
    pub gift: Option<ItemKind>,
    /// Riassunto per il diario.
    pub summary: String,
    /// La confidenza (segnaposto `{nome}`).
    pub lines: Vec<String>,
}

pub fn parse(text: &str) -> Vec<Secret> {
    let mut out: Vec<Secret> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            out.push(Secret {
                name: name.trim().to_string(),
                ..Default::default()
            });
            continue;
        }
        let Some(s) = out.last_mut() else {
            continue;
        };
        if let Some(said) = line.strip_prefix('>') {
            s.lines.push(said.trim().to_string());
        } else if let Some((key, value)) = line.split_once(':') {
            let value = value.trim();
            match key.trim() {
                "caro" => s.dear = value.to_string(),
                "segreto" => s.summary = value.to_string(),
                "dono" => {
                    s.gift = ItemKind::from_name(value);
                    if s.gift.is_none() {
                        warn!("[KARMA] dono sconosciuto per {}: {value}", s.name);
                    }
                }
                _ => warn!("[KARMA] riga ignorata: {line}"),
            }
        }
    }
    out
}

/// I segreti di tutti gli abitanti.
#[derive(Resource, Debug)]
pub struct Secrets(pub Vec<Secret>);

impl Default for Secrets {
    fn default() -> Self {
        Self(parse(include_str!("../assets/world/segreti.txt")))
    }
}

impl Secrets {
    pub fn of(&self, name: &str) -> Option<&Secret> {
        self.0.iter().find(|s| s.name.eq_ignore_ascii_case(name))
    }
}

/// Karma del personaggio, fiducia degli abitanti e segreti scoperti.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct Karma {
    pub global: i32,
    /// Fiducia per abitante (nome in minuscolo).
    pub trust: BTreeMap<String, i32>,
    /// Abitanti con cui si è parlato almeno una volta e segreti ricevuti (nomi in minuscolo).
    pub met: BTreeSet<String>,
    pub revealed: BTreeSet<String>,
    /// Azioni già pesate una volta per tutte (la sorte del ladro, la consegna delle monete).
    pub judged: BTreeSet<String>,
    /// Giorno dell'ultima chiacchierata e dell'ultimo regalo (non salvati).
    talked: HashMap<String, u32>,
    gifted: HashMap<String, u32>,
}

impl Karma {
    pub fn trust_of(&self, name: &str) -> i32 {
        self.trust.get(&name.to_lowercase()).copied().unwrap_or(0)
    }

    pub fn bond(&self, name: &str) -> Bond {
        Bond::of(self.trust_of(name))
    }

    pub fn add_trust(&mut self, name: &str, amount: i32) {
        let t = self.trust.entry(name.to_lowercase()).or_insert(0);
        *t = (*t + amount).clamp(TRUST_MIN, TRUST_MAX);
    }

    pub fn knows_secret(&self, name: &str) -> bool {
        self.revealed.contains(&name.to_lowercase())
    }

    /// L'abitante è pronto a confidarsi? `insight`: il giocatore ha occhio per i dettagli.
    pub fn ready_to_confide(&self, name: &str, insight: bool) -> bool {
        let need = if insight { INSIGHT_TRUST } else { FRIEND };
        !self.knows_secret(name) && self.trust_of(name) >= need
    }

    /// Fattore sui prezzi di un venditore: il paese fa credito a chi ha il cuore leggero.
    pub fn price_factor(&self, vendor: &str) -> f32 {
        let world = match self.global {
            g if g >= 40 => 0.94,
            g if g >= 15 => 0.97,
            g if g > -15 => 1.0,
            g if g > -40 => 1.08,
            _ => 1.15,
        };
        let friend = match self.bond(vendor) {
            Bond::Wary => 1.10,
            Bond::Friend => 0.95,
            Bond::Kindred => 0.90,
            _ => 1.0,
        };
        world * friend
    }
}

// --- Canale globale --------------------------------------------------------------------------

enum Deed {
    Global(i32, String),
    Trust(String, i32),
}

static QUEUE: Mutex<Vec<Deed>> = Mutex::new(Vec::new());
static PRICES: Mutex<Option<Karma>> = Mutex::new(None);

/// Un'azione che pesa sulla bilancia (positiva o negativa), con il motivo da mostrare.
pub fn deed(amount: i32, why: impl Into<String>) {
    if amount != 0
        && let Ok(mut q) = QUEUE.lock()
    {
        q.push(Deed::Global(amount, why.into()));
    }
}

/// Fiducia guadagnata o persa con un abitante.
pub fn trust(name: &str, amount: i32) {
    if amount != 0
        && let Ok(mut q) = QUEUE.lock()
    {
        q.push(Deed::Trust(name.to_string(), amount));
    }
}

/// Prezzo in negozio: statistiche e talenti, poi karma e fiducia del venditore.
pub fn price(stats: crate::character::Stats, base: u32, vendor: &str) -> u32 {
    let factor = PRICES
        .lock()
        .ok()
        .and_then(|k| k.as_ref().map(|k| k.price_factor(vendor)))
        .unwrap_or(1.0);
    ((crate::skills::price(stats, base) as f32 * factor).round() as u32).max(1)
}

/// Sistema: applica le azioni del frame e avvisa quando la bilancia si muove.
pub fn apply_deeds(mut karma: ResMut<Karma>, mut session: ResMut<crate::save::Session>) {
    let pending = QUEUE
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default();
    for deed in pending {
        match deed {
            Deed::Global(amount, why) => {
                karma.global = (karma.global + amount).clamp(-100, 100);
                info!("[KARMA] {amount:+} ({why}) → {}", karma.global);
                session.notify(format!("Karma {amount:+}: {why}"));
            }
            Deed::Trust(name, amount) => {
                let before = karma.bond(&name);
                karma.add_trust(&name, amount);
                let after = karma.bond(&name);
                info!(
                    "[KARMA] fiducia di {name} {amount:+} → {}",
                    karma.trust_of(&name)
                );
                if after != before {
                    session.notify(format!("{name}: {}", after.name()));
                }
            }
        }
    }
    if karma.is_changed()
        && let Ok(mut p) = PRICES.lock()
    {
        *p = Some(karma.clone());
    }
}

/// Sistema: pesa una volta sola la sorte del ladro e la consegna delle monete (ADR 0035).
pub fn judge_quest(quest: Res<crate::quests::denaro_rubato::QuestState>, mut karma: ResMut<Karma>) {
    use crate::quests::denaro_rubato::{Fate, QuestStage};
    if quest.fate != Fate::Unknown && !karma.judged.contains("ladro") {
        karma.judged.insert("ladro".into());
        match quest.fate {
            Fate::Spared => {
                deed(10, "hai risparmiato il ladro");
                trust("Don Anselmo", 10);
            }
            Fate::Pact => deed(-10, "hai diviso l'oro col ladro"),
            Fate::Killed if quest.thief_calm => deed(-6, "hai colpito chi ti stava ascoltando"),
            _ => {}
        }
    }
    if quest.stage == QuestStage::Completed && !karma.judged.contains("consegna") {
        karma.judged.insert("consegna".into());
        if quest.fate == Fate::Pact {
            trust("Viandante", -10);
        } else if quest.kept > 0 {
            deed(-8, "hai tenuto monete non tue");
            trust("Viandante", -10);
        } else {
            deed(5, "hai restituito tutte le monete");
            trust("Viandante", 12);
        }
    }
}

fn today(clock: &crate::daynight::GameClock) -> u32 {
    u32::from(clock.date.month) * 100 + u32::from(clock.date.day)
}

/// Sistema: la prima chiacchierata del giorno fa crescere la fiducia; a un amico l'abitante
/// confida il suo segreto al posto del saluto.
#[allow(clippy::too_many_arguments)]
pub fn meet_and_confide(
    mut dialog: ResMut<crate::entities::DialogState>,
    missions: Res<crate::quests::missions::Missions>,
    secrets: Res<Secrets>,
    clock: Res<crate::daynight::GameClock>,
    character: Res<crate::character::Character>,
    mut karma: ResMut<Karma>,
    mut session: ResMut<crate::save::Session>,
    mut was_open: Local<bool>,
) {
    let opened = dialog.open && !*was_open;
    *was_open = dialog.open;
    if !opened || dialog.speaker_entity.is_none() {
        return;
    }
    let name = dialog.speaker.clone();
    let key = name.to_lowercase();
    let day = today(&clock);
    karma.met.insert(key.clone());
    if karma.talked.insert(key.clone(), day) != Some(day) {
        trust(&name, TALK_TRUST);
    }
    // Una battuta di missione o una scelta di quest hanno la precedenza sulla confidenza.
    if missions.in_dialog() || !dialog.choices.is_empty() {
        return;
    }
    let Some(secret) = secrets.of(&name) else {
        return;
    };
    use crate::character::Stat;
    let insight = [Stat::Charisma, Stat::Wisdom, Stat::Intelligence]
        .into_iter()
        .any(|s| character.stats.get(s) >= INSIGHT_STAT);
    if secret.lines.is_empty() || !karma.ready_to_confide(&name, insight) {
        return;
    }
    dialog.lines = secret
        .lines
        .iter()
        .map(|l| l.replace("{nome}", &character.name))
        .collect();
    dialog.voices = vec![Vec::new(); dialog.lines.len()];
    dialog.index = 0;
    karma.revealed.insert(key);
    crate::skills::train(crate::skills::Skill::Eloquence, 15.0);
    session.notify(format!("{name} ti confida il suo segreto"));
    info!("[KARMA] segreto di {name} scoperto");
}

/// Sistema: un regalo gradito, di fronte all'abitante, una volta al giorno (tasto G).
#[allow(clippy::too_many_arguments)]
pub fn give_gift(
    keys: Res<ButtonInput<KeyCode>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    npcs: Query<(Entity, &GlobalTransform, &crate::entities::Npc)>,
    dialog: Res<crate::entities::DialogState>,
    clock: Res<crate::daynight::GameClock>,
    secrets: Res<Secrets>,
    mut karma: ResMut<Karma>,
    mut inventory: ResMut<crate::inventory::Inventory>,
    mut nearest: ResMut<crate::entities::NearestInteract>,
    mut barks: ResMut<crate::quests::Barks>,
) {
    if dialog.open {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();
    let day = today(&clock);
    let target = npcs
        .iter()
        .filter(|(_, t, _)| {
            let to = (p - t.translation()).with_y(0.0);
            // Davanti a lui: alle spalle la mano è quella del borseggio.
            to.length() <= GIFT_RANGE
                && to
                    .normalize_or_zero()
                    .dot((t.rotation() * Vec3::Z).with_y(0.0))
                    >= -0.2
        })
        .min_by(|a, b| {
            a.1.translation()
                .distance(p)
                .total_cmp(&b.1.translation().distance(p))
        });
    let Some((entity, _, npc)) = target else {
        return;
    };
    let Some(gift) = secrets.of(npc.name).and_then(|s| s.gift) else {
        return;
    };
    let key = npc.name.to_lowercase();
    if inventory.qty(gift) == 0 || karma.gifted.get(&key) == Some(&day) {
        return;
    }
    // Il regalo gradito si scopre conoscendolo: a uno sconosciuto non si sa cosa portare.
    if karma.bond(npc.name) == Bond::Stranger && !karma.met.contains(&key) {
        return;
    }
    let hint = format!("[G] Regala: {}", gift.name());
    nearest.prompt = Some(match nearest.prompt.take() {
        Some(prompt) => format!("{prompt} · {hint}"),
        None => hint,
    });
    if !keys.just_pressed(KeyCode::KeyG) {
        return;
    }
    inventory.consume(gift, 1);
    karma.gifted.insert(key, day);
    trust(npc.name, GIFT_TRUST);
    barks.say(entity, npc.name, "Per me? Grazie, davvero.".into());
    info!("[KARMA] regalo a {}: {}", npc.name, gift.name());
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Karma>()
        .init_resource::<Secrets>()
        .add_systems(Update, apply_deeds);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_villager_has_a_complete_secret() {
        let secrets = Secrets::default();
        assert_eq!(secrets.0.len(), 26);
        for s in &secrets.0 {
            assert!(!s.dear.is_empty(), "{}: manca «caro»", s.name);
            assert!(s.gift.is_some(), "{}: manca il dono", s.name);
            assert!(s.summary.len() > 40, "{}: segreto troppo corto", s.name);
            assert!(s.lines.len() >= 2, "{}: confidenza troppo corta", s.name);
        }
        assert!(secrets.of("beppe").is_some());
        assert!(secrets.of("Guardia Mario").is_some());
        // Ogni nome corrisponde a un abitante del paese (o al Viandante).
        let villagers = include_str!("../assets/world/abitanti.txt");
        for s in &secrets.0 {
            assert!(
                s.name == "Viandante" || villagers.contains(&format!("[{}]", s.name)),
                "{} non è un abitante",
                s.name
            );
        }
    }

    #[test]
    fn trust_grows_into_friendship_and_opens_the_secret() {
        let mut k = Karma::default();
        assert_eq!(k.bond("Beppe"), Bond::Stranger);
        k.add_trust("Beppe", 12);
        assert_eq!(k.bond("beppe"), Bond::Acquaintance);
        assert!(!k.ready_to_confide("Beppe", false));
        k.add_trust("Beppe", 10);
        assert!(
            k.ready_to_confide("Beppe", true),
            "chi ha occhio capisce prima"
        );
        assert!(!k.ready_to_confide("Beppe", false));
        k.add_trust("Beppe", 10);
        assert_eq!(k.bond("Beppe"), Bond::Friend);
        assert!(k.ready_to_confide("Beppe", false));
        k.revealed.insert("beppe".into());
        assert!(
            !k.ready_to_confide("Beppe", false),
            "un segreto si confida una volta"
        );
        k.add_trust("Beppe", 500);
        assert_eq!(
            (k.trust_of("Beppe"), k.bond("Beppe")),
            (TRUST_MAX, Bond::Kindred)
        );
        k.add_trust("Nina", -20);
        assert_eq!(k.bond("Nina"), Bond::Wary);
    }

    #[test]
    fn a_light_heart_and_a_friend_lower_the_prices() {
        let mut k = Karma::default();
        assert_eq!(k.price_factor("Tonio"), 1.0);
        k.global = 20;
        assert!(k.price_factor("Tonio") < 1.0);
        k.add_trust("Tonio", 35);
        let friendly = k.price_factor("Tonio");
        assert!(friendly < 0.93);
        k.global = -50;
        k.add_trust("Tonio", -100);
        assert!(k.price_factor("Tonio") > 1.2);
        assert_eq!(heart(50), "Cuore leggero come la piuma");
        assert_eq!(heart(0), "Cuore in bilico");
        assert_eq!(heart(-60), "Cuore pesante come il piombo");
    }
}
