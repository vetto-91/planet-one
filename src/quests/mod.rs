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
/// Secondi che il fumetto resta dopo la fine della sua voce.
const BARK_AFTER_VOICE: f32 = 1.2;

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

/// Le quattro famiglie di missioni del diario (ADR 0059).
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum QuestKind {
    /// La storia principale (il Prologo).
    Main,
    /// Le missioni secondarie, proposte dagli abitanti.
    Side,
    /// Gli eventi casuali legati a luoghi, ore e meteo.
    Event,
    /// Missioni generate dall'IA locale (ancora nessuna).
    Ai,
}

impl QuestKind {
    pub const ALL: [QuestKind; 4] = [Self::Main, Self::Side, Self::Event, Self::Ai];

    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "Storia principale",
            Self::Side => "Missioni secondarie",
            Self::Event => "Eventi",
            Self::Ai => "Generate dall'IA",
        }
    }
}

/// Dove porta la tappa in corso: un punto (x, z) o un abitante, cercato per nome.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    At(Vec2),
    Npc(String),
}

/// Voce del diario (e del pannello TAB) di una quest.
pub struct QuestEntry {
    /// Identificativo stabile (id della missione, `denaro_rubato`): serve a seguirla.
    pub id: String,
    pub kind: QuestKind,
    pub title: String,
    pub summary: String,
    pub status: QuestStatus,
    pub objectives: Vec<(String, ObjState)>,
    /// Dove andare per la tappa in corso, se si sa.
    pub target: Option<Target>,
    /// Regione delle coordinate di `target`: nell'altra regione un punto non vale (isola e
    /// continente hanno coordinate proprie; segnalazione 2026-11-19_22-37-08).
    pub region: crate::region::Region,
}

/// La missione seguita (bussola, segnalino e TAB). Vuota: la prima della storia ancora aperta.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct Tracked(pub Option<String>);

impl Tracked {
    /// La voce seguita: quella scelta se è ancora aperta, altrimenti la prima aperta della storia
    /// principale, poi la prima aperta di qualunque tipo.
    pub fn pick<'a>(&self, entries: &'a [QuestEntry]) -> Option<&'a QuestEntry> {
        let open = |e: &&QuestEntry| e.status == QuestStatus::Active;
        self.0
            .as_ref()
            .and_then(|id| entries.iter().filter(open).find(|e| e.id == *id))
            .or_else(|| {
                entries
                    .iter()
                    .filter(open)
                    .find(|e| e.kind == QuestKind::Main)
            })
            .or_else(|| entries.iter().find(open))
    }
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
    player: &str,
) -> (Vec<String>, Vec<String>, Vec<Vec<String>>) {
    denaro_rubato::npc_dialog(quest, &texts.denaro_rubato, player)
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

/// Le battute fuori dalle missioni (`assets/world/battute.txt`): (chi parla, chiave, battuta).
pub fn bark_lines() -> &'static [(String, String, String)] {
    static LINES: std::sync::OnceLock<Vec<(String, String, String)>> = std::sync::OnceLock::new();
    LINES.get_or_init(|| {
        crate::companions::parse_lines(include_str!("../../assets/world/battute.txt"))
    })
}

/// La battuta di `who` per un momento (`tutti` = quella che può dire ogni abitante).
pub fn bark_line(who: &str, key: &str) -> Option<&'static str> {
    bark_lines()
        .iter()
        .find(|(n, k, _)| n == who && k == key)
        .map(|(_, _, s)| s.as_str())
}

/// Una battuta che può dire chiunque (sezione `[tutti]`); vuota se manca dal file.
pub fn common_line(key: &str) -> &'static str {
    bark_line("tutti", key).unwrap_or("")
}

/// Le battute numerate `<prefisso>-<n>` di tutte le sezioni, in ordine: (chi parla, battuta).
pub fn numbered_lines(prefix: &str) -> Vec<(&'static str, &'static str)> {
    let mut out: Vec<(u32, &'static str, &'static str)> = bark_lines()
        .iter()
        .filter_map(|(who, key, text)| {
            let n = key.strip_prefix(prefix)?.strip_prefix('-')?.parse().ok()?;
            Some((n, who.as_str(), text.as_str()))
        })
        .collect();
    out.sort_by_key(|(n, ..)| *n);
    out.into_iter().map(|(_, w, t)| (w, t)).collect()
}

/// La battuta di dialogo `dialogo` di `who`, come elenco statico per `entities::Npc::lines`.
pub fn talker_lines(who: &str) -> &'static [&'static str] {
    static LISTS: std::sync::OnceLock<Vec<(String, Vec<&'static str>)>> =
        std::sync::OnceLock::new();
    LISTS
        .get_or_init(|| {
            bark_lines()
                .iter()
                .filter(|(_, k, _)| k == "dialogo")
                .map(|(w, _, t)| (w.clone(), vec![t.as_str()]))
                .collect()
        })
        .iter()
        .find(|(w, _)| w == who)
        .map_or(&[], |(_, v)| v.as_slice())
}

/// Fumetti attivi (disegnati da `menu::draw_barks`).
#[derive(Resource, Default)]
pub struct Barks(pub Vec<Bark>);

impl Barks {
    /// Ha un fumetto attivo?
    pub fn has(&self, entity: Entity) -> bool {
        self.0.iter().any(|b| b.entity == entity)
    }

    /// Un fumetto sopra `entity`; se la battuta ha la sua voce generata, si sente anche
    /// (`voice::bark_voice`).
    pub fn say(&mut self, entity: Entity, speaker: &str, text: String) {
        crate::voice::request_bark(entity, speaker, &text);
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
pub fn tick_barks(
    time: Res<Time>,
    mut barks: ResMut<Barks>,
    voice: Res<crate::voice::VoiceQueue>,
    entities: Query<Entity>,
) {
    let dt = time.delta_secs();
    // Il fumetto resta finché la sua voce parla, e poco più (segnalazione 2026-11-18_22-59-58).
    let speaking = voice.bark_speaker();
    barks.0.retain_mut(|b| {
        b.remaining -= dt;
        if speaking == Some(b.entity) {
            b.remaining = b.remaining.max(BARK_AFTER_VOICE);
        }
        b.remaining > 0.0 && entities.contains(b.entity)
    });
}
