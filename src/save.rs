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
use crate::villagers::{Villager, VillagerSave};
use crate::weather::{Kind as WeatherKind, WeatherState};
use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
    pub inventory: Vec<(ItemKind, u32)>,
    pub hotbar: [Option<ItemKind>; 10],
    pub weapon: Option<ItemKind>,
    pub shield: bool,
    pub backpack: Option<ItemKind>,
    pub charm: Option<ItemKind>,
    pub effects: Vec<Effect>,
    pub stage: QuestStage,
    pub coins_collected: u32,
    pub accepted: bool,
    pub final_done: bool,
    pub thief_spotted: bool,
    pub doors: Vec<(String, bool)>,
    pub containers: Vec<ContainerSave>,
    pub pickups: Vec<PickupSave>,
    pub quest_coins: Vec<Vec3>,
    pub enemies: Vec<EnemySave>,
    pub respawns: Vec<RespawnSave>,
    pub villagers: Vec<VillagerSave>,
    pub keeper_asleep: bool,
    pub keeper_stove_manual: bool,
    /// Sezioni trovate nel file: quelle mancanti non toccano il mondo.
    pub present: HashSet<String>,
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
            inventory: Vec::new(),
            hotbar: [None; 10],
            weapon: None,
            shield: false,
            backpack: None,
            charm: None,
            effects: Vec::new(),
            stage: QuestStage::Idle,
            coins_collected: 0,
            accepted: false,
            final_done: false,
            thief_spotted: false,
            doors: Vec::new(),
            containers: Vec::new(),
            pickups: Vec::new(),
            quest_coins: Vec::new(),
            enemies: Vec::new(),
            respawns: Vec::new(),
            villagers: Vec::new(),
            keeper_asleep: false,
            keeper_stove_manual: false,
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
        line(String::new());
        line("[personaggio]".into());
        line(format!("nome = {}", self.name));
        line(format!("classe = {}", self.class.name()));
        for stat in Stat::ALL {
            line(format!("{} = {}", stat.name(), self.stats.get(stat)));
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
        line(format!("zaino = {}", item_or_none(self.backpack)));
        line(format!("incantato = {}", item_or_none(self.charm)));
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
        t
    }

    /// Legge un file. Fallisce solo se mancano i dati indispensabili.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut d = Self::blank();
        let mut section = String::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.trim().to_lowercase();
                d.present.insert(section.clone());
                continue;
            }
            if d.read_line(&section, line).is_none() {
                warn!("[SAVE] riga ignorata in [{section}]: {line}");
            }
        }
        if !d.present.contains("partita") || d.name.trim().is_empty() {
            return Err("file non valido: mancano partita o nome del personaggio".to_string());
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
                    _ => self.stats.set(Stat::parse(k)?, v.parse().ok()?),
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
                    "zaino" => self.backpack = read_item_or_none(v)?,
                    "incantato" => self.charm = read_item_or_none(v)?,
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
                    _ => return None,
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

/// Data e ora correnti (UTC), per il file e per l'elenco.
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
}

/// Risorse del personaggio.
#[derive(SystemParam)]
pub struct HeroParams<'w, 's> {
    character: ResMut<'w, Character>,
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
    containers: ResMut<'w, Containers>,
    village: Res<'w, crate::village::VillageInfo>,
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
}

/// Nome del luogo per l'elenco dei salvataggi.
fn place_name(village: &crate::village::VillageInfo, p: Vec3) -> &'static str {
    if crate::village::in_polygon(&village.poly, Vec2::new(p.x, p.z)) {
        "Fontevivo"
    } else if p.z > 75.0 {
        "Porto"
    } else {
        "Campagna"
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
    d.game_version = crate::version::GAME_VERSION.to_string();
    d.saved_at = now_text();
    d.played_secs = session.played;
    d.place = place_name(&world.village, pos).to_string();
    d.name = hero.character.name.clone();
    d.class = hero.character.class;
    d.stats = hero.character.stats;
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
    d.backpack = hero.equip.backpack;
    d.charm = hero.equip.charm;
    d.effects = hero.effects.0.clone();
    d.stage = world.quest.stage;
    d.coins_collected = world.quest.coins_collected;
    d.accepted = world.quest.accepted;
    d.final_done = world.quest.final_done;
    d.thief_spotted = world.quest.thief_spotted;
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
    if has("personaggio") {
        *hero.character = Character {
            name: d.name.clone(),
            class: d.class,
            stats: d.stats,
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
        hero.equip.backpack = d.backpack;
        hero.equip.charm = d.charm;
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

/// Sistema: esegue le richieste di salvataggio, poi l'eventuale uscita.
#[allow(clippy::too_many_arguments)]
pub fn process_save(
    mut request: ResMut<SaveRequest>,
    mut session: ResMut<Session>,
    hero: HeroParams,
    world: WorldParams,
    mut next: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
    mut closing: MessageReader<WindowCloseRequested>,
) {
    if closing.read().count() > 0 && session.active {
        request.kind = Some(SaveKind::Auto);
    }
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
    let ready = homes.built
        && field.is_some()
        && !hero.players.is_empty()
        && world.props.iter().next().is_some();
    if !ready {
        return;
    }
    match std::mem::take(&mut *load) {
        LoadRequest::None => {}
        LoadRequest::New(character) => {
            start_session(&mut session, &character.name, 0.0);
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
                t.translation = Vec3::new(0.0, 1.0, 0.0);
                hp.max = hero.character.stats.max_health();
                hp.current = hp.max;
            }
            request.kind = Some(SaveKind::Auto);
            info!("[SAVE] nuova partita: {}", hero.character.name);
        }
        LoadRequest::File(data) => {
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
                    ui.label(bevy_egui::egui::RichText::new(&text).size(18.0));
                });
        });
    session.notice = (left > dt).then_some((text, left - dt));
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Session>()
        .init_resource::<SaveRequest>()
        .init_resource::<LoadRequest>()
        .add_systems(
            Update,
            (
                session_tick.run_if(in_state(AppState::Playing)),
                process_save,
                finish_loading.run_if(in_state(AppState::Loading)),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SaveData {
        let mut d = SaveData::blank();
        d.kind = SaveKind::Quick;
        d.game_version = "v0.0.36.0".into();
        d.saved_at = "2026-11-09 12:30 UTC".into();
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
