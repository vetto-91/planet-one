//! Missioni a tappe guidate da un file di testo (ADR 0034): la storia principale del Prologo.
//!
//! `assets/quests/prologo/missioni.txt` descrive ogni missione come una fila di tappe; ogni
//! tappa ha un obiettivo, una o più condizioni (`quando:`), un eventuale dialogo con un
//! abitante e un effetto. Il motore non sa nulla delle singole storie: per aggiungere o
//! cambiare una missione basta il file. «Denaro Rubato» resta la quest scritta nel codice
//! (`denaro_rubato`) e fa da anello della catena tramite la condizione `quest`.

use super::denaro_rubato::{QuestStage, QuestState};
use super::{ObjState, QuestEntry, QuestStatus};
use bevy::prelude::*;

/// File delle missioni: la storia principale, poi le secondarie (ADR 0036).
const FILES: [(&str, &str); 3] = [
    (
        "assets/quests/prologo/missioni.txt",
        include_str!("../../assets/quests/prologo/missioni.txt"),
    ),
    (
        "assets/quests/prologo/secondarie.txt",
        include_str!("../../assets/quests/prologo/secondarie.txt"),
    ),
    (
        "assets/quests/prologo/eventi.txt",
        include_str!("../../assets/quests/prologo/eventi.txt"),
    ),
];

/// Condizione di una tappa: tutte quelle elencate devono valere insieme.
#[derive(Clone, Debug, PartialEq)]
pub enum When {
    /// Il giocatore è entro `r` metri dal punto (x, z).
    Near(Vec2, f32),
    /// Il giocatore è sotto questa quota (gallerie sotterranee).
    Below(f32),
    /// Il giocatore è sopra questa quota (cima di una torre).
    Above(f32),
    /// Ora del giorno fra `a` e `b` (anche a cavallo della mezzanotte).
    Hour(f32, f32),
    /// Il dialogo della tappa con questo abitante è stato letto fino in fondo.
    Talk(String),
    /// «Denaro Rubato»: accettata o completata.
    Quest(bool),
    /// Il giocatore ha almeno `n` di questo oggetto.
    Has(crate::inventory::ItemKind, u32),
    /// Nessun nemico vivo entro `r` metri dal punto (un campo liberato).
    Clear(Vec2, f32),
    /// Il meteo in corso è questo.
    Weather(crate::weather::Kind),
    /// Almeno `n` missioni secondarie già compiute.
    SidesDone(usize),
    /// Un contatore del mondo arriva a `n` (`serrature`, `infilate:<nome>`; ADR 0039).
    Count(String, u32),
    /// Qualcosa è stato sbloccato da una tappa o da una scelta (`quando: sbloccato rito-banditi`).
    Unlocked(String),
}

/// Effetto di una tappa o di una scelta (ADR 0044).
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Give(crate::inventory::ItemKind, u32),
    Take(crate::inventory::ItemKind, u32),
    Coins(u32),
    /// Karma globale, con il motivo mostrato a schermo.
    Karma(i32, String),
    /// Fiducia di un abitante.
    Trust(String, i32),
    /// Una statistica sale (o scende) di tanto.
    Stat(crate::character::Stat, i8),
    Unlock(String),
    /// La missione finisce qui (un bivio che chiude la storia prima delle tappe successive).
    End,
    /// Salto narrativo del tempo: ora, giorni dopo, meteo della scena (`tempo 20 10 pioggia`).
    Time(f32, u32, Option<crate::weather::Kind>),
    /// I doni di chi hai aiutato (Sera delle Lanterne).
    Gifts,
    /// Una scena raccontata e poi il viaggio verso un'altra regione (`viaggio nord mare`).
    Travel(crate::region::Region, String),
}

impl Effect {
    /// «dai scudo 1», «monete 20», «karma 3 hai restituito lo scudo», «fiducia Guardia Aldo 5»,
    /// «stat saggezza 1», «sblocca rito-banditi».
    pub fn parse(text: &str) -> Option<Effect> {
        let text = text.trim();
        let (verb, rest) = text.split_once(' ').unwrap_or((text, ""));
        let rest = rest.trim();
        Some(match verb {
            "dai" => {
                let (k, n) = item_qty(rest)?;
                Effect::Give(k, n)
            }
            "prendi" => {
                let (k, n) = item_qty(rest)?;
                Effect::Take(k, n)
            }
            "monete" => Effect::Coins(rest.parse().ok()?),
            "karma" => {
                let (n, why) = rest.split_once(' ').unwrap_or((rest, ""));
                Effect::Karma(n.parse().ok()?, why.trim().to_string())
            }
            "fiducia" => {
                let (name, n) = rest.rsplit_once(' ')?;
                Effect::Trust(name.trim().to_string(), n.parse().ok()?)
            }
            "stat" => {
                let (name, n) = rest.rsplit_once(' ')?;
                Effect::Stat(crate::character::Stat::parse(name)?, n.parse().ok()?)
            }
            "sblocca" if !rest.is_empty() => Effect::Unlock(rest.to_string()),
            "fine" => Effect::End,
            "doni" => Effect::Gifts,
            "tempo" => {
                let mut w = rest.split_whitespace();
                let hour: f32 = w.next()?.replace(',', ".").parse().ok()?;
                // «estate» al posto dei giorni: fino al primo giorno d'estate (ADR 0055).
                let days: u32 = w.next().map_or(Some(0), |d| match d {
                    "estate" => Some(UNTIL_SUMMER),
                    d => d.parse().ok(),
                })?;
                let weather = match w.next() {
                    Some(k) => Some(crate::weather::Kind::parse(k)?),
                    None => None,
                };
                Effect::Time(hour.rem_euclid(24.0), days, weather)
            }
            "viaggio" => {
                let (region, how) = rest.split_once(' ').unwrap_or((rest, ""));
                let region = crate::region::Region::ALL
                    .into_iter()
                    .find(|r| r.key() == region.trim())?;
                Effect::Travel(region, how.trim().to_string())
            }
            _ => return None,
        })
    }
}

/// Giorni di `tempo:` che vogliono dire «fino all'inizio dell'estate».
pub const UNTIL_SUMMER: u32 = u32::MAX;

/// La data dopo un salto di `days` giorni ([`UNTIL_SUMMER`]: il primo giorno d'estate, se non lo è già).
pub fn jump(mut date: crate::daynight::GameDate, days: u32) -> crate::daynight::GameDate {
    if days == UNTIL_SUMMER {
        // Solo in avanti nello stesso anno: d'estate o d'autunno il salto non serve.
        while date.month <= 6 && date.season() != crate::daynight::Season::Summer {
            date.next_day();
        }
    } else {
        for _ in 0..days {
            date.next_day();
        }
    }
    date
}

/// Una risposta fra cui scegliere alla fine del dialogo della tappa.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Choice {
    pub text: String,
    pub effects: Vec<Effect>,
    /// La risposta compare solo se la statistica arriva a tanto (`scegli: [carisma 12] testo`).
    pub requires: Option<(crate::character::Stat, u8)>,
    /// La risposta compare solo se qualcosa è sbloccato (`scegli: [sbloccato porta-sommersa] testo`).
    pub needs_unlock: Option<String>,
}

/// Oggetto dal nome italiano usato nei file («mela», «pesce», «pozione del mulo»…).
/// Come nei salvataggi: senza badare a maiuscole e accenti, e singolare o plurale («freccia»).
fn item(name: &str) -> Option<crate::inventory::ItemKind> {
    crate::inventory::ItemKind::from_name(name)
}

/// «<oggetto> <n>» (n = 1 se manca).
fn item_qty(text: &str) -> Option<(crate::inventory::ItemKind, u32)> {
    let text = text.trim();
    match text.rsplit_once(' ') {
        Some((name, n)) if n.parse::<u32>().is_ok() => Some((item(name)?, n.parse().ok()?)),
        _ => Some((item(text)?, 1)),
    }
}

/// Una tappa di una missione.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Step {
    pub goal: String,
    pub when: Vec<When>,
    /// Battute dell'abitante della condizione `parla` (segnaposto `{nome}`).
    pub lines: Vec<String>,
    /// Pensiero del protagonista mostrato al completamento.
    pub note: String,
    pub coins: u32,
    /// Oggetti consegnati (tolti dall'inventario) e ricevuti a tappa fatta.
    pub takes: Vec<(crate::inventory::ItemKind, u32)>,
    pub gives: Vec<(crate::inventory::ItemKind, u32)>,
    /// Cose sbloccate nel mondo (per esempio `continente`).
    pub unlocks: Vec<String>,
    /// Effetti in più (karma, fiducia, statistiche: ADR 0044).
    pub effects: Vec<Effect>,
    /// Risposte fra cui scegliere alla fine del dialogo (`scegli: testo => effetto; effetto`).
    pub choices: Vec<Choice>,
}

/// Una missione letta dal file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mission {
    pub id: String,
    pub title: String,
    pub summary: String,
    /// Missione che deve essere finita prima (vuoto = nessuna).
    pub after: String,
    /// Serve «Denaro Rubato» completata.
    pub after_quest: bool,
    /// Missione secondaria: compare nel diario solo dopo la prima tappa (chi la propone).
    pub side: bool,
    /// Evento casuale: un incontro legato a luogo, ora e meteo (conta come secondaria nel diario).
    pub event: bool,
    /// Si ripete: compiuta, torna da capo dopo tanti giorni (`ripeti: 7`, ADR 0053).
    pub repeat: Option<u32>,
    pub steps: Vec<Step>,
}

/// Interpreta `missioni.txt`; le righe errate vengono ignorate.
pub fn parse(text: &str) -> Vec<Mission> {
    let mut out: Vec<Mission> = Vec::new();
    let nums = |v: &str| -> Vec<f32> {
        v.split_whitespace()
            .filter_map(|t| t.replace(',', ".").parse().ok())
            .collect()
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(head) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let mut w = head.split_whitespace();
            match (w.next(), w.next()) {
                (Some("missione"), Some(id)) => out.push(Mission {
                    id: id.to_string(),
                    ..Default::default()
                }),
                (Some("tappa"), _) => {
                    if let Some(m) = out.last_mut() {
                        m.steps.push(Step::default());
                    }
                }
                _ => {}
            }
            continue;
        }
        let Some(m) = out.last_mut() else {
            continue;
        };
        // Battuta di dialogo: «> testo».
        if let Some(say) = line.strip_prefix('>') {
            if let Some(s) = m.steps.last_mut() {
                s.lines.push(say.trim().to_string());
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match (key, m.steps.last_mut()) {
            ("titolo", None) => m.title = value.to_string(),
            ("sommario", None) => m.summary = value.to_string(),
            ("ripeti", None) => m.repeat = value.parse().ok().filter(|d: &u32| *d > 0),
            ("tipo", None) => {
                m.event = value == "evento";
                m.side = value == "secondaria" || m.event;
            }
            ("dopo", None) => {
                if value == "denaro_rubato" {
                    m.after_quest = true;
                } else {
                    m.after = value.to_string();
                }
            }
            ("obiettivo", Some(s)) => s.goal = value.to_string(),
            ("pensiero", Some(s)) => s.note = value.to_string(),
            ("monete", Some(s)) => s.coins = value.parse().unwrap_or(0),
            ("sblocca", Some(s)) => s.unlocks.push(value.to_string()),
            ("doni", Some(s)) => s.effects.push(Effect::Gifts),
            ("karma" | "fiducia" | "stat" | "tempo", Some(s)) => {
                match Effect::parse(&format!("{key} {value}")) {
                    Some(e) => s.effects.push(e),
                    None => warn!("[MISSIONI] effetto non valido: {line}"),
                }
            }
            ("scegli", Some(s)) => {
                let (text, effects) = value.split_once("=>").unwrap_or((value, ""));
                // «[carisma 12] testo»: la risposta chiede una statistica.
                let (requires, text) = match text
                    .trim()
                    .strip_prefix('[')
                    .and_then(|t| t.split_once(']'))
                {
                    Some((need, rest)) if need.trim().starts_with("sbloccato ") => (None, rest),
                    Some((need, rest)) => {
                        let need = need.trim().rsplit_once(' ').and_then(|(stat, n)| {
                            Some((crate::character::Stat::parse(stat)?, n.parse().ok()?))
                        });
                        if need.is_none() {
                            warn!("[MISSIONI] requisito non valido: {line}");
                        }
                        (need, rest)
                    }
                    None => (None, text),
                };
                let needs_unlock = value
                    .trim()
                    .strip_prefix("[sbloccato ")
                    .and_then(|t| t.split_once(']'))
                    .map(|(what, _)| what.trim().to_string());
                let effects: Vec<Effect> = effects
                    .split(';')
                    .filter(|e| !e.trim().is_empty())
                    .filter_map(|e| {
                        let parsed = Effect::parse(e);
                        if parsed.is_none() {
                            warn!("[MISSIONI] effetto non valido nella scelta: {e}");
                        }
                        parsed
                    })
                    .collect();
                s.choices.push(Choice {
                    text: text.trim().to_string(),
                    effects,
                    requires,
                    needs_unlock,
                });
            }
            ("prendi", Some(s)) => match item_qty(value) {
                Some(x) => s.takes.push(x),
                None => warn!("[MISSIONI] oggetto sconosciuto: {line}"),
            },
            ("dai", Some(s)) => match item_qty(value) {
                Some(x) => s.gives.push(x),
                None => warn!("[MISSIONI] oggetto sconosciuto: {line}"),
            },
            ("quando", Some(s)) => {
                let (kind, rest) = value.split_once(' ').unwrap_or((value, ""));
                let n = nums(rest);
                let cond = match (kind, n.as_slice()) {
                    ("vicino", [x, z, r]) => Some(When::Near(Vec2::new(*x, *z), *r)),
                    ("libero", [x, z, r]) => Some(When::Clear(Vec2::new(*x, *z), *r)),
                    ("ha", _) => item_qty(rest).map(|(k, n)| When::Has(k, n)),
                    ("meteo", _) => crate::weather::Kind::parse(rest).map(When::Weather),
                    ("fatte", [n]) => Some(When::SidesDone(*n as usize)),
                    ("sbloccato", _) if !rest.trim().is_empty() => {
                        Some(When::Unlocked(rest.trim().to_string()))
                    }
                    ("conta", _) => rest.trim().rsplit_once(' ').and_then(|(key, n)| {
                        Some(When::Count(key.trim().to_lowercase(), n.parse().ok()?))
                    }),
                    ("sotto", [y]) => Some(When::Below(*y)),
                    ("sopra", [y]) => Some(When::Above(*y)),
                    ("ora", [a, b]) => Some(When::Hour(*a, *b)),
                    ("parla", _) if !rest.trim().is_empty() => {
                        Some(When::Talk(rest.trim().to_string()))
                    }
                    ("quest", _) => match rest.trim() {
                        "accettata" => Some(When::Quest(false)),
                        "completata" => Some(When::Quest(true)),
                        _ => None,
                    },
                    _ => None,
                };
                match cond {
                    Some(c) => s.when.push(c),
                    None => warn!("[MISSIONI] condizione ignorata: {line}"),
                }
            }
            _ => {}
        }
    }
    out
}

/// Stato delle missioni della partita.
#[derive(Resource)]
pub struct Missions {
    pub list: Vec<Mission>,
    /// Per ogni missione: tappa corrente (uguale al numero di tappe = finita).
    pub progress: Vec<usize>,
    /// Le missioni avanzano solo in una partita vera: nelle prove automatiche restano ferme
    /// (e il passaggio di Gaspare è libero) finché il copione non scrive `missioni on`.
    pub enabled: bool,
    /// Abitante il cui dialogo di missione è aperto: si completa leggendolo fino in fondo.
    talking: Option<(String, usize)>,
    /// Risposte mostrate nel dialogo aperto: indice nella tappa (alcune possono essere nascoste).
    shown: Vec<usize>,
    /// Battute del dialogo aperto così come sono nel file (per trovare le voci).
    templates: Vec<String>,
    /// Dialoghi di missione letti fino in fondo (nome dell'abitante), in attesa di essere contati.
    talked: Vec<(String, usize)>,
    /// Risposta scelta alla fine di un dialogo di missione (missione, risposta), in attesa.
    picked: Vec<(usize, usize)>,
    /// Risposte date nelle tappe già fatte: (missione, tappa) → risposta.
    pub chosen: std::collections::BTreeMap<(usize, usize), usize>,
    /// Contatori del mondo dell'ultimo frame (servono anche ai dialoghi, vedi `dialog_for`).
    counters: Vec<(String, u32)>,
    /// Giorno (`GameDate::day_key`) in cui una missione che si ripete è stata compiuta.
    pub done_on: std::collections::BTreeMap<String, u32>,
}

impl Default for Missions {
    fn default() -> Self {
        let mut list = Vec::new();
        for (file, embedded) in FILES {
            let text = [
                std::path::PathBuf::from(file),
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file),
            ]
            .iter()
            .find_map(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_else(|| embedded.to_string());
            list.extend(parse(&text));
        }
        Self {
            progress: vec![0; list.len()],
            list,
            enabled: false,
            talking: None,
            shown: Vec::new(),
            templates: Vec::new(),
            talked: Vec::new(),
            picked: Vec::new(),
            chosen: Default::default(),
            counters: Vec::new(),
            done_on: Default::default(),
        }
    }
}

/// Cosa serve sapere del mondo per valutare le condizioni.
pub struct Facts {
    pub pos: Vec3,
    pub hour: f32,
    pub quest_accepted: bool,
    pub quest_done: bool,
    /// Oggetti nell'inventario e nemici vivi (posizioni).
    pub items: Vec<(crate::inventory::ItemKind, u32)>,
    pub enemies: Vec<Vec3>,
    /// Meteo in corso (`None` dove non serve saperlo).
    pub weather: Option<crate::weather::Kind>,
    /// Missioni secondarie compiute (lo riempie il motore).
    pub sides_done: usize,
    /// Contatori del mondo: serrature forzate, monete infilate (`thievery::Thievery::counters`).
    pub counters: Vec<(String, u32)>,
    /// Cose sbloccate (lo riempie il motore).
    pub unlocked: Vec<String>,
    /// Statistiche del personaggio (per le risposte che le chiedono; `None` = tutte valide).
    pub stats: Option<crate::character::Stats>,
    /// Giorno di gioco (`GameDate::day_key`), per le missioni che si ripetono.
    pub day: u32,
    /// Età del personaggio (`giovane`, `adulto`, `maturo`): sceglie le battute col tono giusto
    /// (`> [giovane] …`, ADR 0047). `None` = tutte.
    pub age: Option<&'static str>,
}

/// Età a cui è rivolta una battuta (`[giovane] testo`) e il testo senza l'etichetta.
pub fn age_line(line: &str) -> (Option<&str>, &str) {
    if let Some(rest) = line.strip_prefix('[')
        && let Some((tag, text)) = rest.split_once(']')
        && crate::character::Age::parse(tag).is_some()
    {
        return (Some(tag.trim()), text.trim());
    }
    (None, line)
}

#[cfg(test)]
#[test]
fn age_lines_pick_the_tone() {
    let lines: Vec<String> = [
        "Ciao",
        "[giovane] Ragazzo!",
        "[maturo] Signore.",
        "[x] resta",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(
        lines_for_age(&lines, Some("giovane")),
        ["Ciao", "Ragazzo!", "[x] resta"]
    );
    assert_eq!(lines_for_age(&lines, Some("adulto")), ["Ciao", "[x] resta"]);
    assert_eq!(lines_for_age(&lines, None).len(), 4);
}

/// Le battute per un personaggio di questa età: quelle senza etichetta e quelle della sua età,
/// senza l'etichetta.
pub fn lines_for_age(lines: &[String], age: Option<&str>) -> Vec<String> {
    lines
        .iter()
        .filter_map(|l| match age_line(l) {
            (Some(tag), text) => (age.is_none_or(|a| a == tag)).then(|| text.to_string()),
            (None, text) => Some(text.to_string()),
        })
        .collect()
}

impl Missions {
    /// Il dialogo aperto è una battuta di missione.
    pub fn in_dialog(&self) -> bool {
        self.talking.is_some()
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.list.iter().position(|m| m.id == id)
    }

    /// Quante missioni secondarie (non gli eventi) sono compiute.
    pub fn sides_done(&self) -> usize {
        self.list
            .iter()
            .zip(&self.progress)
            .filter(|(m, p)| m.side && !m.event && **p >= m.steps.len())
            .count()
    }

    /// Eventi non ancora vissuti che capitano entro `reach` metri da `at` (il segreto di un luogo:
    /// conta per dire che il luogo è esplorato del tutto).
    pub fn events_near(&self, at: Vec2, reach: f32) -> usize {
        self.list
            .iter()
            .zip(&self.progress)
            .filter(|(m, p)| m.event && **p < m.steps.len())
            .filter(|(m, _)| {
                m.steps.iter().flat_map(|s| &s.when).any(|w| match w {
                    When::Near(p, _) => p.distance(at) <= reach,
                    _ => false,
                })
            })
            .count()
    }

    /// Tappa corrente di una missione (per chi deve reagire a un punto preciso della storia).
    pub fn step_of(&self, id: &str) -> Option<usize> {
        self.index(id).map(|i| self.progress[i])
    }

    pub fn done(&self, id: &str) -> bool {
        self.index(id)
            .is_some_and(|i| self.progress[i] >= self.list[i].steps.len())
    }

    /// La missione `i` è cominciata (le precedenti sono finite)?
    fn started(&self, i: usize, quest_done: bool) -> bool {
        let m = &self.list[i];
        (m.after.is_empty() || self.done(&m.after)) && (!m.after_quest || quest_done)
    }

    /// Qualcosa di sbloccato da una tappa già completata (per esempio `continente`).
    /// Tappe fatte davvero della missione `i`: una scelta con `fine` chiude la missione, ma le
    /// tappe saltate non valgono (non sbloccano nulla).
    fn reached(&self, i: usize) -> usize {
        let ended = self
            .chosen
            .iter()
            .filter(|((m, _), _)| *m == i)
            .filter(|((m, s), c)| {
                self.list[*m]
                    .steps
                    .get(*s)
                    .and_then(|st| st.choices.get(**c))
                    .is_some_and(|ch| ch.effects.contains(&Effect::End))
            })
            .map(|((_, s), _)| s + 1)
            .min();
        self.progress[i]
            .min(self.list[i].steps.len())
            .min(ended.unwrap_or(usize::MAX))
    }

    pub fn unlocked(&self, what: &str) -> bool {
        let by_step = self.list.iter().enumerate().any(|(i, m)| {
            m.steps[..self.reached(i)].iter().any(|s| {
                s.unlocks.iter().any(|u| u == what)
                    || s.effects.contains(&Effect::Unlock(what.to_string()))
            })
        });
        by_step
            || self.chosen.iter().any(|((m, s), c)| {
                self.list[*m]
                    .steps
                    .get(*s)
                    .and_then(|step| step.choices.get(*c))
                    .is_some_and(|ch| ch.effects.contains(&Effect::Unlock(what.to_string())))
            })
    }

    /// Tutte le cose sbloccate finora (per la condizione `sbloccato`).
    fn unlocked_all(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (i, m) in self.list.iter().enumerate() {
            for (k, s) in m.steps[..self.reached(i)].iter().enumerate() {
                out.extend(s.unlocks.iter().cloned());
                let chosen = self
                    .chosen
                    .get(&(i, k))
                    .and_then(|c| s.choices.get(*c))
                    .map(|c| c.effects.as_slice())
                    .unwrap_or_default();
                for e in s.effects.iter().chain(chosen) {
                    if let Effect::Unlock(u) = e {
                        out.push(u.clone());
                    }
                }
            }
        }
        out
    }

    /// Tappa corrente di ogni missione cominciata e non finita.
    fn current(&self, quest_done: bool) -> impl Iterator<Item = (usize, &Step)> {
        (0..self.list.len()).filter_map(move |i| {
            let step = self.list[i].steps.get(self.progress[i])?;
            self.started(i, quest_done).then_some((i, step))
        })
    }

    /// Battute che `npc` deve dire adesso per una tappa `parla` (le altre condizioni della
    /// tappa devono già valere). Apre il dialogo di missione.
    pub fn dialog_for(
        &mut self,
        npc: &str,
        facts: &mut Facts,
        player: &str,
    ) -> Option<(Vec<String>, Vec<String>)> {
        if !self.enabled {
            return None;
        }
        facts.sides_done = self.sides_done();
        facts.unlocked.extend(self.unlocked_all());
        if facts.counters.is_empty() {
            facts.counters = self.counters.clone();
        }
        let facts = &*facts;
        let (mission, lines, choices) = self.current(facts.quest_done).find_map(|(i, step)| {
            let mine = step
                .when
                .iter()
                .any(|w| matches!(w, When::Talk(n) if n == npc));
            let rest = step
                .when
                .iter()
                .filter(|w| !matches!(w, When::Talk(_)))
                .all(|w| holds(w, facts, &[], i));
            (mine && rest && !step.lines.is_empty()).then(|| {
                // Le risposte che chiedono una statistica troppo alta non si vedono.
                let choices: Vec<(usize, String)> = step
                    .choices
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| {
                        c.requires
                            .is_none_or(|(stat, n)| facts.stats.is_none_or(|s| s.get(stat) >= n))
                            && c.needs_unlock
                                .as_ref()
                                .is_none_or(|u| facts.unlocked.contains(u))
                    })
                    .map(|(k, c)| (k, c.text.clone()))
                    .collect();
                (i, lines_for_age(&step.lines, facts.age), choices)
            })
        })?;
        self.shown = choices.iter().map(|(k, _)| *k).collect();
        self.templates = lines.clone();
        let choices: Vec<String> = choices.into_iter().map(|(_, t)| t).collect();
        // Il dialogo vale per questa missione sola: un altro incarico dello stesso abitante
        // aspetta la prossima volta che gli si parla.
        self.talking = Some((npc.to_string(), mission));
        Some((
            lines.iter().map(|l| l.replace("{nome}", player)).collect(),
            choices,
        ))
    }

    /// Le voci delle battute dell'ultimo dialogo aperto da `dialog_for` (ADR 0050).
    pub fn voices(&self, npc: &str) -> Vec<Vec<String>> {
        self.templates
            .iter()
            .map(|l| crate::voice::line_clip(npc, l))
            .collect()
    }

    /// Il dialogo si è chiuso: se era di missione ed è stato letto tutto, vale come «parlato».
    /// `choice` è la risposta scelta, se la tappa ne chiedeva una.
    pub fn dialog_closed(&mut self, finished: bool, choice: Option<usize>) {
        if let Some((npc, mission)) = self.talking.take()
            && finished
        {
            if let Some(c) = choice {
                // Dall'indice mostrato a quello della tappa.
                let c = self.shown.get(c).copied().unwrap_or(c);
                self.picked.push((mission, c));
            }
            self.talked.push((npc, mission));
        }
    }

    /// Avanza le tappe le cui condizioni valgono; restituisce quelle appena completate.
    pub fn advance(&mut self, facts: &mut Facts) -> Vec<(usize, Step, bool)> {
        let mut done = Vec::new();
        if !self.enabled {
            return done;
        }
        self.repeat_due(facts.day);
        facts.sides_done = self.sides_done();
        facts.unlocked.extend(self.unlocked_all());
        self.counters = facts.counters.clone();
        let facts = &*facts;
        for i in 0..self.list.len() {
            let Some(step) = self.list[i].steps.get(self.progress[i]) else {
                continue;
            };
            if !self.started(i, facts.quest_done) || step.when.is_empty() {
                continue;
            }
            if step.when.iter().all(|w| holds(w, facts, &self.talked, i)) {
                let mut step = step.clone();
                // La risposta scelta aggiunge i suoi effetti a quelli della tappa.
                if !step.choices.is_empty() {
                    let c = self
                        .picked
                        .iter()
                        .find(|(m, _)| *m == i)
                        .map_or(0, |(_, c)| *c)
                        .min(step.choices.len() - 1);
                    self.chosen.insert((i, self.progress[i]), c);
                    step.effects.extend(step.choices[c].effects.iter().cloned());
                }
                self.progress[i] += 1;
                if step.effects.contains(&Effect::End) {
                    self.progress[i] = self.list[i].steps.len();
                }
                let finished = self.progress[i] >= self.list[i].steps.len();
                done.push((i, step, finished));
            }
        }
        self.talked.clear();
        self.picked.clear();
        done
    }

    /// Voci del diario: missioni cominciate, con le tappe fatte e quella in corso.
    pub fn journal(&self, quest_done: bool) -> Vec<QuestEntry> {
        if !self.enabled {
            return Vec::new();
        }
        (0..self.list.len())
            .filter(|i| self.started(*i, quest_done))
            // Una secondaria esiste per il giocatore solo dopo che gliel'hanno proposta.
            .filter(|i| !self.list[*i].side || self.progress[*i] > 0)
            .map(|i| {
                let (m, p) = (&self.list[i], self.progress[i]);
                let finished = p >= m.steps.len();
                QuestEntry {
                    title: m.title.clone(),
                    summary: m.summary.clone(),
                    status: if finished {
                        QuestStatus::Completed
                    } else {
                        QuestStatus::Active
                    },
                    objectives: m
                        .steps
                        .iter()
                        .enumerate()
                        .filter(|(k, _)| *k <= p)
                        .map(|(k, s)| {
                            let state = if k < p {
                                ObjState::Done
                            } else {
                                ObjState::Current
                            };
                            (s.goal.clone(), state)
                        })
                        .collect(),
                }
            })
            .collect()
    }

    /// Stato da salvare: (missione, tappa).
    pub fn snapshot(&self) -> Vec<(String, usize)> {
        self.list
            .iter()
            .zip(&self.progress)
            .map(|(m, p)| (m.id.clone(), *p))
            .collect()
    }

    pub fn restore(&mut self, saved: &[(String, usize)]) {
        for (id, p) in saved {
            if let Some(i) = self.index(id) {
                self.progress[i] = (*p).min(self.list[i].steps.len());
            }
        }
    }

    /// Risposte date, da salvare: (missione, tappa, risposta).
    /// Le missioni che si ripetono: compiute, si segna il giorno; passati i loro giorni tornano da
    /// capo, con le risposte di allora dimenticate (i loro sblocchi spariscono con loro).
    pub fn repeat_due(&mut self, day: u32) {
        for i in 0..self.list.len() {
            let Some(days) = self.list[i].repeat else {
                continue;
            };
            let id = self.list[i].id.clone();
            if self.progress[i] < self.list[i].steps.len() {
                continue;
            }
            match self.done_on.get(&id).copied() {
                None => {
                    self.done_on.insert(id, day);
                }
                Some(on) if day >= on + days => {
                    self.progress[i] = 0;
                    self.chosen.retain(|(m, _), _| *m != i);
                    self.done_on.remove(&id);
                    info!("[MISSIONE] {id}: torna da capo dopo {days} giorni");
                }
                _ => {}
            }
        }
    }

    pub fn choices_snapshot(&self) -> Vec<(String, usize, usize)> {
        self.chosen
            .iter()
            .map(|((m, s), c)| (self.list[*m].id.clone(), *s, *c))
            .collect()
    }

    pub fn restore_choices(&mut self, saved: &[(String, usize, usize)]) {
        self.chosen.clear();
        for (id, step, choice) in saved {
            if let Some(i) = self.index(id) {
                self.chosen.insert((i, *step), *choice);
            }
        }
    }

    /// Nuova partita: tutto da capo.
    pub fn reset(&mut self) {
        self.progress.iter_mut().for_each(|p| *p = 0);
        self.done_on.clear();
        self.talking = None;
        self.talked.clear();
        self.picked.clear();
        self.chosen.clear();
    }

    /// Shell e prove: porta una missione a una tappa (le precedenti risultano fatte).
    pub fn jump(&mut self, id: &str, step: usize) -> Result<String, String> {
        let i = self
            .index(id)
            .ok_or_else(|| format!("missione sconosciuta: {id}"))?;
        // Le missioni che la precedono risultano finite.
        let mut before = self.list[i].after.clone();
        while let Some(j) = self.index(&before) {
            self.progress[j] = self.list[j].steps.len();
            before = self.list[j].after.clone();
        }
        self.progress[i] = step.min(self.list[i].steps.len());
        self.enabled = true;
        Ok(format!(
            "{}: tappa {} di {}",
            self.list[i].title,
            self.progress[i] + 1,
            self.list[i].steps.len()
        ))
    }
}

fn holds(w: &When, f: &Facts, talked: &[(String, usize)], mission: usize) -> bool {
    match w {
        When::Near(p, r) => f.pos.xz().distance(*p) <= *r,
        When::Below(y) => f.pos.y < *y,
        When::Above(y) => f.pos.y > *y,
        When::Hour(a, b) => {
            if a <= b {
                f.hour >= *a && f.hour < *b
            } else {
                f.hour >= *a || f.hour < *b
            }
        }
        When::Talk(npc) => talked.iter().any(|(t, i)| t == npc && *i == mission),
        When::Has(kind, n) => f.items.iter().any(|(k, q)| k == kind && q >= n),
        When::Clear(p, r) => !f.enemies.iter().any(|e| e.xz().distance(*p) <= *r),
        When::Weather(kind) => f.weather == Some(*kind),
        When::SidesDone(n) => f.sides_done >= *n,
        When::Count(key, n) => f.counters.iter().any(|(k, v)| k == key && v >= n),
        When::Unlocked(what) => f.unlocked.iter().any(|u| u == what),
        When::Quest(done) => {
            if *done {
                f.quest_done
            } else {
                f.quest_accepted
            }
        }
    }
}

/// Tappe aggiunte nelle versioni del gioco: per i salvataggi fatti **prima** di quella versione, la
/// tappa nuova che corrisponde a ogni tappa vecchia (l'ultimo indice è «compiuta»).
/// - v0.0.63: mp01 ha il risveglio prima di Gaspare; mp03 ha Agnese prima della torre; mp04 ha le
///   stele prima del cunicolo. Chi era fermo lì fa la tappa nuova.
/// - v0.0.65: qs01 ha i predoni del mulino e il sacco di farina dopo Gino; qs06 ha il pane di
///   Marta prima del pesce; qs09 ha il duello e la difesa con Aldo dopo l'acqua; qs12 ha il campanile prima del
///   pozzo e la Camera di Risonanza dopo; qs13 ha la chiave di Don Anselmo e il quadro di rame
///   (chi era alla grata riparte dalla chiave); qs17 ha la casa di pietra prima del Viandante; qs14 ha la grotta e il bracciale dopo la prova. Chi le aveva compiute resta a posto.
const MOVES: &[([u32; 4], &str, &[usize])] = &[
    ([0, 0, 63, 0], "mp01", &[0, 2, 4, 5, 6]),
    ([0, 0, 63, 0], "mp03", &[0, 1, 2, 3, 4, 6]),
    ([0, 0, 63, 0], "mp04", &[0, 1, 2, 3, 5]),
    ([0, 0, 65, 0], "qs01", &[0, 1, 4, 5, 6]),
    ([0, 0, 65, 0], "qs06", &[0, 1, 3, 4, 5]),
    ([0, 0, 65, 0], "qs09", &[0, 1, 2, 6]),
    ([0, 0, 65, 0], "qs12", &[0, 1, 2, 3, 4, 6, 9]),
    ([0, 0, 65, 0], "qs13", &[0, 1, 4, 5, 6]),
    ([0, 0, 65, 0], "qs14", &[0, 1, 2, 3, 7]),
    ([0, 0, 65, 0], "qs17", &[0, 1, 2, 3, 4, 8]),
];

/// La disposizione delle tappe di questa versione: un salvataggio riletto vale come questa.
pub const STEP_LAYOUT: &str = "v0.0.65.0";

fn version_parts(game_version: &str) -> Vec<u32> {
    game_version
        .trim()
        .trim_start_matches('v')
        .split('.')
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}

/// Il salvataggio ha le tappe di una versione più vecchia di `STEP_LAYOUT`?
pub fn older_layout(game_version: &str) -> bool {
    version_parts(game_version) < version_parts(STEP_LAYOUT)
}

/// Riallinea la tappa salvata da una versione precedente del gioco (`vA.B.C.D`; vuota = molto
/// vecchia), passando per ogni versione che ha aggiunto tappe. Le missioni esistono dalla v0.0.62.0.
pub fn upgrade_step(game_version: &str, id: &str, step: usize) -> usize {
    let saved = version_parts(game_version);
    MOVES
        .iter()
        .filter(|(since, m, _)| *m == id && saved.as_slice() < since.as_slice())
        .fold(step, |s, (_, _, map)| map.get(s).copied().unwrap_or(s))
}

/// I doni della Sera delle Lanterne: chi ti aveva affidato una missione secondaria compiuta ti
/// porta qualcosa (oggetto, quantità, chi).
pub fn gifts(missions: &Missions) -> Vec<(crate::inventory::ItemKind, u32, String)> {
    use crate::inventory::{ItemKind as K, Relic};
    missions
        .list
        .iter()
        .zip(&missions.progress)
        .filter(|(m, p)| m.side && !m.event && **p >= m.steps.len())
        .filter_map(|(m, _)| {
            let who = m.steps.first()?.when.iter().find_map(|w| match w {
                When::Talk(n) => Some(n.clone()),
                _ => None,
            })?;
            let (kind, n) = match m.id.as_str() {
                "qs01" | "qs06" => (K::Relic(Relic::Focaccia), 2),
                "qs03" | "qs11" | "qs14" => (K::Arrow, 10),
                "qs08" => (K::MulePotion, 1),
                "qs16" => (K::Lockpick, 3),
                "qs02" | "qs04" | "qs09" | "qs13" => (K::HealthPotion, 1),
                "qs05" => (K::Apple, 3),
                _ => (K::Coin, 10),
            };
            Some((kind, n, who))
        })
        .collect()
}

/// Applica un effetto di tappa o di scelta.
fn apply_effect(
    effect: &Effect,
    inventory: &mut crate::inventory::Inventory,
    character: &mut crate::character::Character,
) {
    match effect {
        Effect::Give(kind, n) => inventory.add(*kind, *n),
        Effect::Take(kind, n) => {
            inventory.consume(*kind, *n);
        }
        Effect::Coins(n) => inventory.add(crate::inventory::ItemKind::Coin, *n),
        Effect::Karma(n, why) => {
            // «karma 3 #pieta motivo»: il valore toccato fa cambiare idea a chi ci tiene.
            let (value, why) = match why.strip_prefix('#') {
                Some(rest) => rest.split_once(' ').unwrap_or((rest, "")),
                None => ("", why.as_str()),
            };
            let why = if why.is_empty() {
                "una scelta".to_string()
            } else {
                why.to_string()
            };
            if value.is_empty() {
                crate::karma::deed(*n, why);
            } else {
                crate::karma::deed_as(value, *n, why);
            }
        }
        Effect::Trust(name, n) => crate::karma::trust(name, *n),
        Effect::Stat(stat, n) => {
            let value = i16::from(character.stats.get(*stat)) + i16::from(*n);
            character.stats.set(*stat, value.clamp(0, 255) as u8);
            info!(
                "[MISSIONE] {} → {}",
                stat.name(),
                character.stats.get(*stat)
            );
        }
        Effect::Unlock(_) | Effect::End | Effect::Time(..) | Effect::Gifts | Effect::Travel(..) => {
        }
    }
}

/// Sistema: valuta le condizioni, completa le tappe, dà i premi e avvisa il giocatore.
#[allow(clippy::too_many_arguments)]
pub fn track_missions(
    mut missions: ResMut<Missions>,
    quest: Res<QuestState>,
    mut clock: ResMut<crate::daynight::GameClock>,
    mut weather: ResMut<crate::weather::WeatherState>,
    dialog: Res<crate::entities::DialogState>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    enemies: Query<(&GlobalTransform, &crate::combat::EnemyHealth), With<crate::entities::Enemy>>,
    mut inventory: ResMut<crate::inventory::Inventory>,
    mut session: ResMut<crate::save::Session>,
    thievery: Res<crate::thievery::Thievery>,
    house: Res<crate::house::House>,
    trial: Res<crate::archery::Trial>,
    karma: Res<crate::karma::Karma>,
    mut character: ResMut<crate::character::Character>,
    mut cutscene: ResMut<crate::cutscene::Cutscene>,
) {
    if cutscene.was_open && !dialog.open {
        missions.dialog_closed(dialog.finished, dialog.result);
    }
    cutscene.was_open = dialog.open;
    let Ok(player) = players.single() else {
        return;
    };
    let mut facts = Facts {
        day: clock.date.day_key(),
        age: None,
        pos: player.translation(),
        hour: clock.hour,
        quest_accepted: quest.accepted,
        quest_done: quest.stage == QuestStage::Completed,
        weather: Some(weather.shown()),
        sides_done: 0,
        counters: thievery
            .counters()
            .into_iter()
            .chain([
                ("casa".to_string(), u32::from(house.stage)),
                ("arco".to_string(), u32::from(trial.passed)),
                ("risveglio".to_string(), u32::from(character.awakening)),
            ])
            // Segreti confidati: `quando: conta segreto:beppe 1` (ADR 0044).
            .chain(karma.revealed.iter().map(|n| (format!("segreto:{n}"), 1)))
            .collect(),
        // Ciò che i segreti hanno sbloccato vale anche per le missioni (ADR 0045).
        unlocked: karma.unlocks.iter().cloned().collect(),
        stats: Some(character.stats),
        items: inventory.slots.iter().map(|s| (s.kind, s.qty)).collect(),
        // Solo i vivi: un nemico sconfitto resta un attimo nel mondo prima di sparire.
        enemies: enemies
            .iter()
            .filter(|(_, hp)| hp.current > 0.0)
            .map(|(t, _)| t.translation())
            .collect(),
    };
    for (i, step, finished) in missions.advance(&mut facts) {
        let title = missions.list[i].title.clone();
        if step.coins > 0 {
            inventory.add(crate::inventory::ItemKind::Coin, step.coins);
        }
        for (kind, n) in &step.takes {
            inventory.consume(*kind, *n);
        }
        for (kind, n) in &step.gives {
            inventory.add(*kind, *n);
        }
        for effect in &step.effects {
            match effect {
                Effect::Time(hour, days, kind) => {
                    let date = jump(clock.date, *days);
                    clock.set(date, *hour);
                    weather.scene = kind.map(|k| (date.day_key(), k));
                    info!("[MISSIONE] il tempo passa: {} alle {hour:.1}", date.label());
                }
                Effect::Gifts => {
                    let gifts = gifts(&missions);
                    for (kind, n, _) in &gifts {
                        inventory.add(*kind, *n);
                    }
                    let givers: Vec<String> = gifts.iter().map(|(.., who)| who.clone()).collect();
                    cutscene.queue_notice(if givers.is_empty() {
                        "Nessuno ti porta un dono: non hai ancora aiutato nessuno.".to_string()
                    } else {
                        format!("Doni da chi hai aiutato: {}", givers.join(", "))
                    });
                }
                Effect::Travel(region, how) => {
                    let farewell = missions.done("qs15");
                    cutscene.start_travel(*region, how, &karma, farewell);
                }
                other => apply_effect(other, &mut inventory, &mut character),
            }
        }
        let next = missions.list[i]
            .steps
            .get(missions.progress[i])
            .map(|s| s.goal.clone());
        let text = match (&next, finished) {
            (_, true) => format!("Missione compiuta: {title}"),
            (Some(goal), false) if step.note.is_empty() => format!("{title} — {goal}"),
            _ => step.note.clone(),
        };
        info!(
            "[MISSIONE] {} — tappa fatta: {}{}",
            missions.list[i].id,
            step.goal,
            if finished { " (missione compiuta)" } else { "" }
        );
        session.notify(text);
        // Chi ti ha affidato o chiuso una tappa si fida un po' di più (ADR 0040).
        for w in &step.when {
            if let When::Talk(npc) = w {
                crate::karma::trust(npc, crate::karma::MISSION_TRUST);
            }
        }
        // Una missione chiusa da un bivio (`fine`, per esempio vendendo le piastrine) non vale come
        // compiuta per la bilancia: pesa solo la scelta.
        if finished && !step.effects.contains(&Effect::End) {
            let m = &missions.list[i];
            crate::karma::deed_as(
                "aiuto",
                if m.event {
                    1
                } else if m.side {
                    3
                } else {
                    4
                },
                format!("{} compiuta", m.title),
            );
            crate::skills::reward(if m.event {
                crate::skills::EVENT_XP
            } else if m.side {
                crate::skills::SIDE_XP
            } else {
                crate::skills::MAIN_XP
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_crossing_waits_for_summer() {
        use crate::daynight::{GameDate, Season};
        let may = GameDate {
            year: 0,
            month: 5,
            day: 12,
        };
        let june = jump(may, UNTIL_SUMMER);
        assert_eq!((june.month, june.day), (6, 21));
        assert_eq!(june.season(), Season::Summer);
        // Già d'estate, o più tardi, non si salta.
        assert_eq!(jump(june, UNTIL_SUMMER), june);
        let october = GameDate { month: 10, ..may };
        assert_eq!(jump(october, UNTIL_SUMMER), october);
        assert_eq!(jump(may, 2).day, 14);
        assert!(matches!(
            Effect::parse("tempo 19.5 estate sereno"),
            Some(Effect::Time(_, UNTIL_SUMMER, Some(_)))
        ));
    }

    #[test]
    fn a_repeating_event_comes_back_after_its_days() {
        let mut m = all();
        let i = m.index("ec05").unwrap();
        assert_eq!(m.list[i].repeat, Some(7));
        let len = m.list[i].steps.len();
        m.progress[i] = len;
        m.chosen.insert((i, 0), 2);
        m.repeat_due(100);
        assert_eq!(m.done_on.get("ec05"), Some(&100));
        m.repeat_due(106);
        assert_eq!(m.progress[i], len, "sei giorni non bastano");
        m.repeat_due(107);
        assert_eq!(m.progress[i], 0);
        assert!(
            !m.chosen.contains_key(&(i, 0)),
            "la risposta di allora è dimenticata"
        );
        assert!(m.done_on.is_empty());
    }

    #[test]
    fn every_item_in_the_mission_files_exists() {
        // `dai:`, `prendi:` e gli effetti «dai …» delle scelte: un nome sbagliato sparirebbe in silenzio.
        for file in [
            include_str!("../../assets/quests/prologo/missioni.txt"),
            include_str!("../../assets/quests/prologo/secondarie.txt"),
            include_str!("../../assets/quests/prologo/eventi.txt"),
        ] {
            for line in file.lines().map(str::trim) {
                let mut wanted: Vec<&str> = Vec::new();
                if let Some(v) = line
                    .strip_prefix("dai:")
                    .or_else(|| line.strip_prefix("prendi:"))
                {
                    wanted.push(v);
                }
                if let Some((_, effects)) = line
                    .strip_prefix("scegli:")
                    .and_then(|l| l.split_once("=>"))
                {
                    wanted.extend(effects.split(';').filter_map(|e| {
                        e.trim()
                            .strip_prefix("dai ")
                            .or_else(|| e.trim().strip_prefix("prendi "))
                    }));
                }
                for w in wanted {
                    assert!(
                        item_qty(w).is_some(),
                        "oggetto sconosciuto: «{w}» in «{line}»"
                    );
                }
            }
        }
    }

    fn all() -> Missions {
        Missions {
            enabled: true,
            ..Default::default()
        }
    }

    fn facts(pos: Vec3, hour: f32, accepted: bool, done: bool) -> Facts {
        Facts {
            day: 0,
            age: None,
            pos,
            hour,
            quest_accepted: accepted,
            quest_done: done,
            items: Vec::new(),
            enemies: Vec::new(),
            weather: None,
            sides_done: 0,
            counters: Vec::new(),
            unlocked: Vec::new(),
            stats: None,
        }
    }

    #[test]
    fn events_have_people_on_the_spot_and_the_patrols_tell_their_side() {
        let m = all();
        let get = |id: &str| &m.list[m.index(id).unwrap()];
        let ec02 = get("ec02");
        assert_eq!(ec02.steps.len(), 3);
        assert!(
            ec02.steps[0]
                .when
                .contains(&When::Talk(crate::island_life::CARTER.into()))
        );
        assert!(
            ec02.steps[1]
                .when
                .contains(&When::Unlocked("predoni-vinti".into()))
        );
        let ec04 = get("ec04");
        assert!(
            ec04.steps[0]
                .when
                .contains(&When::Unlocked("pecora-liberata".into()))
        );
        assert!(
            ec04.steps[1]
                .when
                .contains(&When::Unlocked("pecora-al-gregge".into()))
        );
        assert_eq!(get("ec08").steps[0].choices.len(), 3);
        let qs12 = get("qs12");
        for guard in [
            "Guardia Gino",
            "Guardia Livio",
            "Guardia Ivo",
            "Guardia Mario",
        ] {
            assert!(
                qs12.steps
                    .iter()
                    .any(|s| s.when.contains(&When::Talk(guard.into()))
                        && s.effects.contains(&Effect::Trust(guard.into(), 3))),
                "{guard}"
            );
        }
        // La scintilla sulla torre è un segreto del luogo finché non la si vede.
        assert!(m.events_near(Vec2::new(58.5, -58.5), 18.0) >= 1);
        assert_eq!(m.events_near(Vec2::new(-200.0, -200.0), 18.0), 0);
    }

    #[test]
    fn saves_from_before_0_0_63_find_the_same_step_after_the_new_ones() {
        let m = all();
        // Le tappe della v0.0.62.0, con il loro obiettivo di allora.
        let old: &[(&str, &[&str])] = &[
            (
                "mp01",
                &[
                    "",
                    "Raggiungi il pescatore al molo",
                    "Sali a Fontevivo per la strada di pietra",
                    "Trova il Viandante alla casetta in piazza",
                ],
            ),
            (
                "mp03",
                &[
                    "Chiedi al Viandante che cosa c'è a nord",
                    "Parla con Gino al mulino, a nord",
                    "Esamina le rovine antiche, a sud-ovest",
                    "Esamina il santuario, a ovest",
                    "",
                ],
            ),
            (
                "mp04",
                &[
                    "Chiedi a Don Anselmo della grotta",
                    "Raggiungi la grotta sulla costa est",
                    "Scendi nelle gallerie fino al laghetto",
                    "",
                ],
            ),
        ];
        for (id, goals) in old {
            let i = m.index(id).unwrap();
            let steps = &m.list[i].steps;
            for (p, goal) in goals.iter().enumerate() {
                let new = upgrade_step("v0.0.62.0", id, p);
                if !goal.is_empty() {
                    assert_eq!(steps[new].goal, *goal, "{id} tappa {p}");
                }
            }
            // Compiuta resta compiuta.
            assert_eq!(upgrade_step("v0.0.62.0", id, goals.len()), steps.len());
        }
        // Chi era fermo dove ora c'è una tappa nuova la fa.
        assert!(
            m.list[m.index("mp03").unwrap()].steps[upgrade_step("v0.0.62.0", "mp03", 4)]
                .goal
                .contains("Agnese")
        );
        assert!(
            m.list[m.index("mp04").unwrap()].steps[upgrade_step("v0.0.62.0", "mp04", 3)]
                .goal
                .contains("stele")
        );
        // Le altre missioni e i salvataggi nuovi non si toccano.
        assert_eq!(upgrade_step("v0.0.62.0", "mp05", 2), 2);
        assert_eq!(upgrade_step("v0.0.63.0", "mp01", 1), 1);
        assert_eq!(upgrade_step("v0.0.64.0", "mp03", 4), 4);
        assert_eq!(upgrade_step("v0.1.0.0", "mp01", 1), 1);
    }

    #[test]
    fn the_prologue_file_has_the_four_new_main_missions_in_a_chain() {
        let m = all();
        let ids: Vec<&str> = m.list.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids[..4], ["mp01", "mp03", "mp04", "mp05"]);
        assert!(m.list[..4].iter().all(|x| !x.side));
        assert!(
            m.list[4..].iter().all(|x| x.side),
            "le altre sono secondarie"
        );
        assert_eq!(m.list.iter().filter(|x| x.side && !x.event).count(), 18);
        assert_eq!(m.list.iter().filter(|x| x.event).count(), 12);
        for mission in &m.list {
            assert!(!mission.title.is_empty() && !mission.summary.is_empty());
            // Un evento è un incontro: una tappa o due. Le missioni vere ne hanno almeno tre.
            let least = if mission.event { 1 } else { 3 };
            assert!(mission.steps.len() >= least, "{}", mission.id);
            for s in &mission.steps {
                assert!(!s.goal.is_empty() && !s.when.is_empty(), "{}", mission.id);
                // Chi deve parlare ha qualcosa da dire.
                if s.when.iter().any(|w| matches!(w, When::Talk(_))) {
                    assert!(!s.lines.is_empty(), "{}: {}", mission.id, s.goal);
                }
            }
        }
        assert!(m.list[1].after_quest, "mp03 viene dopo Denaro Rubato");
        assert_eq!(m.list[2].after, "mp03");
        assert_eq!(m.list[3].after, "mp04");
        // Solo l'ultima tappa del Prologo apre la via del continente.
        assert!(!m.unlocked("continente"));
    }

    #[test]
    fn steps_advance_only_when_every_condition_holds() {
        let text = "[missione a]\ntitolo: A\nsommario: s\n[tappa]\nobiettivo: vai\n\
                    quando: vicino 10 10 3\nquando: ora 21 4\nmonete: 5\n\
                    [tappa]\nobiettivo: parla\nquando: parla Gino\n> Ciao {nome}.\nsblocca: x\n";
        let mut m = Missions {
            list: parse(text),
            progress: vec![0],
            enabled: true,
            talking: None,
            talked: Vec::new(),
            ..Default::default()
        };
        // Posto giusto, ora sbagliata.
        assert!(
            m.advance(&mut facts(Vec3::new(10.0, 0.0, 11.0), 12.0, false, false))
                .is_empty()
        );
        // Ora giusta (a cavallo della mezzanotte), posto sbagliato.
        assert!(
            m.advance(&mut facts(Vec3::new(50.0, 0.0, 11.0), 23.0, false, false))
                .is_empty()
        );
        let done = m.advance(&mut facts(Vec3::new(10.0, 0.0, 11.0), 2.0, false, false));
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].1.coins, 5);
        assert!(!done[0].2);
        // Il dialogo vale solo se letto fino in fondo.
        let mut f = facts(Vec3::ZERO, 12.0, false, false);
        assert_eq!(m.dialog_for("Marta", &mut f, "Ada"), None);
        assert_eq!(
            m.dialog_for("Gino", &mut f, "Ada"),
            Some((vec!["Ciao Ada.".to_string()], Vec::new()))
        );
        m.dialog_closed(false, None);
        assert!(m.advance(&mut f).is_empty());
        m.dialog_for("Gino", &mut f, "Ada");
        m.dialog_closed(true, None);
        let done = m.advance(&mut f);
        assert!(done[0].2, "missione finita");
        assert!(m.done("a") && m.unlocked("x"));
    }

    #[test]
    fn one_talk_advances_one_mission_even_with_the_same_villager() {
        let text = "[missione a]\ntitolo: A\nsommario: s\n[tappa]\nobiettivo: g\nquando: parla Gino\n> Uno.\n\
                    [missione b]\ntitolo: B\nsommario: s\ntipo: secondaria\n[tappa]\nobiettivo: g\n\
                    quando: parla Gino\n> Due.\n";
        let mut m = Missions {
            list: parse(text),
            progress: vec![0, 0],
            enabled: true,
            talking: None,
            talked: Vec::new(),
            ..Default::default()
        };
        let mut f = facts(Vec3::ZERO, 12.0, false, false);
        assert_eq!(
            m.dialog_for("Gino", &mut f, "x"),
            Some((vec!["Uno.".to_string()], Vec::new()))
        );
        m.dialog_closed(true, None);
        assert_eq!(m.advance(&mut f).len(), 1);
        assert_eq!(m.progress, vec![1, 0]);
        // La seconda volta Gino propone l'altro incarico.
        assert_eq!(
            m.dialog_for("Gino", &mut f, "x"),
            Some((vec!["Due.".to_string()], Vec::new()))
        );
        m.dialog_closed(true, None);
        m.advance(&mut f);
        assert_eq!(m.progress, vec![1, 1]);
    }

    #[test]
    fn a_choice_in_the_dialog_adds_its_effects_and_can_end_the_mission() {
        use crate::inventory::{ItemKind, Relic};
        let text = "[missione q]\ntitolo: Q\nsommario: s\ntipo: secondaria\n[tappa]\nobiettivo: scegli\n\
                    quando: parla Ida\n> Che fai?\nkarma: 1 ascolto\n\
                    scegli: Tienilo => dai amuleto piuma 1; fiducia Ida 5\n\
                    scegli: Vendilo => monete 30; karma -2 avidità; fine\n\
                    [tappa]\nobiettivo: dopo\nquando: vicino 0 0 1\nsblocca: dopo\n";
        let list = parse(text);
        let step = &list[0].steps[0];
        assert_eq!(step.effects, vec![Effect::Karma(1, "ascolto".into())]);
        assert_eq!(step.choices.len(), 2);
        assert_eq!(
            step.choices[0].effects,
            vec![
                Effect::Give(ItemKind::Relic(Relic::FeatherAmulet), 1),
                Effect::Trust("Ida".into(), 5)
            ]
        );
        assert!(step.choices[1].effects.contains(&Effect::End));
        let mut m = Missions {
            list,
            progress: vec![0],
            enabled: true,
            ..Default::default()
        };
        let mut f = facts(Vec3::new(50.0, 0.0, 50.0), 12.0, false, false);
        let (lines, choices) = m.dialog_for("Ida", &mut f, "Ada").unwrap();
        assert_eq!(
            (lines.len(), choices),
            (1, vec!["Tienilo".into(), "Vendilo".into()])
        );
        m.dialog_closed(true, Some(1));
        let done = m.advance(&mut f);
        assert_eq!(done.len(), 1);
        assert!(done[0].1.effects.contains(&Effect::Coins(30)));
        assert!(done[0].2, "la seconda risposta chiude la missione");
        assert_eq!(m.choices_snapshot(), vec![("q".to_string(), 0, 1)]);
        assert!(!m.unlocked("dopo"));
        // Chi sceglie la prima risposta prosegue alla tappa dopo.
        m.reset();
        m.dialog_for("Ida", &mut f, "Ada");
        m.dialog_closed(true, Some(0));
        let done = m.advance(&mut f);
        assert!(!done[0].2);
        assert!(done[0].1.effects.contains(&Effect::Trust("Ida".into(), 5)));
        let saved = m.choices_snapshot();
        m.reset();
        m.restore_choices(&saved);
        assert_eq!(m.chosen.get(&(0, 0)), Some(&0));
    }

    #[test]
    fn an_answer_that_needs_a_stat_hides_when_the_stat_is_low() {
        use crate::character::{Stat, Stats};
        let text = "[missione t]\ntitolo: T\nsommario: s\ntipo: evento\n[tappa]\nobiettivo: g\n\
                    quando: parla Bandito\n> Alt.\nscegli: Pago => monete 0; sblocca pagato\n\
                    scegli: [carisma 12] Sparite => sblocca sventato\nscegli: Combatto => sblocca lotta\n";
        let mut m = Missions {
            list: parse(text),
            progress: vec![0],
            enabled: true,
            ..Default::default()
        };
        assert_eq!(
            m.list[0].steps[0].choices[1].requires,
            Some((Stat::Charisma, 12))
        );
        let mut f = facts(Vec3::ZERO, 12.0, false, false);
        f.stats = Some(Stats::default());
        let (_, choices) = m.dialog_for("Bandito", &mut f, "Ada").unwrap();
        assert_eq!(choices, vec!["Pago".to_string(), "Combatto".to_string()]);
        // «Combatto» è la seconda mostrata ma la terza della tappa.
        m.dialog_closed(true, Some(1));
        m.advance(&mut f);
        assert!(m.unlocked("lotta") && !m.unlocked("sventato"));
        // Con Carisma alto la risposta c'è.
        m.reset();
        let mut stats = Stats::default();
        stats.set(Stat::Charisma, 13);
        f.stats = Some(stats);
        let (_, choices) = m.dialog_for("Bandito", &mut f, "Ada").unwrap();
        assert_eq!(choices.len(), 3);
    }

    #[test]
    fn the_side_quests_offer_their_choices_and_keepsakes() {
        let m = all();
        let find = |id: &str| m.list.iter().find(|x| x.id == id).unwrap();
        let choices = |id: &str| -> usize { find(id).steps.iter().map(|s| s.choices.len()).sum() };
        assert_eq!(choices("qs03"), 2);
        assert_eq!(choices("qs04"), 2);
        assert_eq!(choices("qs07"), 3);
        assert_eq!(choices("qs08"), 2);
        assert_eq!(choices("qs10"), 2);
        // Ogni effetto scritto nei file è stato riconosciuto (nessuna scelta senza effetti).
        for mission in &m.list {
            for step in &mission.steps {
                for c in &step.choices {
                    assert!(
                        !c.effects.is_empty(),
                        "{}: «{}» senza effetti",
                        mission.id,
                        c.text
                    );
                }
            }
        }
        let qs04 = find("qs04");
        assert!(qs04.steps[2].choices[1].effects.contains(&Effect::End));
        assert!(qs04.steps[3].unlocks.contains(&"rito-banditi".to_string()));
    }

    #[test]
    fn every_spoken_mission_line_has_its_voice() {
        // Le voci si generano con `scripts/gen_voci_prologo.py` (ADR 0050).
        let m = all();
        for mission in &m.list {
            for step in &mission.steps {
                let Some(npc) = step.when.iter().find_map(|w| match w {
                    When::Talk(n) => Some(n.clone()),
                    _ => None,
                }) else {
                    continue;
                };
                for line in lines_for_age(&step.lines, None) {
                    assert!(
                        !crate::voice::line_clip(&npc, &line).is_empty(),
                        "{}: manca la voce di {npc} per «{line}»",
                        mission.id
                    );
                }
            }
        }
    }

    #[test]
    fn world_counters_close_a_step() {
        let text = "[missione q]\ntitolo: Q\nsommario: s\ntipo: secondaria\n[tappa]\nobiettivo: apri\n\
                    quando: conta serrature 3\n[tappa]\nobiettivo: infila\n\
                    quando: conta infilate:guardia bruno 15\n";
        let list = parse(text);
        assert_eq!(
            list[0].steps[0].when,
            vec![When::Count("serrature".into(), 3)]
        );
        assert_eq!(
            list[0].steps[1].when,
            vec![When::Count("infilate:guardia bruno".into(), 15)]
        );
        let mut m = Missions {
            list,
            progress: vec![0],
            enabled: true,
            talking: None,
            talked: Vec::new(),
            ..Default::default()
        };
        let mut f = facts(Vec3::ZERO, 12.0, false, false);
        f.counters = vec![("serrature".into(), 2)];
        assert!(m.advance(&mut f).is_empty());
        f.counters = vec![
            ("serrature".into(), 3),
            ("infilate:guardia bruno".into(), 10),
        ];
        assert_eq!(m.advance(&mut f).len(), 1);
        assert!(m.advance(&mut f).is_empty(), "dieci monete non bastano");
        f.counters[1].1 = 15;
        assert_eq!(m.advance(&mut f).len(), 1);
    }

    #[test]
    fn items_are_handed_over_and_camps_must_be_cleared() {
        use crate::inventory::ItemKind;
        let text = "[missione q]\ntitolo: Q\nsommario: s\ntipo: secondaria\n[tappa]\nobiettivo: porta\n\
                    quando: ha pesce 2\nquando: ha mela\nprendi: pesce 2\ndai: pozione del mulo\n\
                    [tappa]\nobiettivo: libera\nquando: libero 10 10 8\n";
        let list = parse(text);
        assert!(list[0].side);
        assert_eq!(list[0].steps[0].takes, vec![(ItemKind::Fish, 2)]);
        assert_eq!(list[0].steps[0].gives, vec![(ItemKind::MulePotion, 1)]);
        let mut m = Missions {
            list,
            progress: vec![0],
            enabled: true,
            talking: None,
            talked: Vec::new(),
            ..Default::default()
        };
        let mut f = facts(Vec3::ZERO, 12.0, false, false);
        f.items = vec![(ItemKind::Fish, 1), (ItemKind::Apple, 3)];
        assert!(m.advance(&mut f).is_empty(), "un pesce solo non basta");
        f.items[0].1 = 2;
        f.enemies = vec![Vec3::new(12.0, 0.0, 11.0)];
        assert_eq!(m.advance(&mut f).len(), 1);
        assert!(
            m.advance(&mut f).is_empty(),
            "c'è ancora un nemico nel campo"
        );
        f.enemies.clear();
        assert!(m.advance(&mut f)[0].2);
    }

    #[test]
    fn progress_survives_a_save_and_the_chain_waits_for_the_stolen_money() {
        let mut m = all();
        // mp03 non comincia finché Denaro Rubato non è completata; le secondarie non ancora
        // proposte non stanno nel diario.
        assert_eq!(m.journal(false).len(), 1);
        m.jump("mp04", 1).unwrap();
        assert!(m.done("mp03"));
        let saved = m.snapshot();
        let mut again = all();
        again.restore(&saved);
        assert_eq!(again.progress, m.progress);
        assert_eq!(again.journal(true).len(), 3);
        // Una secondaria entra nel diario quando chi la propone ha parlato.
        let side = again.list.iter().position(|x| x.side).unwrap();
        again.progress[side] = 1;
        assert_eq!(again.journal(true).len(), 4);
        again.reset();
        assert!(!again.done("mp03"));
    }
}
