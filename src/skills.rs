//! Abilità che crescono con l'uso e livello del personaggio (ADR 0038).
//!
//! - Sedici abilità con un grado da 1 a 100: ogni azione nel mondo dà esperienza a quella giusta
//!   ([`train`], chiamabile da qualunque sistema).
//! - Ogni grado guadagnato fa avanzare il livello del personaggio, come le missioni compiute
//!   ([`reward`]). Un livello porta un punto statistica, un Nodo d'Oro e un po' di vita.
//! - Alle soglie 15/30/50/75/100 un Nodo d'Oro attiva un rango del talento dell'abilità.
//!   L'effetto del talento cresce con i ranghi attivati ([`rank`]).

use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};

use bevy::prelude::*;

pub const COUNT: usize = 16;
pub const MAX_GRADE: u8 = 100;

/// Soglie di grado dei cinque ranghi di ogni talento.
pub const RANKS: [(u8, &str); 5] = [
    (15, "Novizio"),
    (30, "Apprendista"),
    (50, "Esperto"),
    (75, "Maestro"),
    (100, "Leggenda"),
];

/// Esperienza di livello per ogni grado guadagnato e per le missioni compiute.
pub const GRADE_XP: f32 = 10.0;
pub const MAIN_XP: f32 = 60.0;
pub const SIDE_XP: f32 = 40.0;
pub const EVENT_XP: f32 = 20.0;
/// Vita in più per ogni livello oltre il primo.
pub const HEALTH_PER_LEVEL: f32 = 5.0;

/// Le sedici arti del viandante.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Skill {
    Athletics,
    Acrobatics,
    Swimming,
    Carrying,
    Blade,
    Defense,
    Armor,
    Archery,
    Magic,
    Stealth,
    Pickpocket,
    Lockpick,
    Building,
    Herbalism,
    Eloquence,
    Lore,
}

impl Skill {
    pub const ALL: [Skill; COUNT] = [
        Skill::Athletics,
        Skill::Acrobatics,
        Skill::Swimming,
        Skill::Carrying,
        Skill::Blade,
        Skill::Defense,
        Skill::Armor,
        Skill::Archery,
        Skill::Magic,
        Skill::Stealth,
        Skill::Pickpocket,
        Skill::Lockpick,
        Skill::Building,
        Skill::Herbalism,
        Skill::Eloquence,
        Skill::Lore,
    ];

    pub fn index(self) -> usize {
        Skill::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    pub fn name(self) -> &'static str {
        match self {
            Skill::Athletics => "Atletica e Corsa",
            Skill::Acrobatics => "Acrobazia e Salto",
            Skill::Swimming => "Nuoto e Marineria",
            Skill::Carrying => "Tempra del Portatore",
            Skill::Blade => "Lama e Mischia",
            Skill::Defense => "Difesa e Scudo",
            Skill::Armor => "Armatura",
            Skill::Archery => "Tiro con l'Arco",
            Skill::Magic => "Magie",
            Skill::Stealth => "Furtività",
            Skill::Pickpocket => "Borseggio",
            Skill::Lockpick => "Scassinare",
            Skill::Building => "Costruire",
            Skill::Herbalism => "Erboristeria",
            Skill::Eloquence => "Eloquenza e Commercio",
            Skill::Lore => "Sapienza ed Esplorazione",
        }
    }

    /// Nome breve senza accenti: salvataggi, shell e stato del pilota automatico.
    pub fn key(self) -> &'static str {
        match self {
            Skill::Athletics => "atletica",
            Skill::Acrobatics => "acrobazia",
            Skill::Swimming => "nuoto",
            Skill::Carrying => "tempra",
            Skill::Blade => "lama",
            Skill::Defense => "difesa",
            Skill::Armor => "armatura",
            Skill::Archery => "arco",
            Skill::Magic => "magie",
            Skill::Stealth => "furtivita",
            Skill::Pickpocket => "borseggio",
            Skill::Lockpick => "scassinare",
            Skill::Building => "costruire",
            Skill::Herbalism => "erboristeria",
            Skill::Eloquence => "eloquenza",
            Skill::Lore => "sapienza",
        }
    }

    pub fn parse(text: &str) -> Option<Skill> {
        let wanted = crate::inventory_ui::normalize(text);
        Skill::ALL.into_iter().find(|s| {
            s.key() == wanted || crate::inventory_ui::normalize(s.name()).starts_with(&wanted)
        })
    }

    /// Come si allena (vuoto: non ancora nel gioco).
    pub fn how(self) -> &'static str {
        match self {
            Skill::Athletics => "correndo",
            Skill::Acrobatics => "saltando",
            Skill::Swimming => "nuotando",
            Skill::Carrying => "camminando con lo zaino pieno almeno a metà",
            Skill::Blade => "colpendo in mischia",
            Skill::Defense => "incassando colpi con lo scudo",
            Skill::Archery => "colpendo con le frecce",
            Skill::Stealth => "restando vicino ai nemici senza farsi vedere",
            Skill::Herbalism => "bevendo pozioni",
            Skill::Eloquence => "parlando e comprando",
            Skill::Lore => "scoprendo luoghi e frugando nei mobili",
            Skill::Pickpocket => "sfilando o infilando monete alle spalle di qualcuno",
            Skill::Lockpick => "forzando le serrature dei bauli",
            Skill::Magic => "lanciando incantesimi",
            Skill::Armor => "incassando colpi con un'armatura indosso",
            Skill::Building => "",
        }
    }

    /// Talento dell'abilità: nome ed effetto per ogni rango attivato.
    pub fn perk(self) -> Option<(&'static str, &'static str)> {
        Some(match self {
            Skill::Athletics => ("Passo del Messaggero", "corsa +5%"),
            Skill::Acrobatics => ("Balzo del Capriolo", "salto +4%, cadute +0,5 m senza danno"),
            Skill::Swimming => ("Figlio di Poseidone", "nuoto +8%"),
            Skill::Carrying => ("Spalle d'Atlante", "portata +3 kg"),
            Skill::Blade => ("Fendente Fluido", "danno in mischia +6%"),
            Skill::Defense => ("Muro di Bronzo", "danno subito con lo scudo −10%"),
            Skill::Archery => ("Freccia di Odisseo", "danno delle frecce +8%"),
            Skill::Stealth => (
                "Figlio della Bruma",
                "i nemici ti notano da −8% di distanza",
            ),
            Skill::Herbalism => ("Infusione Pura", "cura delle pozioni +10%"),
            Skill::Eloquence => ("Mercante di Sidone", "prezzi −3%"),
            Skill::Lore => ("Passo del Viandante", "esperienza di livello +5%"),
            Skill::Pickpocket => ("Mano di Piuma", "riuscita del borseggio +5%"),
            Skill::Lockpick => ("Dita Sensibili", "tacca della serratura +10%"),
            Skill::Armor => ("Seconda Pelle di Ferro", "danno subito con l'armatura −5%"),
            Skill::Magic => ("Maestro dell'Accordo", "costo degli incantesimi −8%"),
            _ => return None,
        })
    }
}

/// Esperienza che serve per passare dal grado `grade` al successivo.
pub fn grade_need(grade: u8) -> f32 {
    10.0 + 2.0 * f32::from(grade)
}

/// Esperienza che serve per passare dal livello `level` al successivo.
pub fn level_need(level: u32) -> f32 {
    40.0 + 20.0 * level as f32
}

/// Progressi del personaggio: gradi, talenti, livello e punti da spendere.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Progress {
    pub grades: [u8; COUNT],
    /// Esperienza accumulata verso il grado successivo.
    pub xp: [f32; COUNT],
    /// Ranghi del talento attivati (0..=5).
    pub perks: [u8; COUNT],
    pub level: u32,
    pub level_xp: f32,
    pub stat_points: u32,
    /// Nodi d'Oro: attivano i ranghi dei talenti.
    pub nodes: u32,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            grades: [1; COUNT],
            xp: [0.0; COUNT],
            perks: [0; COUNT],
            level: 1,
            level_xp: 0.0,
            stat_points: 0,
            nodes: 0,
        }
    }
}

impl Progress {
    pub fn grade(&self, skill: Skill) -> u8 {
        self.grades[skill.index()]
    }

    pub fn perk_rank(&self, skill: Skill) -> u8 {
        self.perks[skill.index()]
    }

    /// Avanzamento (0..1) verso il grado successivo.
    pub fn grade_fraction(&self, skill: Skill) -> f32 {
        let i = skill.index();
        if self.grades[i] >= MAX_GRADE {
            1.0
        } else {
            (self.xp[i] / grade_need(self.grades[i])).clamp(0.0, 1.0)
        }
    }

    pub fn level_fraction(&self) -> f32 {
        (self.level_xp / level_need(self.level)).clamp(0.0, 1.0)
    }

    /// Vita massima in più data dal livello.
    pub fn health_bonus(&self) -> f32 {
        HEALTH_PER_LEVEL * self.level.saturating_sub(1) as f32
    }

    /// Dà esperienza a un'abilità. Rende gli avvisi (gradi e livelli guadagnati).
    pub fn gain(&mut self, skill: Skill, amount: f32) -> Vec<String> {
        let i = skill.index();
        let mut news = Vec::new();
        if amount <= 0.0 || self.grades[i] >= MAX_GRADE {
            return news;
        }
        self.xp[i] += amount;
        let mut ups = 0;
        while self.grades[i] < MAX_GRADE && self.xp[i] >= grade_need(self.grades[i]) {
            self.xp[i] -= grade_need(self.grades[i]);
            self.grades[i] += 1;
            ups += 1;
        }
        if self.grades[i] >= MAX_GRADE {
            self.xp[i] = 0.0;
        }
        if ups > 0 {
            news.push(format!("{}: grado {}", skill.name(), self.grades[i]));
            news.extend(self.add_level_xp(GRADE_XP * ups as f32));
        }
        news
    }

    /// Esperienza di livello (gradi guadagnati, missioni compiute).
    pub fn add_level_xp(&mut self, amount: f32) -> Vec<String> {
        let mut news = Vec::new();
        self.level_xp += amount * (1.0 + 0.05 * f32::from(self.perk_rank(Skill::Lore)));
        while self.level_xp >= level_need(self.level) {
            self.level_xp -= level_need(self.level);
            self.level += 1;
            self.stat_points += 1;
            self.nodes += 1;
            news.push(format!(
                "Livello {}: un punto statistica e un Nodo d'Oro",
                self.level
            ));
        }
        news
    }

    /// Rango che si può attivare adesso: (numero, nome), se il grado e i Nodi bastano.
    pub fn next_rank(&self, skill: Skill) -> Option<(u8, &'static str)> {
        skill.perk()?;
        let have = usize::from(self.perk_rank(skill));
        let (need, name) = *RANKS.get(have)?;
        (self.nodes > 0 && self.grade(skill) >= need).then_some((have as u8 + 1, name))
    }

    /// Spende un Nodo d'Oro per il rango successivo del talento.
    pub fn activate(&mut self, skill: Skill) -> bool {
        if self.next_rank(skill).is_none() {
            return false;
        }
        self.nodes -= 1;
        self.perks[skill.index()] += 1;
        true
    }

    /// Porta un'abilità a un grado preciso (shell dei trucchi).
    pub fn set_grade(&mut self, skill: Skill, grade: u8) {
        let i = skill.index();
        self.grades[i] = grade.clamp(1, MAX_GRADE);
        self.xp[i] = 0.0;
    }

    /// Porta il personaggio a un livello, con i punti dei livelli guadagnati (shell).
    pub fn set_level(&mut self, level: u32) {
        let level = level.max(1);
        let gained = level.saturating_sub(self.level);
        self.level = level;
        self.level_xp = 0.0;
        self.stat_points += gained;
        self.nodes += gained;
    }
}

// --- Canale globale: chi allena un'abilità non deve conoscere la risorsa -------------------

static QUEUE: Mutex<Vec<(Option<Skill>, f32)>> = Mutex::new(Vec::new());
static ACTIVE: [AtomicU8; COUNT] = [const { AtomicU8::new(0) }; COUNT];

/// Esperienza per un'abilità, applicata al prossimo frame.
pub fn train(skill: Skill, amount: f32) {
    if amount > 0.0
        && let Ok(mut q) = QUEUE.lock()
    {
        q.push((Some(skill), amount));
    }
}

/// Esperienza di livello (missioni compiute).
pub fn reward(amount: f32) {
    if let Ok(mut q) = QUEUE.lock() {
        q.push((None, amount));
    }
}

/// Ranghi attivati del talento di un'abilità (0..=5), per chi ne applica l'effetto.
pub fn rank(skill: Skill) -> f32 {
    f32::from(ACTIVE[skill.index()].load(Ordering::Relaxed))
}

/// Prezzo in negozio: carisma e talento dell'eloquenza.
pub fn price(stats: crate::character::Stats, base: u32) -> u32 {
    let price = stats.price(base) as f32 * (1.0 - 0.03 * rank(Skill::Eloquence));
    (price.round() as u32).max(1)
}

fn publish(progress: &Progress) {
    for (slot, rank) in ACTIVE.iter().zip(progress.perks) {
        slot.store(rank, Ordering::Relaxed);
    }
    for (slot, grade) in GRADES.iter().zip(progress.grades) {
        slot.store(grade, Ordering::Relaxed);
    }
}

static GRADES: [AtomicU8; COUNT] = [const { AtomicU8::new(1) }; COUNT];

/// Grado di un'abilità, per chi deve sapere se un talento attivo è sbloccato.
pub fn grade(skill: Skill) -> u8 {
    GRADES[skill.index()].load(Ordering::Relaxed)
}

/// Grado a cui si sbloccano i talenti attivi (Apprendista; ADR 0049).
pub const ACTIVE_TALENT: u8 = 30;

/// Il talento attivo dell'abilità è sbloccato?
pub fn active_talent(skill: Skill) -> bool {
    grade(skill) >= ACTIVE_TALENT
}

/// I talenti attivi: abilità, nome, che cosa fa (per il pannello del personaggio).
pub const ACTIVE_TALENTS: [(Skill, &str, &str); 5] = [
    (
        Skill::Swimming,
        "Apnea dei Pescatori di Perle",
        "[Ctrl] in acqua profonda ti immergi fino al fondale (fiato 20 s, di più col talento)",
    ),
    (
        Skill::Athletics,
        "Scatto di Ermes",
        "[X] un balzo rapido in avanti (20 di vigore)",
    ),
    (
        Skill::Blade,
        "Disarmo di Grazia",
        "un nemico ferito gravemente in mischia si arrende e se ne va",
    ),
    (
        Skill::Defense,
        "Controparata di Atena",
        "dopo un colpo parato con lo scudo, il prossimo fendente entro 2 s vale doppio",
    ),
    (
        Skill::Pickpocket,
        "Dita di Mercurio",
        "[F] alle spalle di un nemico ignaro; con le monete di un abitante sfili la chiave di casa e lettere o mappe",
    ),
];

/// Sistema: applica l'esperienza raccolta nel frame e avvisa di gradi e livelli.
pub fn apply_training(mut progress: ResMut<Progress>, mut session: ResMut<crate::save::Session>) {
    let pending = QUEUE
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default();
    if !pending.is_empty() {
        let mut news = Vec::new();
        for (skill, amount) in pending {
            news.extend(match skill {
                Some(skill) => progress.gain(skill, amount),
                None => progress.add_level_xp(amount),
            });
        }
        for line in &news {
            info!("[ABILITA] {line}");
        }
        // L'avviso più importante è l'ultimo (il livello viene dopo il grado).
        if let Some(last) = news.pop() {
            session.notify(last);
        }
    }
    if progress.is_changed() {
        publish(&progress);
    }
}

/// Sistema: parlare e frugare nei mobili allenano eloquenza e sapienza.
pub fn train_by_state(
    dialog: Res<crate::entities::DialogState>,
    state: Res<State<crate::AppState>>,
    mut was: Local<(bool, bool)>,
) {
    let searching = *state.get() == crate::AppState::Container;
    if dialog.open && !was.0 {
        train(Skill::Eloquence, 2.0);
    }
    if searching && !was.1 {
        train(Skill::Lore, 2.0);
    }
    *was = (dialog.open, searching);
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Progress>()
        .add_systems(Update, (train_by_state, apply_training).chain());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn using_a_skill_raises_its_grade_and_the_level() {
        let mut p = Progress::default();
        assert!(p.gain(Skill::Athletics, 5.0).is_empty());
        assert_eq!(p.grade(Skill::Athletics), 1);
        // Dal grado 1 al 2 servono 12 punti, dal 2 al 3 altri 14.
        let news = p.gain(Skill::Athletics, 7.0 + 14.0);
        assert_eq!(p.grade(Skill::Athletics), 3);
        assert_eq!(news, vec!["Atletica e Corsa: grado 3".to_string()]);
        assert_eq!(p.level_xp, 2.0 * GRADE_XP);
        assert_eq!(p.grade(Skill::Blade), 1);
    }

    #[test]
    fn a_level_gives_a_stat_point_a_node_and_health() {
        let mut p = Progress::default();
        let news = p.add_level_xp(level_need(1) + 1.0);
        assert_eq!((p.level, p.stat_points, p.nodes), (2, 1, 1));
        assert_eq!(p.health_bonus(), HEALTH_PER_LEVEL);
        assert_eq!(news.len(), 1);
        assert!(p.level_fraction() < 0.05);
    }

    #[test]
    fn a_perk_needs_the_grade_and_a_node() {
        let mut p = Progress::default();
        p.nodes = 2;
        assert!(!p.activate(Skill::Athletics), "grado troppo basso");
        p.set_grade(Skill::Athletics, 15);
        assert_eq!(p.next_rank(Skill::Athletics), Some((1, "Novizio")));
        assert!(p.activate(Skill::Athletics));
        assert!(
            !p.activate(Skill::Athletics),
            "il secondo rango vuole il grado 30"
        );
        assert_eq!((p.perk_rank(Skill::Athletics), p.nodes), (1, 1));
        p.set_grade(Skill::Building, 100);
        assert!(!p.activate(Skill::Building), "abilità senza talento");
    }

    #[test]
    fn grades_stop_at_one_hundred() {
        let mut p = Progress::default();
        p.gain(Skill::Blade, 1.0e6);
        assert_eq!(p.grade(Skill::Blade), MAX_GRADE);
        assert_eq!(p.grade_fraction(Skill::Blade), 1.0);
        assert!(p.gain(Skill::Blade, 100.0).is_empty());
        assert!(p.level > 5);
    }

    #[test]
    fn names_and_keys_are_found() {
        for s in Skill::ALL {
            assert_eq!(Skill::parse(s.key()), Some(s));
            assert_eq!(s.perk().is_some(), !s.how().is_empty(), "{s:?}");
        }
        assert_eq!(Skill::parse("Furtività"), Some(Skill::Stealth));
        assert_eq!(Skill::parse("boh"), None);
    }
}
