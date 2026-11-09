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
    "aiuto · stato · pulisci",
    "god [on|off] · noclip [on|off] · invisibile [on|off]",
    "tp <x> <y> <z> · tp <luogo o abitante> [n] (es. tp chiesa, tp porta nord, tp marta)",
    "passepartout [off] · chiavi · dai <oggetto> [quantità]",
    "f3 (collider) · f4 (piazzamenti) · debug [on|off] (tasti F3 e F4) · coordinate [on|off]",
    "ora <0-24> · meteo <sereno|nuvoloso|pioggia|temporale|nebbia|neve|auto>",
    "ia [on|off] · ia modelli · ia modello <nome>",
    "tts [on|off] · tts voci · tts voce <nome|auto>",
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

/// Comando riconosciuto. `None` nei trucchi vuol dire "inverti".
#[derive(Debug, PartialEq, Clone)]
pub enum Cmd {
    Help,
    Status,
    Clear,
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

/// Tutto ciò che i comandi possono toccare.
#[derive(SystemParam)]
pub struct ShellCtx<'w, 's> {
    cheats: ResMut<'w, Cheats>,
    inventory: ResMut<'w, Inventory>,
    clock: ResMut<'w, crate::daynight::GameClock>,
    weather: ResMut<'w, crate::weather::WeatherState>,
    audio: ResMut<'w, crate::audio::AudioSettings>,
    ai: ResMut<'w, crate::ai::AiState>,
    tts: Res<'w, crate::ai::tts::TtsState>,
    place: ResMut<'w, crate::debug_place::DebugPlace>,
    gizmos: ResMut<'w, GizmoConfigStore>,
    homes: Res<'w, crate::world_props::Homes>,
    village: Res<'w, crate::village::VillageInfo>,
    field: Option<Res<'w, crate::terrain::TerrainField>>,
    player: Query<'w, 's, (&'static mut Transform, &'static mut LinearVelocity), With<Player>>,
    npcs: Query<
        'w,
        's,
        (
            &'static Npc,
            &'static Transform,
            Option<&'static NpcInventory>,
        ),
        Without<Player>,
    >,
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
        .find(|(npc, ..)| npc.name.to_lowercase().contains(name))
    {
        return Some(t.translation + Vec3::new(1.2, 0.4, 0.0));
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

/// Sistema: esegue i comandi in coda (dalla shell o da `PLANET_ONE_SHELL`).
pub fn shell_execute(mut shell: ResMut<Shell>, mut ctx: ShellCtx) {
    for line in std::mem::take(&mut shell.pending) {
        shell.say(format!("> {line}"));
        match parse(&line) {
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
    fn unknown_commands_are_rejected() {
        assert!(parse("").is_err());
        assert!(parse("sudo rm").is_err());
    }
}
