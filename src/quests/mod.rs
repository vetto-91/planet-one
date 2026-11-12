//! Sistema quest: ogni quest vive in una propria cartella.
//!
//! ```text
//! src/quests/<quest>/mod.rs                 logica (stadi, sistemi, spawn)
//! assets/quests/<quest>/dialoghi.txt        tutti i testi, modificabili a mano
//! ```
//!
//! Qui stanno le parti comuni: parser dei testi ([`Dialogs`]), voci del diario
//! ([`QuestEntry`], [`ObjState`]), fumetti sopra la testa ([`Barks`]) e il
//! registro delle quest. Per aggiungere una quest: nuova cartella, un `mod` qui
//! e una riga in [`journal_entries`] / [`npc_dialog`].

pub mod denaro_rubato;
pub mod missions;

use bevy::prelude::*;
use denaro_rubato::QuestState;
use std::collections::HashMap;

/// Durata (s) di un fumetto.
pub const BARK_SECS: f32 = 3.0;

// ---------------------------------------------------------------------------
// Testi (dialoghi.txt)
// ---------------------------------------------------------------------------

/// Una sezione del file: righe di testo e scelte (righe che iniziano con `?`).
#[derive(Default, Clone)]
struct Section {
    lines: Vec<String>,
    choices: Vec<String>,
}

/// Testi di una quest, letti da `dialoghi.txt`.
#[derive(Default, Clone)]
pub struct Dialogs {
    sections: HashMap<String, Section>,
    /// Mappa battuta -> clip audio (da `voci.txt`): chiave `sezione.riga`.
    voices: HashMap<String, String>,
}

impl Dialogs {
    /// Parser del formato: `[nome]`, righe di testo, `? scelta`, `# commento`.
    fn parse(text: &str) -> HashMap<String, Section> {
        let mut sections: HashMap<String, Section> = HashMap::new();
        let mut current: Option<String> = None;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                current = Some(name.trim().to_string());
                sections.entry(name.trim().to_string()).or_default();
            } else if let Some(sec) = current.as_ref().and_then(|c| sections.get_mut(c)) {
                match line.strip_prefix('?') {
                    Some(choice) => sec.choices.push(choice.trim().to_string()),
                    None => sec.lines.push(line.to_string()),
                }
            }
        }
        sections
    }

    /// Parser di `voci.txt`: righe `chiave = clip`, `#` commento.
    fn parse_voices(text: &str) -> HashMap<String, String> {
        text.lines()
            .filter_map(|l| {
                let l = l.trim();
                if l.is_empty() || l.starts_with('#') {
                    return None;
                }
                let (k, v) = l.split_once('=')?;
                Some((k.trim().to_string(), v.trim().to_string()))
            })
            .collect()
    }

    /// Carica i testi: prima la copia incorporata (default), poi il file su
    /// disco (se esiste) sovrascrive le sezioni che contiene. Stesso schema per
    /// la mappa delle voci (`voci.txt`).
    pub fn load(quest_id: &str, embedded: &str, embedded_voices: &str) -> Self {
        let mut sections = Self::parse(embedded);
        let rel = format!("assets/quests/{quest_id}/dialoghi.txt");
        let candidates = [
            std::path::PathBuf::from(&rel),
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(&rel),
        ];
        match candidates
            .iter()
            .find_map(|p| std::fs::read_to_string(p).ok())
        {
            Some(file) => {
                let mut n = 0;
                for (name, sec) in Self::parse(&file) {
                    sections.insert(name, sec);
                    n += 1;
                }
                info!("[QUEST] testi di '{quest_id}' letti da {rel} ({n} sezioni)");
            }
            None => info!("[QUEST] {rel} non trovato: uso i testi incorporati"),
        }
        let mut voices = Self::parse_voices(embedded_voices);
        let vrel = format!("assets/quests/{quest_id}/voci.txt");
        let vcandidates = [
            std::path::PathBuf::from(&vrel),
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(&vrel),
        ];
        if let Some(file) = vcandidates
            .iter()
            .find_map(|p| std::fs::read_to_string(p).ok())
        {
            voices.extend(Self::parse_voices(&file));
        }
        Self { sections, voices }
    }

    /// Clip audio (relativa a `assets/audio/voci/`, senza estensione) di una
    /// battuta: chiave `sezione.riga` (riga da 1), `direzione.<nome>`, ecc.
    pub fn voice(&self, key: &str) -> Option<&str> {
        self.voices.get(key).map(String::as_str)
    }

    /// Righe di una sezione con i segnaposto `{chiave}` sostituiti.
    pub fn lines(&self, name: &str, vars: &[(&str, String)]) -> Vec<String> {
        match self.sections.get(name) {
            Some(sec) => sec.lines.iter().map(|l| fill(l, vars)).collect(),
            None => {
                warn!("[QUEST] sezione '{name}' mancante nei dialoghi");
                Vec::new()
            }
        }
    }

    /// Scelte di una sezione (righe `?`).
    pub fn choices(&self, name: &str) -> Vec<String> {
        self.sections
            .get(name)
            .map(|s| s.choices.clone())
            .unwrap_or_default()
    }

    /// Prima riga di una sezione (per testi singoli: titoli, fumetti).
    pub fn text(&self, name: &str, vars: &[(&str, String)]) -> String {
        self.lines(name, vars)
            .into_iter()
            .next()
            .unwrap_or_default()
    }
}

fn fill(line: &str, vars: &[(&str, String)]) -> String {
    let mut out = line.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Testi di tutte le quest (resource; ricaricata dal reset scena).
#[derive(Resource, Clone)]
pub struct QuestTexts {
    pub denaro_rubato: Dialogs,
}

impl QuestTexts {
    pub fn load() -> Self {
        Self {
            denaro_rubato: Dialogs::load(
                "denaro_rubato",
                include_str!("../../assets/quests/denaro_rubato/dialoghi.txt"),
                include_str!("../../assets/quests/denaro_rubato/voci.txt"),
            ),
        }
    }
}

impl Default for QuestTexts {
    fn default() -> Self {
        Self::load()
    }
}

// ---------------------------------------------------------------------------
// Diario
// ---------------------------------------------------------------------------

/// Stato di un obiettivo (icona: spunta verde / sfera azzurra / vuoto / X rossa).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjState {
    Done,
    Current,
    Pending,
    /// Non usato da "Denaro Rubato", pronto per altre quest.
    #[allow(dead_code)]
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum QuestStatus {
    Active,
    Completed,
}

/// Voce del diario (e del pannello TAB) di una quest.
pub struct QuestEntry {
    pub title: String,
    pub summary: String,
    pub status: QuestStatus,
    pub objectives: Vec<(String, ObjState)>,
}

/// Voci del diario: solo le quest accettate. Una riga per quest registrata.
pub fn journal_entries(
    quest: &QuestState,
    texts: &QuestTexts,
    missions: &missions::Missions,
) -> Vec<QuestEntry> {
    let mut entries = Vec::new();
    entries.extend(denaro_rubato::journal_entry(quest, &texts.denaro_rubato));
    // Le missioni del Prologo, nell'ordine della storia.
    entries.extend(missions.journal(quest.stage == denaro_rubato::QuestStage::Completed));
    entries
}

/// Battute del Viandante (righe, scelte, clip vocali per riga) per lo stato corrente.
pub fn npc_dialog(
    quest: &QuestState,
    texts: &QuestTexts,
) -> (Vec<String>, Vec<String>, Vec<Vec<String>>) {
    denaro_rubato::npc_dialog(quest, &texts.denaro_rubato)
}

// ---------------------------------------------------------------------------
// Fumetti
// ---------------------------------------------------------------------------

/// Un fumetto sopra la testa di un'entità.
pub struct Bark {
    pub entity: Entity,
    pub speaker: String,
    pub text: String,
    pub remaining: f32,
}

/// Fumetti attivi (disegnati da `menu::draw_barks`).
#[derive(Resource, Default)]
pub struct Barks(pub Vec<Bark>);

impl Barks {
    /// Ha un fumetto attivo?
    pub fn has(&self, entity: Entity) -> bool {
        self.0.iter().any(|b| b.entity == entity)
    }

    pub fn say(&mut self, entity: Entity, speaker: &str, text: String) {
        self.0.retain(|b| b.entity != entity);
        self.0.push(Bark {
            entity,
            speaker: speaker.to_string(),
            text,
            remaining: BARK_SECS,
        });
    }
}

/// Sistema: scala il tempo dei fumetti e scarta quelli scaduti o orfani.
pub fn tick_barks(time: Res<Time>, mut barks: ResMut<Barks>, entities: Query<Entity>) {
    let dt = time.delta_secs();
    barks.0.retain_mut(|b| {
        b.remaining -= dt;
        b.remaining > 0.0 && entities.contains(b.entity)
    });
}
