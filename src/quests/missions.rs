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
}

/// Oggetto dal nome italiano usato nei file («mela», «pesce», «pozione del mulo»…).
fn item(name: &str) -> Option<crate::inventory::ItemKind> {
    let wanted = name.trim().to_lowercase();
    crate::inventory::ItemKind::ALL
        .into_iter()
        .find(|k| k.name().to_lowercase() == wanted)
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
    /// Dialoghi di missione letti fino in fondo (nome dell'abitante), in attesa di essere contati.
    talked: Vec<(String, usize)>,
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
            talked: Vec::new(),
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
}

impl Missions {
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
    pub fn unlocked(&self, what: &str) -> bool {
        self.list.iter().zip(&self.progress).any(|(m, p)| {
            m.steps[..(*p).min(m.steps.len())]
                .iter()
                .any(|s| s.unlocks.iter().any(|u| u == what))
        })
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
    ) -> Option<Vec<String>> {
        if !self.enabled {
            return None;
        }
        facts.sides_done = self.sides_done();
        let facts = &*facts;
        let (mission, lines) = self.current(facts.quest_done).find_map(|(i, step)| {
            let mine = step
                .when
                .iter()
                .any(|w| matches!(w, When::Talk(n) if n == npc));
            let rest = step
                .when
                .iter()
                .filter(|w| !matches!(w, When::Talk(_)))
                .all(|w| holds(w, facts, &[], i));
            (mine && rest && !step.lines.is_empty()).then(|| (i, step.lines.clone()))
        })?;
        // Il dialogo vale per questa missione sola: un altro incarico dello stesso abitante
        // aspetta la prossima volta che gli si parla.
        self.talking = Some((npc.to_string(), mission));
        Some(lines.iter().map(|l| l.replace("{nome}", player)).collect())
    }

    /// Il dialogo si è chiuso: se era di missione ed è stato letto tutto, vale come «parlato».
    pub fn dialog_closed(&mut self, finished: bool) {
        if let Some(npc) = self.talking.take()
            && finished
        {
            self.talked.push(npc);
        }
    }

    /// Avanza le tappe le cui condizioni valgono; restituisce quelle appena completate.
    pub fn advance(&mut self, facts: &mut Facts) -> Vec<(usize, Step, bool)> {
        let mut done = Vec::new();
        if !self.enabled {
            return done;
        }
        facts.sides_done = self.sides_done();
        let facts = &*facts;
        for i in 0..self.list.len() {
            let Some(step) = self.list[i].steps.get(self.progress[i]) else {
                continue;
            };
            if !self.started(i, facts.quest_done) || step.when.is_empty() {
                continue;
            }
            if step.when.iter().all(|w| holds(w, facts, &self.talked, i)) {
                let step = step.clone();
                self.progress[i] += 1;
                let finished = self.progress[i] >= self.list[i].steps.len();
                done.push((i, step, finished));
            }
        }
        self.talked.clear();
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

    /// Nuova partita: tutto da capo.
    pub fn reset(&mut self) {
        self.progress.iter_mut().for_each(|p| *p = 0);
        self.talking = None;
        self.talked.clear();
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
        When::Quest(done) => {
            if *done {
                f.quest_done
            } else {
                f.quest_accepted
            }
        }
    }
}

/// Sistema: valuta le condizioni, completa le tappe, dà i premi e avvisa il giocatore.
#[allow(clippy::too_many_arguments)]
pub fn track_missions(
    mut missions: ResMut<Missions>,
    quest: Res<QuestState>,
    clock: Res<crate::daynight::GameClock>,
    weather: Res<crate::weather::WeatherState>,
    dialog: Res<crate::entities::DialogState>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    enemies: Query<(&GlobalTransform, &crate::combat::EnemyHealth), With<crate::entities::Enemy>>,
    mut inventory: ResMut<crate::inventory::Inventory>,
    mut session: ResMut<crate::save::Session>,
    mut was_open: Local<bool>,
) {
    if *was_open && !dialog.open {
        missions.dialog_closed(dialog.finished);
    }
    *was_open = dialog.open;
    let Ok(player) = players.single() else {
        return;
    };
    let mut facts = Facts {
        pos: player.translation(),
        hour: clock.hour,
        quest_accepted: quest.accepted,
        quest_done: quest.stage == QuestStage::Completed,
        weather: Some(weather.shown()),
        sides_done: 0,
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
        if finished {
            let m = &missions.list[i];
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

    fn all() -> Missions {
        Missions {
            enabled: true,
            ..Default::default()
        }
    }

    fn facts(pos: Vec3, hour: f32, accepted: bool, done: bool) -> Facts {
        Facts {
            pos,
            hour,
            quest_accepted: accepted,
            quest_done: done,
            items: Vec::new(),
            enemies: Vec::new(),
            weather: None,
            sides_done: 0,
        }
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
        assert_eq!(m.list.iter().filter(|x| x.side && !x.event).count(), 14);
        assert_eq!(m.list.iter().filter(|x| x.event).count(), 11);
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
            Some(vec!["Ciao Ada.".to_string()])
        );
        m.dialog_closed(false);
        assert!(m.advance(&mut f).is_empty());
        m.dialog_for("Gino", &mut f, "Ada");
        m.dialog_closed(true);
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
        };
        let mut f = facts(Vec3::ZERO, 12.0, false, false);
        assert_eq!(
            m.dialog_for("Gino", &mut f, "x"),
            Some(vec!["Uno.".to_string()])
        );
        m.dialog_closed(true);
        assert_eq!(m.advance(&mut f).len(), 1);
        assert_eq!(m.progress, vec![1, 0]);
        // La seconda volta Gino propone l'altro incarico.
        assert_eq!(
            m.dialog_for("Gino", &mut f, "x"),
            Some(vec!["Due.".to_string()])
        );
        m.dialog_closed(true);
        m.advance(&mut f);
        assert_eq!(m.progress, vec![1, 1]);
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
