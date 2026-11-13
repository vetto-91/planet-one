//! Shell dei trucchi (tasto `\`): godmode, teletrasporto, debug F3/F4, modelli IA e voci.
//!
//! Il gioco normale non ha trucchi: tutto parte spento a ogni avvio e nulla viene salvato.
//! I comandi passano da [`parse`] (funzione pura) e sono eseguiti da [`shell_execute`].

use crate::AppState;
use crate::entities::{DialogState, Npc};
use crate::inventory::{Inventory, ItemKind};
use crate::player::Player;
use crate::villagers::NpcInventory;
use avian3d::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

const HELP: &[&str] = &[
    "aiuto · stato · pulisci · reset (ricostruisce la scena)",
    "god [on|off] · noclip [on|off] · invisibile [on|off]",
    "tp <x> <y> <z> · tp <luogo o abitante> [n] (es. tp chiesa, tp porta nord, tp marta)",
    "passepartout [off] · chiavi · dai <oggetto> [quantità]",
    "f3 (collider) · f4 (piazzamenti) · debug [on|off] (tasti F3 e F4) · coordinate [on|off]",
    "ora <0-24> · meteo <sereno|nuvoloso|pioggia|temporale|nebbia|neve|auto>",
    "ia [on|off] · ia modelli · ia modello <nome>",
    "tts [on|off] · tts voci · tts voce <nome|auto>",
    "stt [on|off] · stt modelli · stt modello <nome>",
    "stat [<nome> <valore>] · effetto peso <kg> [secondi] · effetto via",
    "vai|corri <x> <z> · vai <luogo o abitante> · dove <nome> · guarda <yaw> <pitch>|<nome> [distanza]",
    "foto [nome] · dump [nome] (stato in JSON) · copione <file> (pilota automatico)",
    "quest <inizio|cerca|raccogli|torna|completata> · uccidi [raggio] · vicino [raggio] · fps · entita",
    "volume <musica|ambiente|voci> <0-100> (solo per questa partita)",
    "regione <isola|nord> (viaggio fra l'Isola di Fontevivo e il Continente del Nord)",
    "missioni [on|off] · missione <id> <tappa> (storia del Prologo e secondarie)",
    "veloce [luogo] (viaggio veloce verso un luogo scoperto; senza nome: l'elenco)",
    "abilita [<nome> <grado>] · livello [n] · talento <abilità> (attiva un rango con un Nodo d'Oro)",
    "dietro <abitante> (alle sue spalle, per il borseggio) · scassina (la serratura aperta si forza da sola)",
    "karma [n] · fiducia <abitante> <n> · davanti <abitante> · volti (diario su «Volti e segreti»)",
    "incantesimo <luce|dardo|cura|passo|scudo> · magia <n> (riserva di magia)",
];

/// Trucchi attivi: non salvati, tutti spenti a ogni avvio.
#[derive(Resource, Default, Debug)]
pub struct Cheats {
    pub god: bool,
    pub noclip: bool,
    pub invisible: bool,
    /// I tasti F3 e F4 funzionano solo con questa opzione.
    pub debug_keys: bool,
    pub coords: bool,
}

/// Stato della shell: righe a video, comandi da eseguire, storico.
#[derive(Resource, Default)]
pub struct Shell {
    pub input: String,
    pub log: Vec<String>,
    pub pending: Vec<String>,
    history: Vec<String>,
    cursor: Option<usize>,
}

impl Shell {
    fn say(&mut self, line: impl Into<String>) {
        let line = line.into();
        info!("[SHELL] {line}");
        self.log.push(line);
        let excess = self.log.len().saturating_sub(300);
        self.log.drain(..excess);
    }
}

/// Dove teletrasportarsi.
#[derive(Debug, PartialEq, Clone)]
pub enum Target {
    Pos(Vec3),
    Place(String, usize),
}

#[derive(Debug, PartialEq, Clone)]
pub enum AiCmd {
    Status,
    Switch(bool),
    Models,
    Model(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum TtsCmd {
    Status,
    Switch(bool),
    Voices,
    /// Vuoto = voce automatica del personaggio.
    Voice(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum SttCmd {
    Status,
    Switch(bool),
    Models,
    Model(String),
}

/// Comando riconosciuto. `None` nei trucchi vuol dire "inverti".
#[derive(Debug, PartialEq, Clone)]
pub enum Cmd {
    Help,
    Status,
    Clear,
    Reset,
    /// Viaggio veloce verso un luogo scoperto (senza nome: l'elenco).
    Fast(String),
    /// Missioni del Prologo: accese/spente, o salto a una tappa.
    Missions(Option<bool>),
    Mission(String, usize),
    /// Viaggio in un'altra regione del mondo.
    Region(crate::region::Region),
    God(Option<bool>),
    Noclip(Option<bool>),
    Invisible(Option<bool>),
    Teleport(Target),
    Master(bool),
    Keys,
    Give(String, u32),
    F3,
    F4,
    Debug(Option<bool>),
    Coords(Option<bool>),
    Hour(f32),
    Weather(Option<crate::weather::Kind>),
    Ai(AiCmd),
    Tts(TtsCmd),
    Stt(SttCmd),
    Stats,
    SetStat(crate::character::Stat, u8),
    Skills,
    SetSkill(crate::skills::Skill, u8),
    Level(Option<u32>),
    Perk(crate::skills::Skill),
    /// Lancia un incantesimo; riempie o imposta la riserva di magia.
    Cast(crate::magic::Spell),
    SetMagic(f32),
    /// Karma: lo mostra o lo imposta; fiducia di un abitante.
    Karma(Option<i32>),
    Trust(String, i32),
    /// Alle spalle di un abitante (per provare il borseggio).
    Behind(String, bool),
    /// Apre il diario sulla scheda «Volti e segreti».
    Faces,
    /// Durante lo scasso: il perno si spinge da solo quando l'ago è sulla tacca.
    AutoPick,
    /// Chili e secondi (`None` = senza scadenza).
    CarryEffect(f32, Option<f32>),
    ClearEffects,
    /// Cammina davvero fino a una meta (`vai`, `corri`): il pilota tiene premuto W.
    Go(Target, bool),
    /// Coordinate di un luogo o abitante.
    Where(String),
    /// Orienta la camera: angoli in gradi oppure verso un luogo o abitante.
    Look(LookTarget),
    Photo(String),
    Dump(String),
    /// Esegue un copione del pilota automatico.
    Script(String),
    /// Porta la quest «Denaro Rubato» a una fase (per provare i rami).
    Quest(crate::quests::denaro_rubato::QuestStage),
    /// Uccide il nemico più vicino entro il raggio.
    Kill(f32),
    /// Elenca abitanti e nemici entro il raggio.
    Near(f32),
    Fps,
    Entities,
    /// Volume (0-100) di musica, ambiente o voci; non viene salvato.
    Volume(String, f32),
}

#[derive(Debug, PartialEq, Clone)]
pub enum LookTarget {
    Angles(f32, f32),
    Place(String),
}

fn switch(arg: Option<&&str>) -> Result<Option<bool>, String> {
    match arg.map(|a| a.to_lowercase()).as_deref() {
        None => Ok(None),
        Some("on" | "si" | "sì") => Ok(Some(true)),
        Some("off" | "no") => Ok(Some(false)),
        Some(other) => Err(format!("usa on oppure off, non «{other}»")),
    }
}

/// Interpreta una riga. Il modello e la voce mantengono maiuscole e minuscole.
pub fn parse(line: &str) -> Result<Cmd, String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let Some(first) = words.first() else {
        return Err("comando vuoto".into());
    };
    let args = &words[1..];
    match first.to_lowercase().as_str() {
        "aiuto" | "help" | "?" => Ok(Cmd::Help),
        "stato" | "status" => Ok(Cmd::Status),
        "pulisci" | "clear" => Ok(Cmd::Clear),
        "reset" => Ok(Cmd::Reset),
        "regione" | "region" | "viaggia" => args
            .first()
            .and_then(|a| crate::region::Region::parse(a))
            .map(Cmd::Region)
            .ok_or_else(|| "uso: regione <isola|nord>".to_string()),
        "vai" | "go" | "corri" | "run" => {
            let sprint = matches!(first.to_lowercase().as_str(), "corri" | "run");
            let numbers: Vec<f32> = args
                .iter()
                .filter_map(|a| a.replace(',', ".").parse().ok())
                .collect();
            if numbers.len() == 2 && args.len() == 2 {
                return Ok(Cmd::Go(
                    Target::Pos(Vec3::new(numbers[0], 0.0, numbers[1])),
                    sprint,
                ));
            }
            match parse_teleport(args)? {
                Cmd::Teleport(t) => Ok(Cmd::Go(t, sprint)),
                _ => Err("uso: vai <x> <z> oppure vai <luogo o abitante> [n]".into()),
            }
        }
        "dove" | "where" => {
            if args.is_empty() {
                Err("uso: dove <luogo o abitante>".into())
            } else {
                Ok(Cmd::Where(args.join(" ").to_lowercase()))
            }
        }
        "guarda" | "look" => {
            let numbers: Vec<f32> = args
                .iter()
                .filter_map(|a| a.replace(',', ".").parse().ok())
                .collect();
            match (numbers.as_slice(), args.len()) {
                ([yaw, pitch], 2) => Ok(Cmd::Look(LookTarget::Angles(*yaw, *pitch))),
                (_, 0) => Err("uso: guarda <yaw> <pitch> oppure guarda <luogo o abitante>".into()),
                _ => Ok(Cmd::Look(LookTarget::Place(args.join(" ").to_lowercase()))),
            }
        }
        "quest" => {
            use crate::quests::denaro_rubato::QuestStage as Q;
            match args.first().map(|a| a.to_lowercase()).as_deref() {
                Some("idle" | "inizio" | "nessuna") => Ok(Cmd::Quest(Q::Idle)),
                Some("seekthief" | "cerca" | "ladro") => Ok(Cmd::Quest(Q::SeekThief)),
                Some("collecting" | "raccogli" | "monete") => Ok(Cmd::Quest(Q::Collecting)),
                Some("returning" | "ritorna" | "torna") => Ok(Cmd::Quest(Q::Returning)),
                Some("completed" | "completata" | "fine") => Ok(Cmd::Quest(Q::Completed)),
                _ => Err("uso: quest <inizio|cerca|raccogli|torna|completata>".into()),
            }
        }
        "veloce" | "fast" => Ok(Cmd::Fast(args.join(" ").to_lowercase())),
        "missioni" => Ok(Cmd::Missions(match args.first().copied() {
            Some("on" | "si" | "sì") => Some(true),
            Some("off" | "no") => Some(false),
            _ => None,
        })),
        "missione" => match (
            args.first(),
            args.get(1).and_then(|a| a.parse::<usize>().ok()),
        ) {
            (Some(id), Some(step)) if step >= 1 => Ok(Cmd::Mission((*id).to_string(), step - 1)),
            _ => Err("uso: missione <mp01|mp03|mp04|mp05> <tappa da 1>".into()),
        },
        "uccidi" | "kill" => Ok(Cmd::Kill(
            args.first().and_then(|a| a.parse().ok()).unwrap_or(30.0),
        )),
        "vicino" | "near" => Ok(Cmd::Near(
            args.first().and_then(|a| a.parse().ok()).unwrap_or(15.0),
        )),
        "fps" => Ok(Cmd::Fps),
        "volume" => match (
            args.first(),
            args.get(1).and_then(|v| v.parse::<f32>().ok()),
        ) {
            (Some(what), Some(v))
                if matches!(what.to_lowercase().as_str(), "musica" | "ambiente" | "voci")
                    && (0.0..=100.0).contains(&v) =>
            {
                Ok(Cmd::Volume(what.to_lowercase(), v))
            }
            _ => Err("uso: volume <musica|ambiente|voci> <0-100>".into()),
        },
        "entita" | "entità" | "entities" => Ok(Cmd::Entities),
        "foto" | "shot" => Ok(Cmd::Photo(args.first().unwrap_or(&"foto").to_string())),
        "dump" => Ok(Cmd::Dump(args.first().unwrap_or(&"stato").to_string())),
        "copione" | "script" => match args.first() {
            Some(path) => Ok(Cmd::Script((*path).to_string())),
            None => Err("uso: copione <file>".into()),
        },
        "god" | "godmode" => Ok(Cmd::God(switch(args.first())?)),
        "noclip" => Ok(Cmd::Noclip(switch(args.first())?)),
        "invisibile" | "invis" => Ok(Cmd::Invisible(switch(args.first())?)),
        "tp" | "teletrasporto" => parse_teleport(args),
        "passepartout" => Ok(Cmd::Master(switch(args.first())? != Some(false))),
        "chiavi" => Ok(Cmd::Keys),
        "dai" | "give" => parse_give(args),
        "f3" => Ok(Cmd::F3),
        "f4" => Ok(Cmd::F4),
        "debug" => Ok(Cmd::Debug(switch(args.first())?)),
        "coordinate" | "coords" => Ok(Cmd::Coords(switch(args.first())?)),
        "ora" => {
            let hour: f32 = args
                .first()
                .and_then(|a| a.replace(',', ".").parse().ok())
                .ok_or("uso: ora <0-24>")?;
            if (0.0..=24.0).contains(&hour) {
                Ok(Cmd::Hour(hour.rem_euclid(24.0)))
            } else {
                Err("l'ora va da 0 a 24".into())
            }
        }
        "meteo" => match args.first().copied() {
            Some("auto") => Ok(Cmd::Weather(None)),
            Some(name) => crate::weather::Kind::parse(name)
                .map(|k| Cmd::Weather(Some(k)))
                .ok_or_else(|| format!("meteo sconosciuto: {name}")),
            None => Err("uso: meteo <tipo|auto>".into()),
        },
        "ia" | "ai" => match args {
            [] => Ok(Cmd::Ai(AiCmd::Status)),
            ["modelli"] => Ok(Cmd::Ai(AiCmd::Models)),
            ["modello", name] => Ok(Cmd::Ai(AiCmd::Model((*name).to_string()))),
            [_] => switch(args.first())?
                .map(|on| Cmd::Ai(AiCmd::Switch(on)))
                .ok_or_else(|| "uso: ia on|off".to_string()),
            _ => Err("uso: ia [on|off] · ia modelli · ia modello <nome>".into()),
        },
        "tts" => match args {
            [] => Ok(Cmd::Tts(TtsCmd::Status)),
            ["voci"] => Ok(Cmd::Tts(TtsCmd::Voices)),
            ["voce", "auto"] => Ok(Cmd::Tts(TtsCmd::Voice(String::new()))),
            ["voce", name] => Ok(Cmd::Tts(TtsCmd::Voice((*name).to_string()))),
            [_] => switch(args.first())?
                .map(|on| Cmd::Tts(TtsCmd::Switch(on)))
                .ok_or_else(|| "uso: tts on|off".to_string()),
            _ => Err("uso: tts [on|off] · tts voci · tts voce <nome|auto>".into()),
        },
        "stt" => match args {
            [] => Ok(Cmd::Stt(SttCmd::Status)),
            ["modelli"] => Ok(Cmd::Stt(SttCmd::Models)),
            ["modello", name] => Ok(Cmd::Stt(SttCmd::Model((*name).to_string()))),
            [_] => switch(args.first())?
                .map(|on| Cmd::Stt(SttCmd::Switch(on)))
                .ok_or_else(|| "uso: stt on|off".to_string()),
            _ => Err("uso: stt [on|off] · stt modelli · stt modello <nome>".into()),
        },
        "abilita" | "abilità" => match args {
            [] => Ok(Cmd::Skills),
            [name, grade] => {
                let skill = crate::skills::Skill::parse(name)
                    .ok_or_else(|| format!("abilità sconosciuta: {name}"))?;
                let grade: u8 = grade.parse().map_err(|_| "il grado va da 1 a 100")?;
                if (1..=crate::skills::MAX_GRADE).contains(&grade) {
                    Ok(Cmd::SetSkill(skill, grade))
                } else {
                    Err("il grado va da 1 a 100".into())
                }
            }
            _ => Err("uso: abilita [<nome> <grado>]".into()),
        },
        "livello" => match args {
            [] => Ok(Cmd::Level(None)),
            [n] => n
                .parse::<u32>()
                .ok()
                .filter(|n| (1..=200).contains(n))
                .map(|n| Cmd::Level(Some(n)))
                .ok_or_else(|| "il livello va da 1 a 200".to_string()),
            _ => Err("uso: livello [n]".into()),
        },
        "dietro" if !args.is_empty() => Ok(Cmd::Behind(args.join(" ").to_lowercase(), true)),
        "davanti" if !args.is_empty() => Ok(Cmd::Behind(args.join(" ").to_lowercase(), false)),
        "volti" => Ok(Cmd::Faces),
        "scassina" => Ok(Cmd::AutoPick),
        "incantesimo" if !args.is_empty() => crate::magic::Spell::parse(&args.join(" "))
            .map(Cmd::Cast)
            .ok_or_else(|| "incantesimi: luce, dardo, cura, passo, scudo".to_string()),
        "magia" => args
            .first()
            .and_then(|a| a.parse::<f32>().ok())
            .filter(|n| (0.0..=999.0).contains(n))
            .map(Cmd::SetMagic)
            .ok_or_else(|| "uso: magia <0-999>".to_string()),
        "karma" => match args {
            [] => Ok(Cmd::Karma(None)),
            [n] => n
                .parse::<i32>()
                .ok()
                .filter(|n| (-100..=100).contains(n))
                .map(|n| Cmd::Karma(Some(n)))
                .ok_or_else(|| "il karma va da -100 a 100".to_string()),
            _ => Err("uso: karma [n]".into()),
        },
        "fiducia" if args.len() >= 2 => {
            let (n, name) = args.split_last().ok_or("uso: fiducia <abitante> <n>")?;
            let n: i32 = n.parse().map_err(|_| "uso: fiducia <abitante> <n>")?;
            Ok(Cmd::Trust(name.join(" ").to_lowercase(), n))
        }
        "talento" => match args {
            [name] => crate::skills::Skill::parse(name)
                .map(Cmd::Perk)
                .ok_or_else(|| format!("abilità sconosciuta: {name}")),
            _ => Err("uso: talento <abilità>".into()),
        },
        "stat" | "statistiche" => match args {
            [] => Ok(Cmd::Stats),
            [name, value] => {
                let stat = crate::character::Stat::parse(name)
                    .ok_or_else(|| format!("statistica sconosciuta: {name}"))?;
                let value: u8 = value.parse().map_err(|_| "il valore va da 1 a 20")?;
                if (crate::character::STAT_MIN..=crate::character::STAT_MAX).contains(&value) {
                    Ok(Cmd::SetStat(stat, value))
                } else {
                    Err("il valore va da 1 a 20".into())
                }
            }
            _ => Err("uso: stat [<nome> <valore>]".into()),
        },
        "effetto" => match args {
            ["via"] => Ok(Cmd::ClearEffects),
            ["peso", kg, rest @ ..] if rest.len() <= 1 => {
                let kg: f32 = kg
                    .replace(',', ".")
                    .parse()
                    .map_err(|_| "uso: effetto peso <kg> [secondi]")?;
                let secs = match rest.first() {
                    Some(s) => Some(
                        s.replace(',', ".")
                            .parse::<f32>()
                            .map_err(|_| "uso: effetto peso <kg> [secondi]")?,
                    ),
                    None => None,
                };
                Ok(Cmd::CarryEffect(kg, secs))
            }
            _ => Err("uso: effetto peso <kg> [secondi] · effetto via".into()),
        },
        other => Err(format!("comando sconosciuto: {other} (scrivi aiuto)")),
    }
}

fn parse_teleport(args: &[&str]) -> Result<Cmd, String> {
    if args.is_empty() {
        return Err("uso: tp <x> <y> <z> oppure tp <luogo o abitante> [n]".into());
    }
    let numbers: Vec<f32> = args
        .iter()
        .filter_map(|a| a.replace(',', ".").parse().ok())
        .collect();
    if numbers.len() == 3 && args.len() == 3 {
        return Ok(Cmd::Teleport(Target::Pos(Vec3::new(
            numbers[0], numbers[1], numbers[2],
        ))));
    }
    // Un numero finale sceglie quale edificio di quel tipo.
    let (name, n) = match args.split_last() {
        Some((last, rest)) if !rest.is_empty() && last.parse::<usize>().is_ok() => {
            (rest.join(" "), last.parse().unwrap_or(0))
        }
        _ => (args.join(" "), 0),
    };
    Ok(Cmd::Teleport(Target::Place(name.to_lowercase(), n)))
}

fn parse_give(args: &[&str]) -> Result<Cmd, String> {
    let (name, qty) = match args.split_last() {
        Some((last, rest)) if !rest.is_empty() && last.parse::<u32>().is_ok() => {
            (rest.join(" "), last.parse().unwrap_or(1))
        }
        _ => (args.join(" "), 1),
    };
    if name.is_empty() {
        return Err("uso: dai <oggetto> [quantità]".into());
    }
    Ok(Cmd::Give(name.to_lowercase(), qty))
}

/// Nemici visti dalla shell (`uccidi`, `vicino`).
type EnemyData = (
    &'static GlobalTransform,
    &'static mut crate::combat::EnemyHealth,
);
type EnemyFilter = (With<crate::entities::Enemy>, Without<Player>);

/// Tutto ciò che i comandi possono toccare.
#[derive(SystemParam)]
pub struct ShellCtx<'w, 's> {
    cheats: ResMut<'w, Cheats>,
    character: ResMut<'w, crate::character::Character>,
    effects: ResMut<'w, crate::character::ActiveEffects>,
    inventory: ResMut<'w, Inventory>,
    clock: ResMut<'w, crate::daynight::GameClock>,
    weather: ResMut<'w, crate::weather::WeatherState>,
    audio: ResMut<'w, crate::audio::AudioSettings>,
    ai: ResMut<'w, crate::ai::AiState>,
    tts: Res<'w, crate::ai::tts::TtsState>,
    stt: Res<'w, crate::ai::stt::SttState>,
    place: ResMut<'w, crate::debug_place::DebugPlace>,
    gizmos: ResMut<'w, GizmoConfigStore>,
    homes: Res<'w, crate::world_props::Homes>,
    pilot: ResMut<'w, crate::autopilot::Autopilot>,
    quest: ResMut<'w, crate::quests::denaro_rubato::QuestState>,
    missions: ResMut<'w, crate::quests::missions::Missions>,
    known: Res<'w, crate::places::Known>,
    progress: ResMut<'w, crate::skills::Progress>,
    picking: ResMut<'w, crate::thievery::Lockpicking>,
    karma: ResMut<'w, crate::karma::Karma>,
    magic: ResMut<'w, crate::magic::Magic>,
    vitals: ResMut<'w, crate::vitals::Vitals>,
    journal_tab: ResMut<'w, crate::menu::JournalTab>,
    fast_travel: ResMut<'w, crate::places::FastTravel>,
    diagnostics: Res<'w, bevy::diagnostic::DiagnosticsStore>,
    every: Query<'w, 's, Entity>,
    enemies: Query<'w, 's, EnemyData, EnemyFilter>,
    orbit: ResMut<'w, crate::camera::OrbitCamera>,
    village: Res<'w, crate::village::VillageInfo>,
    field: Option<Res<'w, crate::terrain::TerrainField>>,
    player: Query<'w, 's, (&'static mut Transform, &'static mut LinearVelocity), With<Player>>,
    npcs: Query<
        'w,
        's,
        (
            &'static Npc,
            &'static GlobalTransform,
            Option<&'static NpcInventory>,
        ),
        Without<Player>,
    >,
}

/// Quanto un nome corrisponde a quello cercato (minuscolo): 2 esatto, 1 lo contiene, 0 no.
/// «gino» è il mugnaio Gino, non la Guardia Gino.
pub fn name_rank(full: &str, wanted: &str) -> u8 {
    let full = full.to_lowercase();
    if full == wanted {
        2
    } else {
        u8::from(full.contains(wanted))
    }
}

fn on_off(v: bool) -> &'static str {
    if v { "attivo" } else { "spento" }
}

/// Punto di arrivo di un teletrasporto per nome: porta, abitante, edificio, oggetto piazzato.
fn resolve_place(ctx: &ShellCtx, name: &str, n: usize) -> Option<Vec3> {
    let ground = |p: Vec2| {
        let y = ctx.field.as_ref().map_or(1.0, |f| f.height_at(p.x, p.y));
        Vec3::new(p.x, y + 1.2, p.y)
    };
    if let Some(side) = name.strip_prefix("porta ")
        && let Some(g) = ctx
            .village
            .gates
            .iter()
            .find(|g| g.name.eq_ignore_ascii_case(side))
    {
        return Some(ground(g.pos + g.inward * 6.0));
    }
    if let Some((_, t, _)) = ctx
        .npcs
        .iter()
        .max_by_key(|(npc, ..)| name_rank(npc.name, name))
        .filter(|(npc, ..)| name_rank(npc.name, name) > 0)
    {
        // Posizione nel mondo: il Viandante è figlio della casetta.
        return Some(t.translation() + Vec3::new(1.2, 0.4, 0.0));
    }
    if let Some(home) = ctx.homes.find(name, n) {
        return Some(ground(home.door));
    }
    ctx.place
        .items
        .iter()
        .filter(|p| p.kind == name)
        .flat_map(|p| p.points.iter())
        .nth(n)
        .map(|p| ground(*p))
}

fn apply_switch(flag: &mut bool, wanted: Option<bool>) -> bool {
    *flag = wanted.unwrap_or(!*flag);
    *flag
}

fn run(cmd: Cmd, shell: &mut Shell, ctx: &mut ShellCtx) {
    match cmd {
        Cmd::Help => {
            for line in HELP {
                shell.say(*line);
            }
        }
        Cmd::Status => {
            let c = &ctx.cheats;
            shell.say(format!(
                "god {} · noclip {} · invisibile {} · tasti debug {} · coordinate {}",
                on_off(c.god),
                on_off(c.noclip),
                on_off(c.invisible),
                on_off(c.debug_keys),
                on_off(c.coords)
            ));
            shell.say(format!(
                "IA: {} ({}) · TTS: {} · passepartout: {}",
                ctx.ai.status.label(),
                ctx.audio.ai_model,
                ctx.tts.status.label(),
                if ctx.inventory.qty(ItemKind::MasterKey) > 0 {
                    "sì"
                } else {
                    "no"
                }
            ));
        }
        Cmd::Clear => shell.log.clear(),
        // Gestito in `shell_execute`, che può cambiare stato.
        Cmd::Reset | Cmd::Region(_) => {}
        Cmd::God(v) => {
            let on = apply_switch(&mut ctx.cheats.god, v);
            shell.say(format!("godmode {}", on_off(on)));
        }
        Cmd::Noclip(v) => {
            let on = apply_switch(&mut ctx.cheats.noclip, v);
            shell.say(format!("noclip {} (Spazio sale, Ctrl scende)", on_off(on)));
        }
        Cmd::Invisible(v) => {
            let on = apply_switch(&mut ctx.cheats.invisible, v);
            shell.say(format!("invisibilità {}", on_off(on)));
        }
        Cmd::Teleport(target) => {
            let dest = match target {
                Target::Pos(p) => Some(p),
                Target::Place(name, n) => {
                    let found = resolve_place(ctx, &name, n);
                    if found.is_none() {
                        shell.say(format!("luogo sconosciuto: {name}"));
                    }
                    found
                }
            };
            if let Some(dest) = dest
                && let Ok((mut t, mut v)) = ctx.player.single_mut()
            {
                t.translation = dest;
                v.0 = Vec3::ZERO;
                shell.say(format!(
                    "teletrasporto a {:.1} {:.1} {:.1}",
                    dest.x, dest.y, dest.z
                ));
            }
        }
        Cmd::Go(target, sprint) => {
            let is_npc = matches!(&target, Target::Place(name, _)
                if ctx.npcs.iter().any(|(npc, ..)| npc.name.to_lowercase().contains(name.as_str())));
            let dest = match target {
                Target::Pos(p) => Some(p),
                // Verso un abitante si punta a lui, non al punto di teletrasporto accanto.
                Target::Place(name, n) => resolve_place(ctx, &name, n).map(|p| {
                    if is_npc {
                        p - Vec3::new(1.2, 0.4, 0.0)
                    } else {
                        p
                    }
                }),
            };
            match (dest, ctx.player.single()) {
                (Some(dest), Ok((t, _))) => {
                    let label = format!("({:.1}, {:.1})", dest.x, dest.z);
                    shell.say(format!("in cammino verso {label}"));
                    ctx.pilot.active = true;
                    let mut goal =
                        crate::autopilot::Goal::new(dest.xz(), label, t.translation.xz(), sprint);
                    // Da un abitante ci si ferma a portata di voce, non addosso.
                    if is_npc {
                        goal.arrive = 2.2;
                    }
                    ctx.pilot.goal = Some(goal);
                }
                _ => shell.say("meta sconosciuta"),
            }
        }
        Cmd::Where(name) => match resolve_place(ctx, &name, 0) {
            Some(p) => shell.say(format!("{name}: {:.1} {:.1} {:.1}", p.x, p.y, p.z)),
            None => shell.say(format!("luogo sconosciuto: {name}")),
        },
        Cmd::Look(target) => match target {
            LookTarget::Angles(yaw, pitch) => {
                ctx.orbit.yaw = yaw.to_radians();
                ctx.orbit.pitch = pitch.to_radians();
                shell.say(format!("camera: yaw {yaw:.0}°, pitch {pitch:.0}°"));
            }
            LookTarget::Place(text) => {
                // Un numero in fondo è la distanza della camera (foto da lontano).
                let (name, distance) = match text.rsplit_once(' ') {
                    Some((n, d)) if d.parse::<f32>().is_ok() => {
                        (n.to_string(), d.parse::<f32>().ok())
                    }
                    _ => (text.clone(), None),
                };
                // Un abitante si mira al petto; un edificio a metà facciata (non alla porta,
                // che è sotto i piedi di chi ci si è appena teletrasportato); un luogo nel punto di arrivo.
                let npc = ctx
                    .npcs
                    .iter()
                    .max_by_key(|(npc, ..)| name_rank(npc.name, &name))
                    .filter(|(npc, ..)| name_rank(npc.name, &name) > 0)
                    .map(|(_, t, _)| t.translation() + Vec3::Y * 0.2);
                let building = ctx
                    .place
                    .items
                    .iter()
                    .filter(|p| p.kind == name)
                    .flat_map(|p| p.points.iter())
                    .next()
                    .map(|p| {
                        let y = ctx.field.as_ref().map_or(0.0, |f| f.height_at(p.x, p.y));
                        Vec3::new(p.x, y + 3.0, p.y)
                    });
                match npc.or(building).or_else(|| resolve_place(ctx, &name, 0)) {
                    Some(p) => {
                        ctx.pilot.active = true;
                        ctx.pilot.aim = Some((p, 0));
                        if let Some(d) = distance {
                            ctx.orbit.distance = d.clamp(1.5, 30.0);
                        }
                        shell.say(format!("mira a {:.1} {:.1} {:.1}", p.x, p.y, p.z));
                    }
                    None => shell.say("non trovo dove guardare"),
                }
            }
        },
        Cmd::Photo(name) => {
            let dir = if ctx.pilot.out.as_os_str().is_empty() {
                crate::ai::tts::data_dir().join("foto")
            } else {
                ctx.pilot.out.clone()
            };
            ctx.pilot.photo = Some(dir.join(format!("{name}.png")));
            shell.say(format!("foto {name}"));
        }
        Cmd::Dump(name) => {
            let dir = if ctx.pilot.out.as_os_str().is_empty() {
                crate::ai::tts::data_dir().join("stato")
            } else {
                ctx.pilot.out.clone()
            };
            ctx.pilot.dump = Some(Some(dir.join(format!("stato-{name}.json"))));
            shell.say(format!("stato in {}", dir.display()));
        }
        Cmd::Fast(name) => {
            if name.is_empty() {
                let prefix = format!("{}|", crate::region::current().key());
                let names: Vec<&str> = ctx
                    .known
                    .0
                    .keys()
                    .filter_map(|k| k.strip_prefix(&prefix))
                    .collect();
                shell.say(format!("luoghi scoperti: {}", names.join(", ")));
            } else {
                match ctx.known.find(&name) {
                    Some(place) => {
                        shell.say(format!("viaggio veloce verso {}", place.0));
                        ctx.fast_travel.0 = Some(place);
                    }
                    None => shell.say(format!("luogo non ancora scoperto: {name}")),
                }
            }
        }
        Cmd::Missions(on) => {
            if let Some(on) = on {
                ctx.missions.enabled = on;
            }
            shell.say(format!("missioni {}", on_off(ctx.missions.enabled)));
            let done = ctx.quest.stage == crate::quests::denaro_rubato::QuestStage::Completed;
            for e in ctx.missions.journal(done) {
                let goal = e
                    .objectives
                    .last()
                    .map_or(String::new(), |(g, _)| g.clone());
                shell.say(format!("  {} — {}", e.title, goal));
            }
        }
        Cmd::Mission(id, step) => match ctx.missions.jump(&id, step) {
            Ok(text) | Err(text) => shell.say(text),
        },
        Cmd::Quest(stage) => {
            use crate::quests::denaro_rubato::QuestStage as Q;
            let q = &mut ctx.quest;
            q.stage = stage;
            q.accepted = stage != Q::Idle;
            q.final_done = stage == Q::Completed;
            q.coins_collected = match stage {
                Q::Returning | Q::Completed => q.coins_total,
                _ => 0,
            };
            shell.say(format!("quest «Denaro Rubato»: fase {stage:?}"));
        }
        Cmd::Kill(radius) => {
            let here = ctx.player.single().map(|(t, _)| t.translation).ok();
            let target = here.and_then(|p| {
                ctx.enemies
                    .iter_mut()
                    .filter(|(_, hp)| hp.current > 0.0)
                    .map(|(g, hp)| (g.translation().distance(p), hp))
                    .filter(|(d, _)| *d <= radius)
                    .min_by(|a, b| a.0.total_cmp(&b.0))
            });
            match target {
                Some((dist, mut hp)) => {
                    hp.current = 0.0;
                    shell.say(format!("nemico a {dist:.1} m: ucciso"));
                }
                None => shell.say(format!("nessun nemico entro {radius:.0} m")),
            }
        }
        Cmd::Near(radius) => {
            let Ok((t, _)) = ctx.player.single() else {
                return;
            };
            let p = t.translation;
            let mut found: Vec<(f32, String)> = ctx
                .npcs
                .iter()
                .map(|(npc, g, _)| {
                    let d = g.translation().distance(p);
                    (d, format!("{} ({:?})", npc.name, npc.mode))
                })
                .chain(ctx.enemies.iter().map(|(g, hp)| {
                    let d = g.translation().distance(p);
                    (d, format!("nemico, vita {:.0}", hp.current))
                }))
                .filter(|(d, _)| *d <= radius)
                .collect();
            found.sort_by(|a, b| a.0.total_cmp(&b.0));
            if found.is_empty() {
                shell.say(format!("nessuno entro {radius:.0} m"));
            }
            for (d, what) in found {
                shell.say(format!("{d:5.1} m  {what}"));
            }
        }
        Cmd::Fps => {
            let fps = ctx
                .diagnostics
                .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
                .and_then(|d| d.smoothed());
            shell.say(match fps {
                Some(f) => format!("FPS: {f:.0}"),
                None => "FPS non disponibili".to_string(),
            });
        }
        Cmd::Volume(what, v) => {
            let level = v / 100.0;
            match what.as_str() {
                "musica" => ctx.audio.music_volume = level,
                "ambiente" => ctx.audio.ambient_volume = level,
                _ => ctx.audio.voice_volume = level,
            }
            shell.say(format!("volume {what}: {v:.0}%"));
        }
        Cmd::Entities => {
            shell.say(format!("entità: {}", ctx.every.iter().count()));
        }
        Cmd::Script(path) => match std::fs::read_to_string(&path) {
            Ok(text) => {
                ctx.pilot.load(&path, &text);
                shell.say(format!("copione {path} avviato"));
            }
            Err(e) => shell.say(format!("copione {path}: {e}")),
        },
        Cmd::Master(give) => {
            let have = ctx.inventory.qty(ItemKind::MasterKey);
            if give && have == 0 {
                ctx.inventory.add(ItemKind::MasterKey, 1);
            } else if !give && have > 0 {
                ctx.inventory.consume(ItemKind::MasterKey, have);
            }
            shell.say(if give {
                "passepartout nell'inventario"
            } else {
                "passepartout tolto"
            });
        }
        Cmd::Keys => {
            for (npc, _, keys) in &ctx.npcs {
                if let Some(keys) = keys.filter(|k| !k.keys.is_empty()) {
                    shell.say(format!("{}: {}", npc.name, keys.keys.join(", ")));
                }
            }
        }
        Cmd::Give(name, qty) => {
            match ItemKind::ALL
                .iter()
                .find(|k| k.name().to_lowercase().contains(&name))
            {
                Some(kind) => {
                    ctx.inventory.add(*kind, qty);
                    shell.say(format!("{} x{qty}", kind.name()));
                }
                None => shell.say(format!("oggetto sconosciuto: {name}")),
            }
        }
        Cmd::F3 => {
            let cfg = ctx.gizmos.config_mut::<PhysicsGizmos>();
            cfg.0.enabled = !cfg.0.enabled;
            let on = cfg.0.enabled;
            shell.say(format!("collider {}", on_off(on)));
        }
        Cmd::F4 => {
            let on = ctx.place.toggle();
            shell.say(format!("piazzamenti {}", on_off(on)));
        }
        Cmd::Debug(v) => {
            let on = apply_switch(&mut ctx.cheats.debug_keys, v);
            shell.say(format!("tasti F3 e F4 {}", on_off(on)));
        }
        Cmd::Coords(v) => {
            let on = apply_switch(&mut ctx.cheats.coords, v);
            shell.say(format!("coordinate {}", on_off(on)));
        }
        Cmd::Hour(hour) => {
            ctx.clock.hour = hour;
            shell.say(format!("ora {hour:.1}"));
        }
        Cmd::Weather(kind) => {
            ctx.weather.forced = kind;
            shell.say(match kind {
                Some(k) => format!("meteo forzato: {k:?}"),
                None => "meteo automatico".to_string(),
            });
        }
        Cmd::Ai(cmd) => run_ai(cmd, shell, ctx),
        Cmd::Tts(cmd) => run_tts(cmd, shell, ctx),
        Cmd::Stt(cmd) => run_stt(cmd, shell, ctx),
        Cmd::Stats => {
            for stat in crate::character::Stat::ALL {
                shell.say(format!(
                    "{}: {}",
                    stat.name(),
                    ctx.character.stats.get(stat)
                ));
            }
        }
        Cmd::SetStat(stat, value) => {
            ctx.character.stats.set(stat, value);
            shell.say(format!("{}: {value}", stat.name()));
        }
        Cmd::Skills => {
            let p = &ctx.progress;
            shell.say(format!(
                "livello {} ({:.0}%) · punti statistica {} · Nodi d'Oro {}",
                p.level,
                p.level_fraction() * 100.0,
                p.stat_points,
                p.nodes
            ));
            for skill in crate::skills::Skill::ALL {
                shell.say(format!(
                    "{}: grado {} · talento {}/5",
                    skill.key(),
                    p.grade(skill),
                    p.perk_rank(skill)
                ));
            }
        }
        Cmd::SetSkill(skill, grade) => {
            ctx.progress.set_grade(skill, grade);
            shell.say(format!("{}: grado {grade}", skill.name()));
        }
        Cmd::Level(None) => shell.say(format!("livello {}", ctx.progress.level)),
        Cmd::Level(Some(n)) => {
            ctx.progress.set_level(n);
            shell.say(format!(
                "livello {n} · punti statistica {} · Nodi d'Oro {}",
                ctx.progress.stat_points, ctx.progress.nodes
            ));
        }
        Cmd::Perk(skill) => {
            if ctx.progress.activate(skill) {
                shell.say(format!(
                    "{}: talento al rango {}",
                    skill.name(),
                    ctx.progress.perk_rank(skill)
                ));
            } else {
                shell.say("serve un Nodo d'Oro e il grado della soglia (15, 30, 50, 75, 100)");
            }
        }
        Cmd::Behind(name, behind) => {
            let found = ctx
                .npcs
                .iter()
                .max_by_key(|(npc, ..)| name_rank(npc.name, &name))
                .filter(|(npc, ..)| name_rank(npc.name, &name) > 0)
                .map(|(_, t, _)| {
                    let side = if behind { -1.1 } else { 1.3 };
                    t.translation() + t.rotation() * Vec3::Z * side
                });
            match (found, ctx.player.single_mut()) {
                (Some(dest), Ok((mut t, mut v))) => {
                    t.translation = dest + Vec3::Y * 0.4;
                    v.0 = Vec3::ZERO;
                    shell.say(format!(
                        "{} {name}",
                        if behind {
                            "alle spalle di"
                        } else {
                            "davanti a"
                        }
                    ));
                }
                _ => shell.say(format!("abitante sconosciuto: {name}")),
            }
        }
        Cmd::Karma(None) => {
            shell.say(format!(
                "karma {:+} — {}",
                ctx.karma.global,
                crate::karma::heart(ctx.karma.global)
            ));
            let lines: Vec<String> = ctx
                .karma
                .trust
                .iter()
                .map(|(name, t)| format!("{name}: {t} ({})", crate::karma::Bond::of(*t).name()))
                .collect();
            for line in lines {
                shell.say(line);
            }
        }
        Cmd::Karma(Some(n)) => {
            ctx.karma.global = n;
            shell.say(format!("karma {n:+}"));
        }
        Cmd::Trust(name, n) => {
            let found = ctx
                .npcs
                .iter()
                .max_by_key(|(npc, ..)| name_rank(npc.name, &name))
                .filter(|(npc, ..)| name_rank(npc.name, &name) > 0)
                .map(|(npc, ..)| npc.name.to_lowercase());
            match found {
                Some(key) => {
                    let n = n.clamp(crate::karma::TRUST_MIN, crate::karma::TRUST_MAX);
                    ctx.karma.trust.insert(key.clone(), n);
                    shell.say(format!(
                        "fiducia di {key}: {n} ({})",
                        crate::karma::Bond::of(n).name()
                    ));
                }
                None => shell.say(format!("abitante sconosciuto: {name}")),
            }
        }
        Cmd::Faces => {
            *ctx.journal_tab = crate::menu::JournalTab::Faces;
            shell.say("diario sulla scheda «Volti e segreti» (tasto J)");
        }
        Cmd::Cast(spell) => {
            ctx.magic.request = Some(spell);
            shell.say(format!("incantesimo: {}", spell.name()));
        }
        Cmd::SetMagic(n) => {
            ctx.vitals.magic = n.min(ctx.vitals.magic_max);
            shell.say(format!(
                "magia {:.0}/{:.0}",
                ctx.vitals.magic, ctx.vitals.magic_max
            ));
        }
        Cmd::AutoPick => {
            ctx.picking.auto = true;
            shell.say("la serratura si forza da sola");
        }
        Cmd::CarryEffect(carry_kg, remaining) => {
            ctx.effects.add(crate::character::Effect {
                source: "Magia di prova".to_string(),
                carry_kg,
                remaining,
            });
            shell.say(format!("portata {carry_kg:+.1} kg"));
        }
        Cmd::ClearEffects => {
            ctx.effects.0.clear();
            shell.say("effetti tolti");
        }
    }
}

fn run_ai(cmd: AiCmd, shell: &mut Shell, ctx: &mut ShellCtx) {
    match cmd {
        AiCmd::Status => shell.say(format!(
            "IA: {} · modello {}",
            ctx.ai.status.label(),
            ctx.audio.ai_model
        )),
        AiCmd::Switch(on) => {
            ctx.audio.ai_enabled = on;
            ctx.audio.dirty = true;
            ctx.ai.refresh();
            shell.say(format!("IA {}", on_off(on)));
        }
        AiCmd::Models => {
            if ctx.ai.models.is_empty() {
                shell.say("nessun modello: IA spenta o Ollama non raggiungibile");
            } else {
                shell.say(format!("modelli locali: {}", ctx.ai.models.join(", ")));
            }
        }
        AiCmd::Model(name) => {
            shell.say(format!("modello: {name}"));
            ctx.audio.ai_model = name;
            ctx.audio.dirty = true;
            ctx.ai.refresh();
        }
    }
}

fn run_tts(cmd: TtsCmd, shell: &mut Shell, ctx: &mut ShellCtx) {
    match cmd {
        TtsCmd::Status => shell.say(format!(
            "TTS: {} · voce {}",
            ctx.tts.status.label(),
            if ctx.audio.tts_voice.is_empty() {
                "automatica"
            } else {
                ctx.audio.tts_voice.as_str()
            }
        )),
        TtsCmd::Switch(on) => {
            ctx.audio.tts_enabled = on;
            ctx.audio.dirty = true;
            shell.say(format!("TTS {}", on_off(on)));
        }
        TtsCmd::Voices => {
            if ctx.tts.voices.is_empty() {
                shell.say("nessuna voce installata");
            } else {
                shell.say(format!("voci: {}", ctx.tts.voices.join(", ")));
            }
        }
        TtsCmd::Voice(name) => {
            if name.is_empty() || ctx.tts.voices.contains(&name) {
                ctx.audio.tts_voice = name;
                ctx.audio.dirty = true;
                shell.say("voce impostata");
            } else {
                shell.say(format!("voce sconosciuta: {name}"));
            }
        }
    }
}

fn run_stt(cmd: SttCmd, shell: &mut Shell, ctx: &mut ShellCtx) {
    match cmd {
        SttCmd::Status => shell.say(format!(
            "microfono: {} · modello {}",
            ctx.stt.status.label(),
            if ctx.audio.stt_model.is_empty() {
                "automatico"
            } else {
                ctx.audio.stt_model.as_str()
            }
        )),
        SttCmd::Switch(on) => {
            ctx.audio.stt_enabled = on;
            ctx.audio.dirty = true;
            shell.say(format!("microfono {}", on_off(on)));
        }
        SttCmd::Models => {
            if ctx.stt.models.is_empty() {
                shell.say("nessun modello: microfono spento o non installato");
            } else {
                shell.say(format!("modelli: {}", ctx.stt.models.join(", ")));
            }
        }
        SttCmd::Model(name) => {
            if ctx.stt.models.contains(&name) {
                shell.say(format!("modello: {name}"));
                ctx.audio.stt_model = name;
                ctx.audio.dirty = true;
            } else {
                shell.say(format!("modello sconosciuto: {name}"));
            }
        }
    }
}

/// Sistema: esegue i comandi in coda (dalla shell o da `PLANET_ONE_SHELL`).
pub fn shell_execute(
    mut shell: ResMut<Shell>,
    mut ctx: ShellCtx,
    mut next: ResMut<NextState<AppState>>,
    mut travel: ResMut<crate::save::TravelRequest>,
) {
    for line in std::mem::take(&mut shell.pending) {
        shell.say(format!("> {line}"));
        match parse(&line) {
            Ok(Cmd::Reset) => next.set(AppState::Resetting),
            Ok(Cmd::Region(region)) => {
                if region == crate::region::current() {
                    shell.say(format!("sei già qui: {}", region.title()));
                } else {
                    shell.say(format!("in viaggio verso {}", region.title()));
                    travel.0 = Some(region);
                }
            }
            Ok(cmd) => run(cmd, &mut shell, &mut ctx),
            Err(e) => shell.say(e),
        }
    }
}

/// Sistema: il tasto `\` (logico, vale su ogni layout) apre e chiude la shell.
pub fn shell_toggle(
    mut keys: MessageReader<KeyboardInput>,
    state: Res<State<AppState>>,
    dialog: Res<DialogState>,
    mut next: ResMut<NextState<AppState>>,
    mut shell: ResMut<Shell>,
) {
    let pressed = keys.read().any(|k| {
        k.state == ButtonState::Pressed
            && !k.repeat
            && matches!(&k.logical_key, Key::Character(c) if c.as_str() == "\\")
    });
    if !pressed || dialog.open {
        return;
    }
    match state.get() {
        AppState::Playing => {
            if shell.log.is_empty() {
                shell.say("Shell di Planet One: scrivi aiuto per l'elenco dei comandi.");
            }
            next.set(AppState::Shell);
        }
        AppState::Shell => next.set(AppState::Playing),
        _ => {}
    }
}

/// Sistema: niente collisioni per il giocatore in noclip.
pub fn apply_cheats(
    mut commands: Commands,
    cheats: Res<Cheats>,
    players: Query<(Entity, Has<ColliderDisabled>), With<Player>>,
) {
    for (e, disabled) in &players {
        if cheats.noclip && !disabled {
            commands.entity(e).insert(ColliderDisabled);
        } else if !cheats.noclip && disabled {
            commands.entity(e).remove::<ColliderDisabled>();
        }
    }
}

/// Sistema (egui): pannello della shell, con registro e riga di comando.
pub fn shell_panel(mut contexts: EguiContexts, mut shell: ResMut<Shell>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let (enter, up, down) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::ArrowDown),
        )
    });
    let size = ctx.viewport_rect().size();
    egui::Area::new(egui::Id::new("shell_panel"))
        .fixed_pos(egui::pos2(20.0, 20.0))
        .show(ctx, |ui| {
            let frame = egui::Frame::default()
                .fill(egui::Color32::from_rgba_unmultiplied(8, 10, 16, 245))
                .stroke(egui::Stroke::new(1.5, crate::ui_style::GOLD))
                .inner_margin(egui::Margin::same(14))
                .corner_radius(egui::CornerRadius::same(8));
            frame.show(ui, |ui| {
                ui.set_width((size.x - 70.0).min(900.0));
                egui::ScrollArea::vertical()
                    .max_height((size.y * 0.5).max(160.0))
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        for line in &shell.log {
                            ui.label(egui::RichText::new(line).monospace().size(15.0));
                        }
                    });
                ui.add_space(6.0);
                let edit = egui::TextEdit::singleline(&mut shell.input)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("comando (aiuto per l'elenco, \\ per chiudere)");
                ui.add(edit).request_focus();
            });
        });
    // Il tasto che apre o chiude la shell non deve finire nel testo.
    if shell.input.contains('\\') {
        shell.input = shell.input.replace('\\', "");
    }
    if enter {
        let line = shell.input.trim().to_string();
        shell.input.clear();
        shell.cursor = None;
        if !line.is_empty() {
            shell.history.push(line.clone());
            shell.pending.push(line);
        }
    } else if up || down {
        let n = shell.history.len();
        if n > 0 {
            let next = match (shell.cursor, up) {
                (None, true) => Some(n - 1),
                (None, false) => None,
                (Some(c), true) => Some(c.saturating_sub(1)),
                (Some(c), false) => (c + 1 < n).then_some(c + 1),
            };
            shell.cursor = next;
            shell.input = next.map_or_else(String::new, |c| shell.history[c].clone());
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Cheats>()
        .init_resource::<Shell>()
        .add_systems(Update, (shell_toggle, shell_execute, apply_cheats));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_accept_on_off_or_nothing() {
        assert_eq!(parse("god"), Ok(Cmd::God(None)));
        assert_eq!(parse("GOD on"), Ok(Cmd::God(Some(true))));
        assert_eq!(parse("noclip off"), Ok(Cmd::Noclip(Some(false))));
        assert!(parse("invisibile forse").is_err());
    }

    #[test]
    fn teleport_takes_coordinates_or_a_name() {
        assert_eq!(
            parse("tp 1 2,5 -3"),
            Ok(Cmd::Teleport(Target::Pos(Vec3::new(1.0, 2.5, -3.0))))
        );
        assert_eq!(
            parse("tp Porta Nord"),
            Ok(Cmd::Teleport(Target::Place("porta nord".into(), 0)))
        );
        assert_eq!(
            parse("tp casa-media 1"),
            Ok(Cmd::Teleport(Target::Place("casa-media".into(), 1)))
        );
        assert!(parse("tp").is_err());
        assert_eq!(
            parse("quest torna"),
            Ok(Cmd::Quest(
                crate::quests::denaro_rubato::QuestStage::Returning
            ))
        );
        assert!(parse("quest boh").is_err());
        assert_eq!(parse("uccidi"), Ok(Cmd::Kill(30.0)));
        assert_eq!(parse("vicino 8"), Ok(Cmd::Near(8.0)));
        assert_eq!(
            parse("vai 3 4"),
            Ok(Cmd::Go(Target::Pos(Vec3::new(3.0, 0.0, 4.0)), false))
        );
        assert_eq!(
            parse("corri chiesa"),
            Ok(Cmd::Go(Target::Place("chiesa".into(), 0), true))
        );
        assert_eq!(
            parse("guarda 90 10"),
            Ok(Cmd::Look(LookTarget::Angles(90.0, 10.0)))
        );
    }

    #[test]
    fn master_key_and_items() {
        assert_eq!(parse("passepartout"), Ok(Cmd::Master(true)));
        assert_eq!(parse("passepartout off"), Ok(Cmd::Master(false)));
        assert_eq!(parse("dai mela 3"), Ok(Cmd::Give("mela".into(), 3)));
        assert_eq!(
            parse("dai spada corta"),
            Ok(Cmd::Give("spada corta".into(), 1))
        );
        assert!(parse("dai").is_err());
    }

    #[test]
    fn hour_weather_and_voices() {
        assert_eq!(parse("ora 22,5"), Ok(Cmd::Hour(22.5)));
        assert!(parse("ora 30").is_err());
        assert_eq!(parse("meteo auto"), Ok(Cmd::Weather(None)));
        assert!(parse("meteo grandine").is_err());
        assert_eq!(
            parse("ia modello Gemma:E4B"),
            Ok(Cmd::Ai(AiCmd::Model("Gemma:E4B".into())))
        );
        assert_eq!(parse("ia off"), Ok(Cmd::Ai(AiCmd::Switch(false))));
        assert_eq!(
            parse("tts voce auto"),
            Ok(Cmd::Tts(TtsCmd::Voice(String::new())))
        );
    }

    #[test]
    fn stt_commands() {
        assert_eq!(parse("stt"), Ok(Cmd::Stt(SttCmd::Status)));
        assert_eq!(parse("stt on"), Ok(Cmd::Stt(SttCmd::Switch(true))));
        assert_eq!(
            parse("stt modello ggml-base"),
            Ok(Cmd::Stt(SttCmd::Model("ggml-base".into())))
        );
    }

    #[test]
    fn stat_and_effect_commands() {
        use crate::character::Stat;
        assert_eq!(parse("stat"), Ok(Cmd::Stats));
        assert_eq!(parse("stat forza 15"), Ok(Cmd::SetStat(Stat::Strength, 15)));
        assert!(parse("stat forza 40").is_err());
        use crate::skills::Skill;
        assert_eq!(parse("abilita"), Ok(Cmd::Skills));
        assert_eq!(
            parse("abilita arco 30"),
            Ok(Cmd::SetSkill(Skill::Archery, 30))
        );
        assert!(parse("abilita arco 300").is_err());
        assert_eq!(parse("livello 5"), Ok(Cmd::Level(Some(5))));
        assert_eq!(parse("talento furtivita"), Ok(Cmd::Perk(Skill::Stealth)));
        assert_eq!(
            parse("effetto peso 20 60"),
            Ok(Cmd::CarryEffect(20.0, Some(60.0)))
        );
        assert_eq!(parse("effetto peso 5"), Ok(Cmd::CarryEffect(5.0, None)));
        assert_eq!(parse("effetto via"), Ok(Cmd::ClearEffects));
    }

    #[test]
    fn unknown_commands_are_rejected() {
        assert!(parse("").is_err());
        assert!(parse("sudo rm").is_err());
    }
}
