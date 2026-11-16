//! Salvataggi permanenti (ADR 0025): un file di testo per salvataggio, modificabile a mano.
//!
//! `SaveData` è la fotografia della partita; `to_text` e `parse` sono funzioni pure. Righe sconosciute
//! o errate vengono ignorate con un avviso, e una sezione tolta dal file lascia il mondo com'è.

use crate::AppState;
use crate::character::{ActiveEffects, Character, Class, Effect, Stat, Stats};
use crate::containers::{ContainerKind, Containers};
use crate::daynight::{GameClock, GameDate};
use crate::entities::{Enemy, PatrolState};
use crate::hut::{Keeper, PropId, PropState};
use crate::inventory::{Equipment, Inventory, ItemKind, Pickup};
use crate::player::{Player, PlayerHealth};
use crate::quests::denaro_rubato::{
    NeedsGroundSnap, QuestCoin, QuestStage, QuestState, QuestThief,
};
use crate::region::Region;
use crate::villagers::{Villager, VillagerSave};
use crate::weather::{Kind as WeatherKind, WeatherState};
use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Dove comincia una nuova partita: la spiaggia a est del molo, tra i rottami (missione mp01).
pub const NEW_GAME_START: Vec3 = Vec3::new(30.0, 1.6, 87.5);

/// Salvataggi automatici tenuti per ogni personaggio e intervallo (secondi reali).
const AUTO_SLOTS: u32 = 3;
const AUTO_EVERY: f32 = 300.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveKind {
    Manual,
    Auto,
    Quick,
}

impl SaveKind {
    pub fn label(self) -> &'static str {
        match self {
            SaveKind::Manual => "manuale",
            SaveKind::Auto => "automatico",
            SaveKind::Quick => "rapido",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        [SaveKind::Manual, SaveKind::Auto, SaveKind::Quick]
            .into_iter()
            .find(|k| k.label() == text)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContainerSave {
    pub key: String,
    pub kind: ContainerKind,
    pub items: Vec<(ItemKind, u32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PickupSave {
    pub kind: ItemKind,
    pub qty: u32,
    pub pos: Vec3,
    pub dropped: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnemySave {
    pub thief: bool,
    pub pos: Vec3,
    pub hp: f32,
    pub hp_max: f32,
    pub a: Vec3,
    pub b: Vec3,
    pub current: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RespawnSave {
    pub timer: f32,
    pub a: Vec3,
    pub b: Vec3,
}

/// Fotografia della partita.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveData {
    pub kind: SaveKind,
    pub game_version: String,
    pub saved_at: String,
    pub played_secs: f32,
    pub place: String,
    pub name: String,
    pub class: Class,
    pub stats: Stats,
    pub age: crate::character::Age,
    pub awakening: u8,
    pub year: u32,
    pub month: u8,
    pub day: u8,
    pub hour: f32,
    pub snow_cover: f32,
    pub forced_weather: Option<WeatherKind>,
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub first_person: bool,
    pub health: f32,
    /// Vigore e magia (ADR 0039); negativo = non salvato, si parte pieni.
    pub vigor: f32,
    pub magic: f32,
    pub inventory: Vec<(ItemKind, u32)>,
    pub hotbar: [Option<ItemKind>; 10],
    pub weapon: Option<ItemKind>,
    pub shield: bool,
    /// Lo scudo imbracciato è lo Scudo dell'Orlo; la Lanterna di Tiro è accesa.
    pub rim: bool,
    pub lantern: bool,
    pub backpack: Option<ItemKind>,
    pub charm: Option<ItemKind>,
    pub armor: Option<ItemKind>,
    pub effects: Vec<Effect>,
    pub stage: QuestStage,
    pub coins_collected: u32,
    pub accepted: bool,
    pub final_done: bool,
    pub thief_spotted: bool,
    /// Bivi di «Denaro Rubato» (ADR 0035).
    pub fate: crate::quests::denaro_rubato::Fate,
    pub tablet: bool,
    pub kept: u32,
    pub mother_coin: bool,
    pub thief_provoked: bool,
    pub doors: Vec<(String, bool)>,
    pub containers: Vec<ContainerSave>,
    pub pickups: Vec<PickupSave>,
    pub quest_coins: Vec<Vec3>,
    pub enemies: Vec<EnemySave>,
    pub respawns: Vec<RespawnSave>,
    pub villagers: Vec<VillagerSave>,
    pub keeper_asleep: bool,
    pub keeper_stove_manual: bool,
    /// Missioni del Prologo: (id, tappa corrente).
    pub missions: Vec<(String, usize)>,
    /// Risposte date nelle missioni: (missione, tappa, risposta) (ADR 0044).
    pub choices: Vec<(String, usize, usize)>,
    /// Missioni che si ripetono: giorno in cui sono state compiute (`ripetuta:ec05 = 12345`).
    pub repeats: Vec<(String, u32)>,
    /// Luoghi scoperti: chiave «regione|nome» e punto d'arrivo del viaggio veloce.
    pub known: Vec<(String, Vec3)>,
    /// Abilità, talenti e livello (ADR 0038).
    pub skills: crate::skills::Progress,
    /// Serrature forzate e monete infilate (ADR 0039).
    pub thievery: crate::thievery::Thievery,
    /// Karma, fiducia degli abitanti e segreti scoperti (ADR 0040).
    pub karma: crate::karma::Karma,
    /// La casa del giocatore sull'isola (ADR 0042).
    pub house: crate::house::House,
    /// La Prova dell'Arco è stata superata (ADR 0043).
    pub archery_passed: bool,
    /// Chi ti segue e con quanta vita, e chi è a casa ferito (da che giorno) (ADR 0049).
    pub companion: Option<(String, f32)>,
    pub wounded: Vec<(String, u32)>,
    /// Regione in cui si trova il giocatore.
    pub region: Region,
    /// Stato delle altre regioni visitate (porte, contenitori, nemici…), com'erano quando
    /// le si è lasciate.
    pub elsewhere: Vec<(Region, Box<SaveData>)>,
    /// Sezioni trovate nel file: quelle mancanti non toccano il mondo.
    pub present: HashSet<String>,
}

/// Sezioni che descrivono il mondo di una regione (le altre seguono il personaggio).
const WORLD_SECTIONS: [&str; 8] = [
    "porte",
    "contenitori",
    "a_terra",
    "monete_quest",
    "nemici",
    "ricomparse",
    "abitanti",
    "viandante",
];
const HERO_SECTIONS: [&str; 15] = [
    "abilita",
    "compagni",
    "casa",
    "karma",
    "furti",
    "luoghi",
    "missioni",
    "personaggio",
    "tempo",
    "giocatore",
    "inventario",
    "barra",
    "equipaggiamento",
    "effetti",
    "quest",
];

/// Stato delle regioni lasciate durante questa partita.
#[derive(Resource, Default)]
pub struct RegionMemory(pub Vec<(Region, SaveData)>);

impl RegionMemory {
    fn put(&mut self, region: Region, data: SaveData) {
        self.0.retain(|(r, _)| *r != region);
        self.0.push((region, data));
    }

    fn get(&self, region: Region) -> Option<&SaveData> {
        self.0.iter().find(|(r, _)| *r == region).map(|(_, d)| d)
    }
}

fn v3(v: Vec3) -> String {
    format!("{:.3} {:.3} {:.3}", v.x, v.y, v.z)
}

fn read_v3(text: &str) -> Option<Vec3> {
    let mut it = text.split_whitespace().map(|t| t.parse::<f32>().ok());
    let v = Vec3::new(it.next()??, it.next()??, it.next()??);
    v.is_finite().then_some(v)
}

fn yes(v: bool) -> &'static str {
    if v { "si" } else { "no" }
}

fn read_bool(text: &str) -> Option<bool> {
    match text {
        "si" | "sì" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

fn item_or_none(item: Option<ItemKind>) -> &'static str {
    item.map_or("nessuno", ItemKind::name)
}

fn read_item_or_none(text: &str) -> Option<Option<ItemKind>> {
    if text == "nessuno" {
        Some(None)
    } else {
        ItemKind::from_name(text).map(Some)
    }
}

fn stage_name(stage: QuestStage) -> &'static str {
    match stage {
        QuestStage::Idle => "inattiva",
        QuestStage::SeekThief => "cerca_il_ladro",
        QuestStage::Collecting => "raccogli_monete",
        QuestStage::Returning => "torna_dal_viandante",
        QuestStage::Completed => "completata",
    }
}

fn read_stage(text: &str) -> Option<QuestStage> {
    [
        QuestStage::Idle,
        QuestStage::SeekThief,
        QuestStage::Collecting,
        QuestStage::Returning,
        QuestStage::Completed,
    ]
    .into_iter()
    .find(|s| stage_name(*s) == text)
}

fn container_name(kind: ContainerKind) -> &'static str {
    kind.name()
}

fn read_container(text: &str) -> Option<ContainerKind> {
    [
        ContainerKind::Shelf,
        ContainerKind::Crate,
        ContainerKind::Barrel,
        ContainerKind::Chest,
        ContainerKind::Wardrobe,
    ]
    .into_iter()
    .find(|k| k.name() == text)
}

fn items_text(items: &[(ItemKind, u32)]) -> String {
    if items.is_empty() {
        return "vuoto".to_string();
    }
    items
        .iter()
        .map(|(k, q)| format!("{} x{q}", k.name()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn read_items(text: &str) -> Option<Vec<(ItemKind, u32)>> {
    if text == "vuoto" {
        return Some(Vec::new());
    }
    text.split(',')
        .map(|part| {
            let (name, qty) = part.trim().rsplit_once(" x")?;
            Some((ItemKind::from_name(name)?, qty.trim().parse().ok()?))
        })
        .collect()
}

/// Etichetta del tasto `i` della barra (l'ultimo è lo 0).
fn slot_label(i: usize) -> usize {
    (i + 1) % 10
}

fn slot_index(label: usize) -> Option<usize> {
    match label {
        0 => Some(9),
        1..=9 => Some(label - 1),
        _ => None,
    }
}

impl SaveData {
    /// Salvataggio vuoto, da riempire leggendo un file.
    pub fn blank() -> Self {
        Self {
            kind: SaveKind::Manual,
            game_version: String::new(),
            saved_at: String::new(),
            played_secs: 0.0,
            place: String::new(),
            name: String::new(),
            class: Class::Custom,
            stats: Stats::default(),
            age: Default::default(),
            awakening: crate::character::AWAKE,
            year: 0,
            month: 5,
            day: 20,
            hour: 10.0,
            snow_cover: 0.0,
            forced_weather: None,
            pos: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.3,
            distance: 5.0,
            first_person: false,
            health: 100.0,
            vigor: -1.0,
            magic: -1.0,
            inventory: Vec::new(),
            hotbar: [None; 10],
            weapon: None,
            shield: false,
            rim: false,
            lantern: false,
            backpack: None,
            charm: None,
            armor: None,
            effects: Vec::new(),
            stage: QuestStage::Idle,
            coins_collected: 0,
            accepted: false,
            final_done: false,
            thief_spotted: false,
            fate: Default::default(),
            tablet: false,
            kept: 0,
            mother_coin: false,
            thief_provoked: false,
            doors: Vec::new(),
            containers: Vec::new(),
            pickups: Vec::new(),
            quest_coins: Vec::new(),
            enemies: Vec::new(),
            respawns: Vec::new(),
            villagers: Vec::new(),
            keeper_asleep: false,
            keeper_stove_manual: false,
            missions: Vec::new(),
            choices: Vec::new(),
            repeats: Vec::new(),
            companion: None,
            wounded: Vec::new(),
            known: Vec::new(),
            skills: Default::default(),
            thievery: Default::default(),
            karma: Default::default(),
            house: Default::default(),
            archery_passed: false,
            region: Region::Isola,
            elsewhere: Vec::new(),
            present: HashSet::new(),
        }
    }

    /// Il file di testo del salvataggio.
    pub fn to_text(&self) -> String {
        use std::fmt::Write;
        let mut t = String::new();
        let mut line = |text: String| {
            let _ = writeln!(t, "{text}");
        };
        line("# Salvataggio di Planet One: si può modificare a mano.".into());
        line("# Le righe con # sono commenti. Righe errate vengono ignorate.".into());
        line("# Togliendo un'intera sezione, quella parte del mondo resta come all'inizio.".into());
        line(String::new());
        line("[partita]".into());
        line(format!("tipo = {}", self.kind.label()));
        line(format!("versione = {}", self.game_version));
        line(format!("salvato = {}", self.saved_at));
        line(format!("tempo_giocato = {:.0}", self.played_secs));
        line(format!("luogo = {}", self.place));
        line(format!("regione = {}", self.region.key()));
        line(String::new());
        line("[personaggio]".into());
        line(format!("nome = {}", self.name));
        line(format!("classe = {}", self.class.name()));
        line(format!("eta = {}", self.age.key()));
        line(format!("risveglio = {}", self.awakening));
        for stat in Stat::ALL {
            line(format!("{} = {}", stat.name(), self.stats.get(stat)));
        }
        line(String::new());
        line("# abilità = grado, esperienza verso il prossimo, ranghi del talento attivati".into());
        line("[abilita]".into());
        line(format!("livello = {}", self.skills.level));
        line(format!("esperienza = {:.2}", self.skills.level_xp));
        line(format!("punti_statistica = {}", self.skills.stat_points));
        line(format!("nodi = {}", self.skills.nodes));
        for (i, skill) in crate::skills::Skill::ALL.into_iter().enumerate() {
            line(format!(
                "{} = {} {:.2} {}",
                skill.key(),
                self.skills.grades[i],
                self.skills.xp[i],
                self.skills.perks[i]
            ));
        }
        line(String::new());
        line("# abitante = fiducia, incontrato, segreto scoperto".into());
        line("[karma]".into());
        line(format!("globale = {}", self.karma.global));
        for deed in &self.karma.judged {
            line(format!("pesato:{deed} = sì"));
        }
        for what in &self.karma.unlocks {
            line(format!("sbloccato:{what} = sì"));
        }
        let mut people: Vec<&String> = self
            .karma
            .trust
            .keys()
            .chain(&self.karma.met)
            .chain(&self.karma.revealed)
            .collect();
        people.sort();
        people.dedup();
        for name in people {
            line(format!(
                "{name} = {} {} {}",
                self.karma.trust.get(name).copied().unwrap_or(0),
                yes(self.karma.met.contains(name)),
                yes(self.karma.revealed.contains(name))
            ));
        }
        line(String::new());
        line("# casa sul Poggio del Naufrago: 0 terreno, 1 fondamenta, 2 finita".into());
        line("[casa]".into());
        line(format!("stadio = {}", self.house.stage));
        line(format!("legname = {}", self.house.wood));
        line(format!("pietra = {}", self.house.stone));
        line(format!("prova_arco = {}", yes(self.archery_passed)));
        line(String::new());
        line("[compagni]".into());
        if let Some((name, hp)) = &self.companion {
            line(format!("segue = {name}|{hp:.0}"));
        }
        for (name, day) in &self.wounded {
            line(format!("ferito = {name}|{day}"));
        }
        line(String::new());
        line("[furti]".into());
        line(format!("serrature = {}", self.thievery.picked));
        line(format!("tentativi = {}", self.thievery.attempts));
        for (building, n) in &self.thievery.picked_in {
            line(format!("forzate:{building} = {n}"));
        }
        for (name, n) in &self.thievery.slipped {
            line(format!("infilate:{name} = {n}"));
        }
        for key in &self.thievery.keys {
            line(format!("chiave = {key}"));
        }
        line(String::new());
        line("[tempo]".into());
        line(format!("anno = {}", self.year));
        line(format!("mese = {}", self.month));
        line(format!("giorno = {}", self.day));
        line(format!("ora = {:.3}", self.hour));
        line(format!("neve_a_terra = {:.3}", self.snow_cover));
        line(format!(
            "meteo_forzato = {}",
            self.forced_weather.map_or("no", WeatherKind::save_name)
        ));
        line(String::new());
        line("[giocatore]".into());
        line(format!("posizione = {}", v3(self.pos)));
        line(format!(
            "vista = {:.3} {:.3} {:.3}",
            self.yaw, self.pitch, self.distance
        ));
        line(format!("prima_persona = {}", yes(self.first_person)));
        line(format!("vita = {:.1}", self.health));
        if self.vigor >= 0.0 {
            line(format!("vigore = {:.1}", self.vigor));
            line(format!("magia = {:.1}", self.magic));
        }
        line(String::new());
        line("# oggetto = quantità".into());
        line("[inventario]".into());
        for (kind, qty) in &self.inventory {
            line(format!("{} = {qty}", kind.name()));
        }
        line(String::new());
        line("# tasto = oggetto".into());
        line("[barra]".into());
        for (i, item) in self.hotbar.iter().enumerate() {
            if let Some(item) = item {
                line(format!("{} = {}", slot_label(i), item.name()));
            }
        }
        line(String::new());
        line("[equipaggiamento]".into());
        line(format!("arma = {}", item_or_none(self.weapon)));
        line(format!("scudo = {}", yes(self.shield)));
        line(format!("scudo_orlo = {}", yes(self.rim)));
        line(format!("lanterna = {}", yes(self.lantern)));
        line(format!("zaino = {}", item_or_none(self.backpack)));
        line(format!("incantato = {}", item_or_none(self.charm)));
        line(format!("armatura = {}", item_or_none(self.armor)));
        line(String::new());
        line("# fonte | chili | secondi rimasti oppure sempre".into());
        line("[effetti]".into());
        for e in &self.effects {
            let left = e
                .remaining
                .map_or("sempre".to_string(), |s| format!("{s:.1}"));
            line(format!("{} | {:.1} | {left}", e.source, e.carry_kg));
        }
        line(String::new());
        line("[quest]".into());
        line(format!("fase = {}", stage_name(self.stage)));
        line(format!("monete_raccolte = {}", self.coins_collected));
        line(format!("accettata = {}", yes(self.accepted)));
        line(format!("finale_fatto = {}", yes(self.final_done)));
        line(format!("ladro_avvistato = {}", yes(self.thief_spotted)));
        line(format!("sorte_ladro = {}", self.fate.key()));
        line(format!("tavoletta_letta = {}", yes(self.tablet)));
        line(format!("monete_tenute = {}", self.kept));
        line(format!("moneta_madre = {}", yes(self.mother_coin)));
        line(format!("ladro_provocato = {}", yes(self.thief_provoked)));
        line(String::new());
        line("# oggetto = aperto oppure chiuso (acceso/spento per stufa e luce)".into());
        line("[porte]".into());
        for (id, on) in &self.doors {
            let word = match (id.contains("stufa") || id.contains("luce"), *on) {
                (true, true) => "acceso",
                (true, false) => "spento",
                (false, true) => "aperto",
                (false, false) => "chiuso",
            };
            line(format!("{id} = {word}"));
        }
        line(String::new());
        line("# mobile | tipo | contenuto (Mela x4, Moneta x3 oppure vuoto)".into());
        line("[contenitori]".into());
        for c in &self.containers {
            line(format!(
                "{} | {} | {}",
                c.key,
                container_name(c.kind),
                items_text(&c.items)
            ));
        }
        line(String::new());
        line("# oggetto | quantità | x y z | gettato oppure originale".into());
        line("[a_terra]".into());
        for p in &self.pickups {
            line(format!(
                "{} | {} | {} | {}",
                p.kind.name(),
                p.qty,
                v3(p.pos),
                if p.dropped { "gettato" } else { "originale" }
            ));
        }
        line(String::new());
        line("# una moneta della quest per riga: x y z".into());
        line("[monete_quest]".into());
        for c in &self.quest_coins {
            line(v3(*c));
        }
        line(String::new());
        line("# tipo | posizione | vita | vita massima | tappa A | tappa B | tappa attuale".into());
        line("[nemici]".into());
        for e in &self.enemies {
            line(format!(
                "{} | {} | {:.1} | {:.1} | {} | {} | {}",
                if e.thief { "ladro" } else { "bandito" },
                v3(e.pos),
                e.hp,
                e.hp_max,
                v3(e.a),
                v3(e.b),
                e.current
            ));
        }
        line(String::new());
        line("# secondi alla ricomparsa | tappa A | tappa B".into());
        line("[ricomparse]".into());
        for r in &self.respawns {
            line(format!("{:.1} | {} | {}", r.timer, v3(r.a), v3(r.b)));
        }
        line(String::new());
        line("# nome | fase | a_riposo | posizione | rotazione | giro di ronda".into());
        line("[abitanti]".into());
        for v in &self.villagers {
            line(format!(
                "{} | {} | {} | {} | {:.4} {:.4} {:.4} {:.4} | {:.2}",
                v.name,
                v.phase,
                yes(v.hidden),
                v3(v.pos),
                v.rot.x,
                v.rot.y,
                v.rot.z,
                v.rot.w,
                v.patrol
            ));
        }
        line(String::new());
        line("[viandante]".into());
        line(format!("a_letto = {}", yes(self.keeper_asleep)));
        line(format!("stufa_manuale = {}", yes(self.keeper_stove_manual)));
        line(String::new());
        line("[missioni]".into());
        for (id, step) in &self.missions {
            line(format!("{id} = {step}"));
        }
        for (id, step, choice) in &self.choices {
            line(format!("scelta:{id}.{step} = {choice}"));
        }
        for (id, day) in &self.repeats {
            line(format!("ripetuta:{id} = {day}"));
        }
        line(String::new());
        line("# luogo scoperto = punto d'arrivo del viaggio veloce".into());
        line("[luoghi]".into());
        for (key, at) in &self.known {
            line(format!("{key} = {}", v3(*at)));
        }
        // Le altre regioni visitate: un salvataggio dentro l'altro, righe che iniziano con «>».
        for (region, inner) in &self.elsewhere {
            let mut copy = (**inner).clone();
            copy.elsewhere.clear();
            line(String::new());
            line(format!("[regione:{}]", region.key()));
            for l in copy.to_text().lines() {
                if !l.trim().is_empty() && !l.starts_with('#') {
                    line(format!("> {l}"));
                }
            }
        }
        t
    }

    /// Legge un file. Fallisce solo se mancano i dati indispensabili.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut d = Self::blank();
        let mut section = String::new();
        let mut nested: Vec<(Region, String)> = Vec::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.trim().to_lowercase();
                if !section.starts_with("regione:") {
                    d.present.insert(section.clone());
                }
                continue;
            }
            // Stato di un'altra regione: righe «> …» da leggere come un salvataggio a sé.
            if let Some(key) = section.strip_prefix("regione:") {
                if let (Some(region), Some(inner)) = (Region::parse(key), line.strip_prefix('>')) {
                    match nested.iter_mut().find(|(r, _)| *r == region) {
                        Some((_, text)) => {
                            text.push_str(inner.trim());
                            text.push('\n');
                        }
                        None => nested.push((region, format!("{}\n", inner.trim()))),
                    }
                }
                continue;
            }
            if d.read_line(&section, line).is_none() {
                warn!("[SAVE] riga ignorata in [{section}]: {line}");
            }
        }
        for (region, text) in nested {
            if let Ok(mut inner) = Self::parse(&text) {
                inner.region = region;
                d.elsewhere.push((region, Box::new(inner)));
            }
        }
        if !d.present.contains("partita") || d.name.trim().is_empty() {
            return Err("file non valido: mancano partita o nome del personaggio".to_string());
        }
        // Le tappe aggiunte nelle versioni successive spostano gli indici salvati.
        for (id, step) in &mut d.missions {
            let new = crate::quests::missions::upgrade_step(&d.game_version, id, *step);
            if new != *step {
                info!(
                    "[SAVE] missione {id}: tappa {step} → {new} (salvata con {})",
                    d.game_version
                );
                *step = new;
            }
        }
        // Spostate una volta sola: le regioni annidate si riscrivono con la loro versione.
        if crate::quests::missions::older_layout(&d.game_version) {
            d.game_version = crate::quests::missions::STEP_LAYOUT.into();
        }
        Ok(d)
    }

    fn read_line(&mut self, section: &str, line: &str) -> Option<()> {
        let pair = line.split_once('=').map(|(a, b)| (a.trim(), b.trim()));
        let fields: Vec<&str> = line.split('|').map(str::trim).collect();
        match section {
            "partita" => {
                let (k, v) = pair?;
                match k {
                    "tipo" => self.kind = SaveKind::parse(v)?,
                    "versione" => self.game_version = v.to_string(),
                    "salvato" => self.saved_at = v.to_string(),
                    "tempo_giocato" => self.played_secs = v.parse().ok()?,
                    "luogo" => self.place = v.to_string(),
                    "regione" => self.region = Region::parse(v)?,
                    _ => return None,
                }
            }
            "personaggio" => {
                let (k, v) = pair?;
                match k {
                    "nome" => self.name = v.to_string(),
                    "classe" => {
                        self.class = Class::ALL.into_iter().find(|c| c.name() == v)?;
                    }
                    "eta" => self.age = crate::character::Age::parse(v)?,
                    "risveglio" => {
                        self.awakening = v.parse::<u8>().ok()?.min(crate::character::AWAKE);
                    }
                    _ => self.stats.set(Stat::parse(k)?, v.parse().ok()?),
                }
            }
            "abilita" => {
                let (k, v) = pair?;
                let p = &mut self.skills;
                match k {
                    "livello" => p.level = v.parse::<u32>().ok()?.max(1),
                    "esperienza" => p.level_xp = v.parse().ok()?,
                    "punti_statistica" => p.stat_points = v.parse().ok()?,
                    "nodi" => p.nodes = v.parse().ok()?,
                    _ => {
                        let i = crate::skills::Skill::parse(k)?.index();
                        let mut parts = v.split_whitespace();
                        p.grades[i] = parts
                            .next()?
                            .parse::<u8>()
                            .ok()?
                            .clamp(1, crate::skills::MAX_GRADE);
                        p.xp[i] = parts.next()?.parse().ok()?;
                        p.perks[i] = parts.next()?.parse::<u8>().ok()?.min(5);
                    }
                }
            }
            "karma" => {
                let (k, v) = pair?;
                if k == "globale" {
                    self.karma.global = v.parse().ok()?;
                } else if let Some(deed) = k.strip_prefix("pesato:") {
                    self.karma.judged.insert(deed.to_string());
                } else if let Some(what) = k.strip_prefix("sbloccato:") {
                    self.karma.unlocks.insert(what.to_string());
                } else {
                    let mut parts = v.split_whitespace();
                    let trust: i32 = parts.next()?.parse().ok()?;
                    if trust != 0 {
                        self.karma.trust.insert(k.to_string(), trust);
                    }
                    if read_bool(parts.next()?)? {
                        self.karma.met.insert(k.to_string());
                    }
                    if read_bool(parts.next()?)? {
                        self.karma.revealed.insert(k.to_string());
                    }
                }
            }
            "casa" => {
                let (k, v) = pair?;
                match k {
                    "stadio" => {
                        self.house.stage = v.parse::<u8>().ok()?.min(crate::house::STONE_HOUSE)
                    }
                    "legname" => self.house.wood = v.parse().ok()?,
                    "pietra" => self.house.stone = v.parse().ok()?,
                    "prova_arco" => self.archery_passed = read_bool(v)?,
                    _ => return None,
                }
            }
            "compagni" => {
                let (k, v) = pair?;
                let (name, n) = v.rsplit_once('|')?;
                match k {
                    "segue" => self.companion = Some((name.trim().to_string(), n.parse().ok()?)),
                    "ferito" => self
                        .wounded
                        .push((name.trim().to_string(), n.parse().ok()?)),
                    _ => return None,
                }
            }
            "furti" => {
                let (k, v) = pair?;
                match k {
                    "serrature" => self.thievery.picked = v.parse().ok()?,
                    "tentativi" => self.thievery.attempts = v.parse().ok()?,
                    "chiave" => {
                        self.thievery.keys.insert(v.to_string());
                    }
                    _ if k.starts_with("forzate:") => {
                        self.thievery
                            .picked_in
                            .insert(k["forzate:".len()..].to_string(), v.parse().ok()?);
                    }
                    _ => {
                        let name = k.strip_prefix("infilate:")?;
                        self.thievery
                            .slipped
                            .insert(name.to_string(), v.parse().ok()?);
                    }
                }
            }
            "tempo" => {
                let (k, v) = pair?;
                match k {
                    "anno" => self.year = v.parse().ok()?,
                    "mese" => self.month = v.parse::<u8>().ok().filter(|m| (1..=12).contains(m))?,
                    "giorno" => self.day = v.parse::<u8>().ok().filter(|d| (1..=31).contains(d))?,
                    "ora" => {
                        self.hour = v.parse::<f32>().ok().filter(|h| (0.0..24.0).contains(h))?;
                    }
                    "neve_a_terra" => self.snow_cover = v.parse::<f32>().ok()?.clamp(0.0, 1.0),
                    "meteo_forzato" => {
                        self.forced_weather = if v == "no" {
                            None
                        } else {
                            Some(WeatherKind::parse(v)?)
                        };
                    }
                    _ => return None,
                }
            }
            "giocatore" => {
                let (k, v) = pair?;
                match k {
                    "posizione" => self.pos = read_v3(v)?,
                    "vista" => {
                        let view = read_v3(v)?;
                        (self.yaw, self.pitch, self.distance) = (view.x, view.y, view.z);
                    }
                    "prima_persona" => self.first_person = read_bool(v)?,
                    "vita" => self.health = v.parse().ok()?,
                    "vigore" => self.vigor = v.parse().ok()?,
                    "magia" => self.magic = v.parse().ok()?,
                    _ => return None,
                }
            }
            "inventario" => {
                let (k, v) = pair?;
                self.inventory
                    .push((ItemKind::from_name(k)?, v.parse().ok()?));
            }
            "barra" => {
                let (k, v) = pair?;
                let slot = slot_index(k.parse().ok()?)?;
                self.hotbar[slot] = Some(ItemKind::from_name(v)?);
            }
            "equipaggiamento" => {
                let (k, v) = pair?;
                match k {
                    "arma" => self.weapon = read_item_or_none(v)?,
                    "scudo" => self.shield = read_bool(v)?,
                    "scudo_orlo" => self.rim = read_bool(v)?,
                    "lanterna" => self.lantern = read_bool(v)?,
                    "zaino" => self.backpack = read_item_or_none(v)?,
                    "incantato" => self.charm = read_item_or_none(v)?,
                    "armatura" => self.armor = read_item_or_none(v)?,
                    _ => return None,
                }
            }
            "effetti" => {
                let [source, kg, left] = fields[..] else {
                    return None;
                };
                self.effects.push(Effect {
                    source: source.to_string(),
                    carry_kg: kg.parse().ok()?,
                    remaining: if left == "sempre" {
                        None
                    } else {
                        Some(left.parse().ok()?)
                    },
                });
            }
            "quest" => {
                let (k, v) = pair?;
                match k {
                    "fase" => self.stage = read_stage(v)?,
                    "monete_raccolte" => self.coins_collected = v.parse().ok()?,
                    "accettata" => self.accepted = read_bool(v)?,
                    "finale_fatto" => self.final_done = read_bool(v)?,
                    "ladro_avvistato" => self.thief_spotted = read_bool(v)?,
                    "sorte_ladro" => {
                        self.fate = crate::quests::denaro_rubato::Fate::parse(v)?;
                    }
                    "tavoletta_letta" => self.tablet = read_bool(v)?,
                    "monete_tenute" => self.kept = v.parse().ok()?,
                    "moneta_madre" => self.mother_coin = read_bool(v)?,
                    "ladro_provocato" => self.thief_provoked = read_bool(v)?,
                    _ => return None,
                }
            }
            "luoghi" => {
                let (k, v) = pair?;
                self.known.push((k.to_string(), read_v3(v)?));
            }
            "missioni" => {
                let (k, v) = pair?;
                if let Some(id) = k.strip_prefix("ripetuta:") {
                    self.repeats.push((id.to_string(), v.parse().ok()?));
                } else if let Some(at) = k.strip_prefix("scelta:") {
                    let (id, step) = at.rsplit_once('.')?;
                    self.choices
                        .push((id.to_string(), step.parse().ok()?, v.parse().ok()?));
                } else {
                    self.missions.push((k.to_string(), v.parse().ok()?));
                }
            }
            "porte" => {
                let (k, v) = pair?;
                let on = match v {
                    "aperto" | "acceso" => true,
                    "chiuso" | "spento" => false,
                    _ => return None,
                };
                self.doors.push((k.to_string(), on));
            }
            "contenitori" => {
                let [key, kind, items] = fields[..] else {
                    return None;
                };
                self.containers.push(ContainerSave {
                    key: key.to_string(),
                    kind: read_container(kind)?,
                    items: read_items(items)?,
                });
            }
            "a_terra" => {
                let [name, qty, pos, origin] = fields[..] else {
                    return None;
                };
                self.pickups.push(PickupSave {
                    kind: ItemKind::from_name(name)?,
                    qty: qty.parse().ok()?,
                    pos: read_v3(pos)?,
                    dropped: origin == "gettato",
                });
            }
            "monete_quest" => self.quest_coins.push(read_v3(line)?),
            "nemici" => {
                let [kind, pos, hp, hp_max, a, b, current] = fields[..] else {
                    return None;
                };
                self.enemies.push(EnemySave {
                    thief: kind == "ladro",
                    pos: read_v3(pos)?,
                    hp: hp.parse().ok()?,
                    hp_max: hp_max.parse().ok()?,
                    a: read_v3(a)?,
                    b: read_v3(b)?,
                    current: current.parse().ok()?,
                });
            }
            "ricomparse" => {
                let [timer, a, b] = fields[..] else {
                    return None;
                };
                self.respawns.push(RespawnSave {
                    timer: timer.parse().ok()?,
                    a: read_v3(a)?,
                    b: read_v3(b)?,
                });
            }
            "abitanti" => {
                let [name, phase, hidden, pos, rot, patrol] = fields[..] else {
                    return None;
                };
                let r: Vec<f32> = rot
                    .split_whitespace()
                    .filter_map(|t| t.parse().ok())
                    .collect();
                let [x, y, z, w] = r[..] else {
                    return None;
                };
                self.villagers.push(VillagerSave {
                    name: name.to_string(),
                    phase: phase.to_string(),
                    hidden: read_bool(hidden)?,
                    pos: read_v3(pos)?,
                    rot: Quat::from_xyzw(x, y, z, w).normalize(),
                    patrol: patrol.parse().ok()?,
                });
            }
            "viandante" => {
                let (k, v) = pair?;
                match k {
                    "a_letto" => self.keeper_asleep = read_bool(v)?,
                    "stufa_manuale" => self.keeper_stove_manual = read_bool(v)?,
                    _ => return None,
                }
            }
            _ => return None,
        }
        Some(())
    }
}

// ----- File -----

/// Cartella di tutti i salvataggi.
pub fn saves_root() -> PathBuf {
    crate::ai::tts::data_dir().join("saves")
}

/// Nome di cartella sicuro per un personaggio; `None` se il nome non è usabile.
pub fn folder_name(name: &str) -> Option<String> {
    let clean: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let clean = clean.trim_matches(['.', ' ', '_']).to_string();
    if clean.is_empty() || clean.chars().count() > 40 {
        return None;
    }
    // Nomi riservati su Windows.
    let upper = clean.to_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "LPT1", "LPT2"];
    Some(if reserved.contains(&upper.as_str()) {
        format!("_{clean}")
    } else {
        clean
    })
}

/// Il nome è già usato da un personaggio? Maiuscole e minuscole non contano.
pub fn name_taken(root: &Path, name: &str) -> bool {
    let Some(folder) = folder_name(name) else {
        return false;
    };
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| {
            e.file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(&folder)
        })
}

/// Data e ora locali, per il file e per l'elenco.
fn now_text() -> String {
    crate::debug_place::utc_now()
}

/// Scrive passando da un file temporaneo: un'interruzione non rovina il salvataggio precedente.
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// Il file automatico da usare: uno libero, altrimenti il più vecchio.
fn auto_slot(dir: &Path) -> PathBuf {
    let slots: Vec<PathBuf> = (1..=AUTO_SLOTS)
        .map(|i| dir.join(format!("automatico-{i}.txt")))
        .collect();
    if let Some(free) = slots.iter().find(|p| !p.exists()) {
        return free.clone();
    }
    slots
        .into_iter()
        .min_by_key(|p| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })
        .unwrap_or_else(|| dir.join("automatico-1.txt"))
}

/// Percorso del nuovo salvataggio di quel tipo.
pub fn target_path(dir: &Path, kind: SaveKind, stamp: &str) -> PathBuf {
    match kind {
        SaveKind::Manual => {
            let safe: String = stamp
                .chars()
                .map(|c| if c.is_ascii_digit() { c } else { '-' })
                .collect();
            dir.join(format!("manuale-{}.txt", safe.trim_matches('-')))
        }
        SaveKind::Auto => auto_slot(dir),
        SaveKind::Quick => dir.join("rapido.txt"),
    }
}

/// Un salvataggio trovato su disco.
#[derive(Clone, Debug)]
pub struct SaveEntry {
    pub path: PathBuf,
    pub folder: String,
    pub modified: SystemTime,
    pub data: SaveData,
}

/// Tutti i salvataggi leggibili, dal più recente.
pub fn list_saves(root: &Path) -> Vec<SaveEntry> {
    let mut out = Vec::new();
    for dir in std::fs::read_dir(root).into_iter().flatten().flatten() {
        if !dir.path().is_dir() {
            continue;
        }
        let folder = dir.file_name().to_string_lossy().to_string();
        for file in std::fs::read_dir(dir.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            let path = file.path();
            if path.extension().is_none_or(|e| e != "txt") {
                continue;
            }
            let Some(data) = std::fs::read_to_string(&path)
                .ok()
                .and_then(|t| SaveData::parse(&t).ok())
            else {
                warn!("[SAVE] file illeggibile: {}", path.display());
                continue;
            };
            let modified = file
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            out.push(SaveEntry {
                path,
                folder: folder.clone(),
                modified,
                data,
            });
        }
    }
    out.sort_by_key(|e| std::cmp::Reverse(e.modified));
    out
}

// ----- Partita in corso -----

/// Dopo il salvataggio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AfterSave {
    #[default]
    Nothing,
    MainMenu,
    Exit,
}

/// La partita in corso.
#[derive(Resource, Default)]
pub struct Session {
    pub active: bool,
    pub folder: String,
    pub played: f32,
    auto_timer: f32,
    notice: Option<(String, f32)>,
}

impl Session {
    pub fn notify(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), 3.0));
    }
}

/// Richiesta di salvataggio.
#[derive(Resource, Default)]
pub struct SaveRequest {
    pub kind: Option<SaveKind>,
    pub after: AfterSave,
}

/// Cosa caricare al termine della ricostruzione della scena.
#[derive(Resource, Default)]
pub enum LoadRequest {
    #[default]
    None,
    New(Character),
    File(Box<SaveData>),
    /// Viaggio verso un'altra regione: la fotografia della partita al momento della partenza.
    Travel(Box<SaveData>, Region),
}

impl LoadRequest {
    /// Regione in cui si troverà il giocatore dopo questa richiesta (`None` = quella corrente).
    pub fn region(&self) -> Option<Region> {
        match self {
            LoadRequest::None => None,
            LoadRequest::New(_) => Some(Region::Isola),
            LoadRequest::File(d) => Some(d.region),
            LoadRequest::Travel(_, to) => Some(*to),
        }
    }
}

/// Richiesta di viaggio verso un'altra regione (barca di Gaspare, shell).
#[derive(Resource, Default)]
pub struct TravelRequest(pub Option<Region>);

/// Risorse del personaggio.
#[derive(SystemParam)]
pub struct HeroParams<'w, 's> {
    character: ResMut<'w, Character>,
    progress: ResMut<'w, crate::skills::Progress>,
    vitals: ResMut<'w, crate::vitals::Vitals>,
    thievery: ResMut<'w, crate::thievery::Thievery>,
    karma: ResMut<'w, crate::karma::Karma>,
    house: ResMut<'w, crate::house::House>,
    trial: ResMut<'w, crate::archery::Trial>,
    effects: ResMut<'w, ActiveEffects>,
    inventory: ResMut<'w, Inventory>,
    equip: ResMut<'w, Equipment>,
    orbit: ResMut<'w, crate::camera::OrbitCamera>,
    cheats: ResMut<'w, crate::shell::Cheats>,
    players: Query<
        'w,
        's,
        (
            &'static mut Transform,
            &'static mut LinearVelocity,
            &'static mut PlayerHealth,
        ),
        With<Player>,
    >,
}

type EnemyItem = (
    Entity,
    &'static Transform,
    &'static crate::combat::EnemyHealth,
    &'static PatrolState,
    Has<QuestThief>,
);

type VillagerItem = (&'static mut Villager, &'static mut Transform);
type KeeperItem = (
    &'static mut Keeper,
    &'static mut Transform,
    &'static mut crate::entities::Npc,
);
type KeeperFilter = (Without<Player>, Without<Enemy>, Without<Villager>);

/// Risorse del mondo.
#[derive(SystemParam)]
pub struct WorldParams<'w, 's> {
    commands: Commands<'w, 's>,
    clock: ResMut<'w, GameClock>,
    weather: ResMut<'w, WeatherState>,
    quest: ResMut<'w, QuestState>,
    missions: ResMut<'w, crate::quests::missions::Missions>,
    known: ResMut<'w, crate::places::Known>,
    containers: ResMut<'w, Containers>,
    village: Res<'w, crate::village::VillageInfo>,
    memory: ResMut<'w, RegionMemory>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    asset_server: Res<'w, AssetServer>,
    props: Query<'w, 's, (&'static PropId, &'static mut PropState)>,
    pickups: Query<'w, 's, (Entity, &'static GlobalTransform, &'static Pickup)>,
    coins: Query<'w, 's, (Entity, &'static GlobalTransform), With<QuestCoin>>,
    enemies: Query<'w, 's, EnemyItem, (With<Enemy>, Without<Player>)>,
    respawns: Query<'w, 's, (Entity, &'static crate::combat::EnemyRespawn)>,
    villagers: Query<'w, 's, VillagerItem, (Without<Player>, Without<Enemy>)>,
    keepers: Query<'w, 's, KeeperItem, KeeperFilter>,
    companions: Query<
        'w,
        's,
        (
            &'static crate::entities::Npc,
            &'static crate::companions::Companion,
        ),
        Without<Keeper>,
    >,
    companion_state: ResMut<'w, crate::companions::CompanionState>,
}

/// Nome del luogo per l'elenco dei salvataggi.
fn place_name(village: &crate::village::VillageInfo, p: Vec3) -> String {
    let inside = crate::village::in_polygon(&village.poly, Vec2::new(p.x, p.z));
    match crate::region::current() {
        _ if inside => village.spec.name.clone(),
        Region::Isola if p.z > 75.0 => "Porto".into(),
        Region::Isola => "Campagna".into(),
        Region::Nord if p.z > 420.0 => "Approdo delle Brume".into(),
        Region::Nord => "Piana delle Brume".into(),
    }
}

/// Fotografia della partita in corso.
fn collect(kind: SaveKind, session: &Session, hero: &HeroParams, world: &WorldParams) -> SaveData {
    let (pos, health) = hero
        .players
        .single()
        .map_or((Vec3::ZERO, 100.0), |(t, _, hp)| {
            (t.translation, hp.current)
        });
    let mut d = SaveData::blank();
    d.kind = kind;
    d.missions = world.missions.snapshot();
    d.choices = world.missions.choices_snapshot();
    d.repeats = world
        .missions
        .done_on
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    d.known = world.known.0.iter().map(|(k, v)| (k.clone(), *v)).collect();
    d.skills = hero.progress.clone();
    d.thievery = hero.thievery.clone();
    d.karma = hero.karma.clone();
    d.house = *hero.house;
    d.archery_passed = hero.trial.passed;
    d.companion = world
        .companions
        .iter()
        .next()
        .map(|(npc, c)| (npc.name.to_string(), c.hp));
    d.wounded = world.companion_state.wounded.clone();
    d.vigor = hero.vitals.vigor;
    d.magic = hero.vitals.magic;
    d.region = crate::region::current();
    d.elsewhere = world
        .memory
        .0
        .iter()
        .filter(|(r, _)| *r != d.region)
        .map(|(r, m)| (*r, Box::new(m.clone())))
        .collect();
    d.present = WORLD_SECTIONS
        .iter()
        .chain(&HERO_SECTIONS)
        .map(|s| (*s).to_string())
        .collect();
    d.game_version = crate::version::GAME_VERSION.to_string();
    d.saved_at = now_text();
    d.played_secs = session.played;
    d.place = place_name(&world.village, pos);
    d.name = hero.character.name.clone();
    d.class = hero.character.class;
    d.stats = hero.character.stats;
    d.age = hero.character.age;
    d.awakening = hero.character.awakening;
    d.year = world.clock.date.year;
    d.month = world.clock.date.month;
    d.day = world.clock.date.day;
    d.hour = world.clock.hour;
    d.snow_cover = world.weather.snow_cover;
    d.forced_weather = world.weather.forced;
    d.pos = pos;
    d.yaw = hero.orbit.yaw;
    d.pitch = hero.orbit.pitch;
    d.distance = hero.orbit.distance;
    d.first_person = hero.orbit.first_person_lock;
    d.health = health;
    d.inventory = hero
        .inventory
        .slots
        .iter()
        .map(|s| (s.kind, s.qty))
        .collect();
    d.hotbar = hero.inventory.hotbar;
    d.weapon = hero.equip.weapon;
    d.shield = hero.equip.shield;
    d.rim = hero.equip.rim;
    d.lantern = hero.equip.lantern;
    d.backpack = hero.equip.backpack;
    d.charm = hero.equip.charm;
    d.armor = hero.equip.armor;
    d.effects = hero.effects.0.clone();
    d.stage = world.quest.stage;
    d.coins_collected = world.quest.coins_collected;
    d.accepted = world.quest.accepted;
    d.final_done = world.quest.final_done;
    d.thief_spotted = world.quest.thief_spotted;
    d.fate = world.quest.fate;
    d.tablet = world.quest.tablet;
    d.kept = world.quest.kept;
    d.mother_coin = world.quest.mother_coin;
    d.thief_provoked = world.quest.thief_provoked;
    d.doors = world
        .props
        .iter()
        .map(|(id, s)| (id.0.clone(), s.on))
        .collect();
    d.doors.sort();
    d.containers = world.containers.snapshot();
    d.pickups = world
        .pickups
        .iter()
        .map(|(_, t, p)| PickupSave {
            kind: p.kind,
            qty: p.qty,
            pos: t.translation(),
            dropped: p.dropped,
        })
        .collect();
    d.quest_coins = world.coins.iter().map(|(_, t)| t.translation()).collect();
    d.enemies = world
        .enemies
        .iter()
        .map(|(_, t, hp, patrol, thief)| EnemySave {
            thief,
            pos: t.translation,
            hp: hp.current,
            hp_max: hp.max,
            a: patrol.waypoints.first().copied().unwrap_or(t.translation),
            b: patrol.waypoints.get(1).copied().unwrap_or(t.translation),
            current: patrol.current,
        })
        .collect();
    d.respawns = world
        .respawns
        .iter()
        .map(|(_, r)| RespawnSave {
            timer: r.timer,
            a: r.waypoints.first().copied().unwrap_or_default(),
            b: r.waypoints.get(1).copied().unwrap_or_default(),
        })
        .collect();
    d.villagers = world.villagers.iter().map(|(v, t)| v.snapshot(t)).collect();
    if let Some((k, ..)) = world.keepers.iter().next() {
        d.keeper_asleep = k.is_asleep();
        d.keeper_stove_manual = k.stove_manual();
    }
    d
}

/// Applica una fotografia alla scena appena ricostruita.
fn apply(d: &SaveData, hero: &mut HeroParams, world: &mut WorldParams) {
    let has = |s: &str| d.present.contains(s);
    if has("missioni") {
        world.missions.restore(&d.missions);
        world.missions.restore_choices(&d.choices);
        world.missions.done_on = d.repeats.iter().cloned().collect();
    }
    if has("luoghi") {
        world.known.0 = d.known.iter().cloned().collect();
    }
    if has("compagni") {
        world.companion_state.restore = Some(d.companion.clone());
        world.companion_state.wounded = d.wounded.clone();
    }
    if has("casa") {
        *hero.house = d.house;
        *hero.trial = crate::archery::Trial {
            passed: d.archery_passed,
            hits: if d.archery_passed { 3 } else { 0 },
            request: None,
        };
    }
    if has("karma") {
        *hero.karma = d.karma.clone();
    }
    if has("furti") {
        *hero.thievery = d.thievery.clone();
    }
    if has("abilita") {
        *hero.progress = d.skills.clone();
    }
    if has("personaggio") {
        *hero.character = Character {
            name: d.name.clone(),
            class: d.class,
            stats: d.stats,
            age: d.age,
            awakening: d.awakening,
        };
    }
    if has("tempo") {
        world.clock.set(
            GameDate {
                year: d.year,
                month: d.month,
                day: d.day,
            },
            d.hour,
        );
        world.weather.snow_cover = d.snow_cover;
        world.weather.forced = d.forced_weather;
    }
    if has("giocatore")
        && let Ok((mut t, mut vel, mut hp)) = hero.players.single_mut()
    {
        t.translation = d.pos;
        vel.0 = Vec3::ZERO;
        hp.max = hero.character.stats.max_health();
        hp.current = d.health.clamp(1.0, hp.max);
        let mut vitals = crate::vitals::Vitals::full(hero.character.stats);
        if d.vigor >= 0.0 {
            vitals.vigor = d.vigor.min(vitals.vigor_max);
            vitals.magic = d.magic.clamp(0.0, vitals.magic_max);
        }
        *hero.vitals = vitals;
        hero.orbit.yaw = d.yaw;
        hero.orbit.pitch = d.pitch;
        hero.orbit.distance = d.distance;
        hero.orbit.first_person_lock = d.first_person;
        hero.orbit.mode = if d.first_person {
            crate::camera::CameraMode::FirstPerson
        } else {
            crate::camera::CameraMode::ThirdPerson
        };
    }
    if has("inventario") {
        hero.inventory.slots = d
            .inventory
            .iter()
            .filter(|(_, q)| *q > 0)
            .map(|(kind, qty)| crate::inventory::ItemStack {
                kind: *kind,
                qty: *qty,
            })
            .collect();
    }
    if has("barra") {
        hero.inventory.hotbar = d.hotbar;
    }
    if has("equipaggiamento") {
        hero.equip.weapon = d.weapon;
        hero.equip.shield = d.shield;
        hero.equip.rim = d.rim;
        hero.equip.lantern = d.lantern;
        hero.equip.backpack = d.backpack;
        hero.equip.charm = d.charm;
        hero.equip.armor = d.armor;
    }
    if has("effetti") {
        hero.effects.0 = d.effects.clone();
    }
    if has("quest") {
        world.quest.stage = d.stage;
        world.quest.coins_collected = d.coins_collected;
        world.quest.accepted = d.accepted;
        world.quest.final_done = d.final_done;
        world.quest.thief_spotted = d.thief_spotted;
        world.quest.fate = d.fate;
        world.quest.tablet = d.tablet;
        world.quest.kept = d.kept;
        world.quest.mother_coin = d.mother_coin;
        world.quest.thief_provoked = d.thief_provoked;
    }
    if has("porte") {
        for (id, mut state) in &mut world.props {
            if let Some((_, on)) = d.doors.iter().find(|(k, _)| *k == id.0) {
                state.on = *on;
            }
        }
    }
    if has("contenitori") {
        world.containers.restore(&d.containers);
    }
    if has("a_terra") {
        for (e, ..) in &world.pickups {
            world.commands.entity(e).despawn();
        }
        for p in &d.pickups {
            crate::inventory::spawn_ground_item(
                &mut world.commands,
                &mut world.meshes,
                &mut world.materials,
                p.kind,
                p.qty,
                p.pos,
                p.dropped,
            );
        }
    }
    if has("monete_quest") {
        for (e, _) in &world.coins {
            world.commands.entity(e).despawn();
        }
        crate::quests::denaro_rubato::spawn_coins_at(
            &mut world.commands,
            &mut world.meshes,
            &mut world.materials,
            &d.quest_coins,
        );
    }
    if has("nemici") {
        for (e, ..) in &world.enemies {
            world.commands.entity(e).despawn();
        }
        for e in &d.enemies {
            let entity = if e.thief {
                crate::quests::denaro_rubato::spawn_thief(
                    &mut world.commands,
                    &world.asset_server,
                    e.a,
                    e.b,
                )
            } else {
                crate::entities::spawn_enemy(&mut world.commands, &world.asset_server, e.a, e.b)
            };
            let speed = if e.thief { 3.0 } else { 2.5 };
            let mut c = world.commands.entity(entity);
            c.insert((
                Transform::from_translation(e.pos),
                crate::combat::EnemyHealth {
                    current: e.hp,
                    max: e.hp_max,
                },
                PatrolState {
                    waypoints: vec![e.a, e.b],
                    current: e.current.min(1),
                    speed,
                },
            ));
            c.remove::<NeedsGroundSnap>();
        }
    }
    if has("ricomparse") {
        for (e, _) in &world.respawns {
            world.commands.entity(e).despawn();
        }
        for r in &d.respawns {
            world.commands.spawn((
                crate::scene::SceneNode,
                crate::combat::EnemyRespawn {
                    timer: r.timer,
                    waypoints: vec![r.a, r.b],
                },
            ));
        }
    }
    if has("abitanti") {
        for (mut v, mut t) in &mut world.villagers {
            if let Some(s) = d.villagers.iter().find(|s| s.name == v.name()) {
                v.restore(s, &mut t);
            }
        }
    }
    if has("viandante") {
        for (mut k, mut t, mut npc) in &mut world.keepers {
            k.restore(d.keeper_asleep, d.keeper_stove_manual, &mut t, &mut npc);
        }
    }
}

/// Sistema: chiudere la finestra durante una partita chiede un salvataggio automatico.
pub fn save_on_close(
    mut request: ResMut<SaveRequest>,
    session: Res<Session>,
    mut closing: MessageReader<WindowCloseRequested>,
) {
    if closing.read().count() > 0 && session.active {
        request.kind = Some(SaveKind::Auto);
    }
}

/// C'è un salvataggio da fare? `process_save` tocca quasi tutto il mondo: girando solo allora
/// non blocca più ogni frame gli altri sistemi (ADR 0056).
fn save_requested(request: Res<SaveRequest>) -> bool {
    request.kind.is_some()
}

/// Sistema: esegue le richieste di salvataggio, poi l'eventuale uscita.
#[allow(clippy::too_many_arguments)]
pub fn process_save(
    mut request: ResMut<SaveRequest>,
    mut session: ResMut<Session>,
    hero: HeroParams,
    world: WorldParams,
    mut next: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(kind) = request.kind.take() else {
        return;
    };
    let after = std::mem::take(&mut request.after);
    if session.active {
        let data = collect(kind, &session, &hero, &world);
        let dir = saves_root().join(&session.folder);
        let _ = std::fs::create_dir_all(&dir);
        let path = target_path(&dir, kind, &data.saved_at);
        match write_atomic(&path, &data.to_text()) {
            Ok(()) => {
                info!("[SAVE] salvato {}", path.display());
                if after == AfterSave::Nothing {
                    session.notify(match kind {
                        SaveKind::Auto => "Salvataggio automatico",
                        SaveKind::Quick => "Salvataggio rapido",
                        SaveKind::Manual => "Partita salvata",
                    });
                }
            }
            Err(e) => {
                warn!("[SAVE] impossibile salvare: {e}");
                session.notify(format!("Salvataggio non riuscito: {e}"));
            }
        }
    }
    match after {
        AfterSave::Nothing => {}
        AfterSave::MainMenu => {
            session.active = false;
            next.set(AppState::MainMenu);
        }
        AfterSave::Exit => {
            exit.write(AppExit::Success);
        }
    }
}

/// Sistema: tempo giocato, salvataggio automatico, F5 e F9.
#[allow(clippy::too_many_arguments)]
pub fn session_tick(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    dialog: Res<crate::entities::DialogState>,
    mut session: ResMut<Session>,
    mut request: ResMut<SaveRequest>,
    mut load: ResMut<LoadRequest>,
    mut next: ResMut<NextState<AppState>>,
) {
    if !session.active {
        return;
    }
    let dt = time.delta_secs();
    session.played += dt;
    session.auto_timer += dt;
    if session.auto_timer >= AUTO_EVERY && !dialog.open {
        session.auto_timer = 0.0;
        request.kind = Some(SaveKind::Auto);
    }
    if keys.just_pressed(KeyCode::F5) {
        request.kind = Some(SaveKind::Quick);
    }
    if keys.just_pressed(KeyCode::F9) {
        let path = saves_root().join(&session.folder).join("rapido.txt");
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| SaveData::parse(&t))
        {
            Ok(data) => {
                *load = LoadRequest::File(Box::new(data));
                next.set(AppState::Resetting);
            }
            Err(_) => session.notify("Nessun salvataggio rapido per questo personaggio"),
        }
    }
}

/// Sistema: parte il viaggio chiesto in `TravelRequest`. Fotografa la partita, ricorda lo stato
/// della regione che si lascia e ricostruisce la scena nell'altra (`scene::reset_scene`).
pub fn start_travel(
    mut travel: ResMut<TravelRequest>,
    session: Res<Session>,
    hero: HeroParams,
    mut world: WorldParams,
    mut load: ResMut<LoadRequest>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(to) = travel.0.take() else {
        return;
    };
    let from = crate::region::current();
    if to == from {
        return;
    }
    let data = collect(SaveKind::Auto, &session, &hero, &world);
    let mut left = data.clone();
    left.elsewhere.clear();
    world.memory.put(from, left);
    info!(
        "[VIAGGIO] partenza da {} verso {}",
        from.title(),
        to.title()
    );
    *load = LoadRequest::Travel(Box::new(data), to);
    next.set(AppState::Resetting);
}

/// Sistema (stato Loading): appena la scena è pronta applica la richiesta ed entra in gioco.
#[allow(clippy::too_many_arguments)]
pub fn finish_loading(
    homes: Res<crate::world_props::Homes>,
    field: Option<Res<crate::terrain::TerrainField>>,
    mut load: ResMut<LoadRequest>,
    mut session: ResMut<Session>,
    mut request: ResMut<SaveRequest>,
    mut hero: HeroParams,
    mut world: WorldParams,
    mut next: ResMut<NextState<AppState>>,
) {
    // Sull'isola si aspetta anche la casetta del Viandante (i suoi oggetti con stato).
    let ready = homes.built
        && field.is_some()
        && !hero.players.is_empty()
        && (crate::region::current() != Region::Isola || world.props.iter().next().is_some());
    if !ready {
        return;
    }
    match std::mem::take(&mut *load) {
        LoadRequest::None => {}
        LoadRequest::New(character) => {
            // Il naufrago non ha ancora un nome: la cartella si chiama «Naufrago» finché non lo dice.
            let folder = if character.awakening < crate::character::AWAKE {
                free_folder(&saves_root(), "Naufrago")
            } else {
                character.name.clone()
            };
            start_session(&mut session, &folder, 0.0);
            world.memory.0.clear();
            // La storia comincia sulla spiaggia a est del molo, tra i rottami (missione mp01).
            world.missions.reset();
            world.missions.enabled = true;
            world.known.0.clear();
            *hero.progress = Default::default();
            *hero.thievery = Default::default();
            *hero.karma = Default::default();
            *hero.house = Default::default();
            *hero.trial = Default::default();
            *hero.vitals = crate::vitals::Vitals::full(character.stats);
            *hero.character = character;
            *hero.inventory = Inventory::default();
            *hero.equip = Equipment::default();
            *hero.effects = ActiveEffects::default();
            *hero.cheats = Default::default();
            *world.clock = GameClock::default();
            world.weather.forced = None;
            world.weather.snow_cover = 0.0;
            *world.containers = Containers::default();
            if let Ok((mut t, _, mut hp)) = hero.players.single_mut() {
                t.translation = NEW_GAME_START;
                hero.orbit.yaw = std::f32::consts::PI * 0.75;
                hp.max = hero.character.stats.max_health();
                hp.current = hp.max;
            }
            request.kind = Some(SaveKind::Auto);
            info!("[SAVE] nuova partita: {}", hero.character.name);
        }
        LoadRequest::Travel(mut data, to) => {
            // Il personaggio porta con sé tutto il suo; il mondo è quello ricordato della
            // regione d'arrivo (o nuovo, la prima volta).
            data.present.retain(|s| HERO_SECTIONS.contains(&s.as_str()));
            if let Some(m) = world.memory.get(to).cloned() {
                data.doors = m.doors;
                data.containers = m.containers;
                data.pickups = m.pickups;
                data.quest_coins = m.quest_coins;
                data.enemies = m.enemies;
                data.respawns = m.respawns;
                data.villagers = m.villagers;
                data.keeper_asleep = m.keeper_asleep;
                data.keeper_stove_manual = m.keeper_stove_manual;
                data.present
                    .extend(WORLD_SECTIONS.iter().map(|s| (*s).to_string()));
            }
            // Dalla via sotto il mare si arriva alla fonte (o alla nicchia), altrimenti al molo.
            let (pos, yaw) = crate::crossing::arrival(to);
            data.pos = pos;
            data.yaw = yaw;
            data.first_person = false;
            *world.containers = Containers::default();
            apply(&data, &mut hero, &mut world);
            info!("[VIAGGIO] arrivo: {}", to.title());
            session.notify(to.title());
            request.kind = Some(SaveKind::Auto);
        }
        LoadRequest::File(data) => {
            // Una partita salvata è una partita vera: le missioni sono attive (un salvataggio
            // vecchio, senza la sezione, riparte dalla prima tappa di ogni missione).
            world.missions.reset();
            world.missions.enabled = true;
            world.known.0.clear();
            *hero.progress = Default::default();
            *hero.thievery = Default::default();
            *hero.karma = Default::default();
            *hero.house = Default::default();
            *hero.trial = Default::default();
            world.memory.0 = data
                .elsewhere
                .iter()
                .map(|(r, d)| (*r, (**d).clone()))
                .collect();
            start_session(&mut session, &data.name, data.played_secs);
            *hero.inventory = Inventory::default();
            *hero.equip = Equipment::default();
            *hero.effects = ActiveEffects::default();
            *hero.cheats = Default::default();
            *world.containers = Containers::default();
            apply(&data, &mut hero, &mut world);
            info!("[SAVE] partita caricata: {}", data.name);
            session.notify("Partita caricata");
        }
    }
    next.set(AppState::Playing);
}

/// Prove automatiche: partita attiva senza passare dal menu (`PLANET_ONE_SESSION=Nome`).
pub fn start_test_session(session: &mut Session, name: &str) {
    start_session(session, name, 0.0);
}

/// Una cartella libera per i salvataggi a partire da `name` («Naufrago», «Naufrago 2»…).
pub fn free_folder(root: &Path, name: &str) -> String {
    let base = folder_name(name).unwrap_or_else(|| "personaggio".to_string());
    let mut n = 1;
    loop {
        let candidate = if n == 1 {
            base.clone()
        } else {
            format!("{base} {n}")
        };
        if !name_taken(root, &candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Il naufrago ha detto il suo nome: la cartella dei salvataggi lo prende (ADR 0047).
pub fn rename_session(session: &mut Session, name: &str) {
    let root = saves_root();
    let wanted = folder_name(name).unwrap_or_default();
    if wanted.is_empty() || wanted.eq_ignore_ascii_case(&session.folder) {
        return;
    }
    let target = free_folder(&root, name);
    let from = root.join(&session.folder);
    if from.exists()
        && let Err(e) = std::fs::rename(&from, root.join(&target))
    {
        warn!("[SAVE] non riesco a rinominare la cartella dei salvataggi: {e}");
        return;
    }
    info!(
        "[SAVE] cartella dei salvataggi: {} → {target}",
        session.folder
    );
    session.folder = target;
}

fn start_session(session: &mut Session, name: &str, played: f32) {
    session.active = true;
    session.folder = folder_name(name).unwrap_or_else(|| "personaggio".to_string());
    session.played = played;
    session.auto_timer = 0.0;
}

/// Sistema (egui): avviso breve in gioco.
pub fn draw_notice(
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut contexts: bevy_egui::EguiContexts,
) {
    let dt = time.delta_secs();
    let Some((text, left)) = session.notice.clone() else {
        return;
    };
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    bevy_egui::egui::Area::new(bevy_egui::egui::Id::new("save_notice"))
        .anchor(
            bevy_egui::egui::Align2::CENTER_TOP,
            bevy_egui::egui::vec2(0.0, 112.0),
        )
        .show(ctx, |ui| {
            bevy_egui::egui::Frame::default()
                .fill(bevy_egui::egui::Color32::from_rgba_unmultiplied(
                    10, 12, 20, 235,
                ))
                .corner_radius(bevy_egui::egui::CornerRadius::same(6))
                .inner_margin(bevy_egui::egui::Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.add(
                        bevy_egui::egui::Label::new(
                            bevy_egui::egui::RichText::new(&text).size(18.0),
                        )
                        .wrap_mode(bevy_egui::egui::TextWrapMode::Extend),
                    );
                });
        });
    session.notice = (left > dt).then_some((text, left - dt));
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Session>()
        .init_resource::<SaveRequest>()
        .init_resource::<LoadRequest>()
        .init_resource::<RegionMemory>()
        .init_resource::<TravelRequest>()
        .add_systems(
            Update,
            (
                session_tick.run_if(in_state(AppState::Playing)),
                start_travel.run_if(in_state(AppState::Playing)),
                (save_on_close, process_save.run_if(save_requested)).chain(),
                finish_loading.run_if(in_state(AppState::Loading)),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skills_perks_and_level_survive_a_save() {
        use crate::skills::Skill;
        let mut d = sample();
        d.skills.set_level(4);
        d.skills.set_grade(Skill::Athletics, 31);
        d.skills.gain(Skill::Blade, 5.0);
        assert!(d.skills.activate(Skill::Athletics));
        d.thievery.picked = 2;
        d.thievery.picked_in.insert("caserma".into(), 2);
        d.choices = vec![("qs03".into(), 2, 1), ("qs08".into(), 3, 0)];
        d.repeats = vec![("ec05".into(), 412)];
        d.thievery.attempts = 7;
        d.thievery.keys.insert("casa-piccola 2".into());
        d.thievery.slipped.insert("guardia bruno".into(), 15);
        d.vigor = 42.0;
        d.magic = 17.0;
        d.house = crate::house::House {
            stage: 1,
            wood: 7,
            stone: 2,
        };
        d.karma.global = -7;
        d.karma.add_trust("Guardia Mario", 31);
        d.karma.add_trust("Nina", -15);
        d.karma.met.insert("guardia mario".into());
        d.karma.met.insert("rosa".into());
        d.karma.revealed.insert("guardia mario".into());
        d.karma.judged.insert("ladro".into());
        let text = d.to_text();
        let back = SaveData::parse(&text).unwrap();
        assert_eq!(back.skills, d.skills);
        assert_eq!(back.thievery, d.thievery);
        assert_eq!(back.choices, d.choices);
        assert_eq!(back.repeats, d.repeats);
        assert_eq!(back.karma, d.karma);
        assert_eq!(back.house, d.house);
        assert!(!back.archery_passed);
        d.archery_passed = true;
        assert!(SaveData::parse(&d.to_text()).unwrap().archery_passed);
        assert_eq!((back.vigor, back.magic), (42.0, 17.0));
        assert!(back.present.contains("abilita"));
        // Un salvataggio vecchio, senza la sezione, resta leggibile.
        let start = text.find("[abilita]").unwrap();
        let end = text.find("[tempo]").unwrap();
        let old = format!("{}{}", &text[..start], &text[end..]);
        let back = SaveData::parse(&old).unwrap();
        assert!(!back.present.contains("abilita"));
        assert_eq!(back.skills, crate::skills::Progress::default());
    }

    #[test]
    fn the_region_and_the_other_regions_survive_a_save() {
        let mut d = sample();
        d.region = Region::Nord;
        let mut island = sample();
        island.region = Region::Isola;
        island.doors = vec![("casa:0".into(), true)];
        island.elsewhere.clear();
        d.elsewhere = vec![(Region::Isola, Box::new(island))];
        let back = SaveData::parse(&d.to_text()).unwrap();
        assert_eq!(back.region, Region::Nord);
        assert_eq!(back.elsewhere.len(), 1);
        let (region, inner) = &back.elsewhere[0];
        assert_eq!(*region, Region::Isola);
        assert_eq!(inner.doors, vec![("casa:0".to_string(), true)]);
        // Un salvataggio vecchio, senza la riga della regione, è sull'isola.
        let old = d.to_text().replace("regione = nord\n", "");
        assert_eq!(SaveData::parse(&old).unwrap().region, Region::Isola);
    }

    fn sample() -> SaveData {
        let mut d = SaveData::blank();
        d.kind = SaveKind::Quick;
        d.game_version = crate::quests::missions::STEP_LAYOUT.into();
        d.saved_at = "2026-11-09 12:30".into();
        d.played_secs = 321.0;
        d.place = "Fontevivo".into();
        d.name = "Àlda Rossi".into();
        d.class = Class::Explorer;
        d.stats = Class::Explorer.stats();
        d.year = 1;
        d.month = 12;
        d.day = 3;
        d.hour = 21.5;
        d.snow_cover = 0.25;
        d.forced_weather = Some(WeatherKind::Snow);
        d.pos = Vec3::new(1.5, 2.5, -3.5);
        d.yaw = 1.0;
        d.pitch = 0.5;
        d.distance = 4.0;
        d.first_person = true;
        d.health = 80.0;
        d.inventory = vec![(ItemKind::Coin, 30), (ItemKind::Apple, 2)];
        d.hotbar[0] = Some(ItemKind::Apple);
        d.hotbar[9] = Some(ItemKind::Shield);
        d.weapon = Some(ItemKind::Axe);
        d.shield = true;
        d.rim = true;
        d.lantern = true;
        d.backpack = Some(ItemKind::LeatherPack);
        d.charm = Some(ItemKind::CarryBelt);
        d.effects = vec![
            Effect {
                source: "Pozione del mulo".into(),
                carry_kg: 20.0,
                remaining: Some(12.5),
            },
            Effect {
                source: "Benedizione".into(),
                carry_kg: 5.0,
                remaining: None,
            },
        ];
        d.stage = QuestStage::Collecting;
        d.coins_collected = 3;
        d.accepted = true;
        d.thief_spotted = true;
        d.doors = vec![
            ("casetta:porta".into(), true),
            ("porta:208:0".into(), false),
        ];
        d.containers = vec![
            ContainerSave {
                key: "301:0:0".into(),
                kind: ContainerKind::Crate,
                items: vec![(ItemKind::Apple, 4), (ItemKind::Coin, 3)],
            },
            ContainerSave {
                key: "casetta:baule".into(),
                kind: ContainerKind::Chest,
                items: Vec::new(),
            },
        ];
        d.pickups = vec![PickupSave {
            kind: ItemKind::Shield,
            qty: 1,
            pos: Vec3::new(4.0, 0.5, 2.0),
            dropped: true,
        }];
        d.quest_coins = vec![Vec3::new(30.0, 1.0, -30.0)];
        d.enemies = vec![EnemySave {
            thief: true,
            pos: Vec3::new(31.0, 2.0, -34.0),
            hp: 12.5,
            hp_max: 30.0,
            a: Vec3::new(31.2, 2.0, -34.9),
            b: Vec3::new(27.2, 2.0, -34.9),
            current: 1,
        }];
        d.respawns = vec![RespawnSave {
            timer: 9.5,
            a: Vec3::X,
            b: Vec3::Z,
        }];
        d.villagers = vec![VillagerSave {
            name: "Marta".into(),
            phase: "Night".into(),
            hidden: true,
            pos: Vec3::new(4.5, 3.0, 17.9),
            rot: Quat::IDENTITY,
            patrol: 0.0,
        }];
        d.keeper_asleep = true;
        d.keeper_stove_manual = true;
        d.companion = Some(("Guardia Dino".into(), 64.0));
        d.wounded = vec![("Silvia".into(), 812)];
        d
    }

    #[test]
    fn a_save_reads_back_identical() {
        let original = sample();
        let mut parsed = SaveData::parse(&original.to_text()).expect("leggibile");
        assert!(parsed.present.contains("nemici"));
        parsed.present.clear();
        assert_eq!(parsed, original);
    }

    #[test]
    fn a_save_from_0_0_62_moves_the_mission_steps_once() {
        let mut d = sample();
        d.game_version = "v0.0.62.0".into();
        d.missions = vec![("mp01".into(), 1), ("mp03".into(), 5), ("mp05".into(), 2)];
        let mut island = d.clone();
        island.region = Region::Isola;
        d.region = Region::Nord;
        d.elsewhere = vec![(Region::Isola, Box::new(island))];
        let once = SaveData::parse(&d.to_text()).unwrap();
        let moved = vec![("mp01".into(), 2), ("mp03".into(), 6), ("mp05".into(), 2)];
        assert_eq!(once.missions, moved);
        assert_eq!(once.elsewhere[0].1.missions, moved);
        // Riscritto e riletto, non si sposta di nuovo (anche la regione annidata).
        let twice = SaveData::parse(&once.to_text()).unwrap();
        assert_eq!(twice.missions, moved);
        assert_eq!(twice.elsewhere[0].1.missions, moved);
    }

    #[test]
    fn a_save_from_0_0_64_keeps_finished_side_missions_finished() {
        let mut d = sample();
        d.game_version = "v0.0.64.0".into();
        // qs09 e qs14 compiute (3 e 4 tappe allora), qs12 ferma al pozzo (tappa 6 di 6).
        d.missions = vec![("qs09".into(), 3), ("qs12".into(), 5), ("qs14".into(), 4)];
        let once = SaveData::parse(&d.to_text()).unwrap();
        assert_eq!(
            once.missions,
            vec![("qs09".into(), 6), ("qs12".into(), 6), ("qs14".into(), 7)]
        );
        let twice = SaveData::parse(&once.to_text()).unwrap();
        assert_eq!(twice.missions, once.missions);
    }

    #[test]
    fn broken_lines_are_skipped_not_fatal() {
        let mut text = sample().to_text();
        text.push_str("\n[inventario]\nOggetto inventato = 3\nMela = tante\n");
        text.push_str("[sezione_ignota]\nriga = qualcosa\n");
        let parsed = SaveData::parse(&text).expect("le righe errate non bloccano il file");
        assert_eq!(parsed.inventory.len(), 2);
    }

    #[test]
    fn hand_edits_are_honoured() {
        let text = sample().to_text().replace("ora = 21.500", "ora = 7.000");
        assert_eq!(SaveData::parse(&text).unwrap().hour, 7.0);
    }

    #[test]
    fn a_file_without_a_name_is_refused() {
        assert!(SaveData::parse("[partita]\ntipo = manuale\n").is_err());
        assert!(SaveData::parse("testo a caso").is_err());
    }

    #[test]
    fn folder_names_are_safe() {
        assert_eq!(folder_name("Àlda Rossi").as_deref(), Some("Àlda Rossi"));
        assert_eq!(folder_name("../../etc").as_deref(), Some("etc"));
        assert_eq!(folder_name("a/b:c").as_deref(), Some("a_b_c"));
        assert_eq!(folder_name("   ").as_deref(), None);
        assert_eq!(folder_name("CON").as_deref(), Some("_CON"));
        assert_eq!(folder_name(&"x".repeat(60)), None);
    }

    #[test]
    fn hotbar_keys_map_to_slots() {
        assert_eq!(slot_label(0), 1);
        assert_eq!(slot_label(9), 0);
        assert_eq!(slot_index(0), Some(9));
        assert_eq!(slot_index(1), Some(0));
        assert_eq!(slot_index(12), None);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("planet-one-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn automatic_saves_rotate_over_three_files() {
        let dir = temp_dir("auto");
        for _ in 0..7 {
            let path = target_path(&dir, SaveKind::Auto, "");
            std::fs::write(&path, "x").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_newest_save_comes_first_and_names_are_case_insensitive() {
        let root = temp_dir("list");
        let mut older = sample();
        older.name = "Primo".into();
        let mut newer = sample();
        newer.name = "Secondo".into();
        for (d, dir) in [(&older, "Primo"), (&newer, "Secondo")] {
            write_atomic(&root.join(dir).join("rapido.txt"), &d.to_text()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        let list = list_saves(&root);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].data.name, "Secondo");
        assert!(name_taken(&root, "primo"));
        assert!(!name_taken(&root, "Terzo"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
