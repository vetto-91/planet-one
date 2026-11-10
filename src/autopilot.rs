//! Pilota automatico per il beta test: esegue un copione di comandi (`PLANET_ONE_SCRIPT=file`)
//! senza tastiera né mouse, fotografa lo stato del gioco in JSON e controlla condizioni.
//!
//! Ogni riga del copione è un comando; le righe vuote e quelle che iniziano con `#` si saltano.
//! Comandi propri del pilota:
//! - `aspetta <s>`: attende `s` secondi di gioco;
//! - `premi <tasto> [s]`: tiene premuto un tasto (`w a s d spazio shift ctrl e invio esc tab su giu
//!   sinistra destra 0-9 i j m t c v f5 f9 click clickdx`) per `s` secondi (predefinito 0,1);
//! - `avanza`: fa scorrere il dialogo (Invio) finché compaiono le scelte o si chiude;
//! - `scegli <n>`: nel dialogo sceglie la risposta `n` (da 1) e conferma;
//! - `foto <nome>`: screenshot del gioco in `<uscita>/<nome>.png`;
//! - `dump [nome]`: stato del gioco in JSON in `<uscita>/stato-<nome>.json`;
//! - `verifica <campo> <op> <valore>`: confronto sullo stato (`=`, `!=`, `<`, `<=`, `>`, `>=`,
//!   `contiene`); i campi sono in [`field`]. Un fallimento scrive `[VERIFICA] FALLITA` nel log;
//! - `attendi <campo> <op> <valore> [s]`: aspetta che la condizione sia vera (massimo `s`, 30 s);
//! - `mai <campo> <op> <valore> [s]`: la condizione non deve mai essere vera per `s` secondi;
//! - `esci`: scrive il rapporto e chiude il gioco (codice d'uscita 1 se qualcosa è fallito).
//!
//! Ogni altra riga va alla shell dei trucchi (`tp`, `ora`, `meteo`, `dai`, `god`, `vai`, `guarda`…):
//! il pilota aspetta che `vai` arrivi a destinazione prima di proseguire.
//!
//! Uscita: `PLANET_ONE_OUT` oppure `prove/out/<nome del copione>/`.

use crate::camera::OrbitCamera;
use crate::player::{Player, PlayerHealth};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::path::PathBuf;

/// Arrivo di `vai`: distanza a cui si considera raggiunto (m).
const ARRIVE: f32 = 1.0;
/// Se in questo tempo (s) non ci si avvicina di almeno [`PROGRESS`] m, si è bloccati.
const STUCK_AFTER: f32 = 3.0;
const PROGRESS: f32 = 0.3;

/// Tasto o pulsante premuto dal pilota.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PilotKey {
    Key(KeyCode),
    Mouse(MouseButton),
}

/// Meta di `vai`: punto sul piano e controllo dei progressi.
#[derive(Debug)]
pub struct Goal {
    pub target: Vec2,
    pub label: String,
    pub sprint: bool,
    /// Distanza di arrivo (m).
    pub arrive: f32,
    /// Tempo massimo (s).
    pub timeout: f32,
    pub elapsed: f32,
    best: f32,
    since_best: f32,
}

impl Goal {
    pub fn new(target: Vec2, label: String, from: Vec2, sprint: bool) -> Self {
        let dist = from.distance(target);
        Self {
            target,
            label,
            sprint,
            arrive: ARRIVE,
            timeout: 15.0 + dist / 2.0,
            elapsed: 0.0,
            best: dist,
            since_best: 0.0,
        }
    }
}

/// Stato del pilota e richieste dalla shell (`foto`, `dump`).
#[derive(Resource, Default)]
pub struct Autopilot {
    lines: VecDeque<(usize, String)>,
    pub active: bool,
    started: bool,
    frames: u32,
    wait: f32,
    held: Vec<(PilotKey, f32)>,
    pub goal: Option<Goal>,
    /// `guarda <nome>`: punto da mettere al centro del mirino e fotogrammi già usati.
    pub aim: Option<(Vec3, u32)>,
    pub out: PathBuf,
    /// Richiesta di fotografia dello stato (percorso del file, `None` = solo in memoria).
    pub dump: Option<Option<PathBuf>>,
    /// Richiesta di screenshot.
    pub photo: Option<PathBuf>,
    /// Ultima fotografia dello stato.
    pub last: Option<Value>,
    /// Verifica in attesa della fotografia aggiornata.
    check: Option<(usize, String)>,
    /// `attendi`: condizione da aspettare, tempo rimasto, prossima prova.
    /// `mai` (ultimo campo `true`): la condizione non deve diventare vera nel tempo dato.
    until: Option<(usize, String, f32, f32, bool)>,
    /// Scelta del dialogo da impostare prima della conferma.
    choice: Option<usize>,
    /// `avanza`: righe del dialogo ancora da far scorrere (fino alle scelte o alla chiusura).
    advance: u32,
    pub failures: Vec<String>,
    pub passed: u32,
    script: String,
}

impl Autopilot {
    /// Carica un copione (righe numerate, senza commenti).
    pub fn load(&mut self, name: &str, text: &str) {
        self.lines = text
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.trim().to_string()))
            .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
            .collect();
        self.script = name.to_string();
        self.active = true;
        self.started = false;
        self.frames = 0;
        self.failures.clear();
        self.passed = 0;
        if self.out.as_os_str().is_empty() {
            self.out = std::env::var_os("PLANET_ONE_OUT")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    let stem = std::path::Path::new(name)
                        .file_stem()
                        .map_or("copione".into(), |s| s.to_string_lossy().to_string());
                    PathBuf::from("prove/out").join(stem)
                });
        }
        let _ = std::fs::create_dir_all(&self.out);
        info!(
            "[PILOTA] copione {name}: {} comandi, uscita {}",
            self.lines.len(),
            self.out.display()
        );
    }

    fn fail(&mut self, line: usize, msg: String) {
        error!("[VERIFICA] FALLITA (riga {line}): {msg}");
        self.failures.push(format!("riga {line}: {msg}"));
    }

    /// Rapporto finale in `<uscita>/RAPPORTO.txt`.
    fn report(&self) -> String {
        let mut s = format!(
            "Copione: {}\nVerifiche passate: {}\nFallimenti: {}\n",
            self.script,
            self.passed,
            self.failures.len()
        );
        for f in &self.failures {
            s.push_str(&format!("- {f}\n"));
        }
        s
    }
}

/// Nome di un tasto del copione.
pub fn parse_key(name: &str) -> Option<PilotKey> {
    use KeyCode as K;
    let k = match name.to_lowercase().as_str() {
        "w" => K::KeyW,
        "a" => K::KeyA,
        "s" => K::KeyS,
        "d" => K::KeyD,
        "e" => K::KeyE,
        "r" => K::KeyR,
        "i" => K::KeyI,
        "j" => K::KeyJ,
        "m" => K::KeyM,
        "t" => K::KeyT,
        "c" => K::KeyC,
        "v" => K::KeyV,
        "x" => K::KeyX,
        "spazio" | "space" => K::Space,
        "shift" => K::ShiftLeft,
        "ctrl" => K::ControlLeft,
        "invio" | "enter" => K::Enter,
        "esc" => K::Escape,
        "tab" => K::Tab,
        "su" | "up" => K::ArrowUp,
        "giu" | "giù" | "down" => K::ArrowDown,
        "sinistra" | "left" => K::ArrowLeft,
        "destra" | "right" => K::ArrowRight,
        "f3" => K::F3,
        "f4" => K::F4,
        "f5" => K::F5,
        "f9" => K::F9,
        "f11" => K::F11,
        "f12" => K::F12,
        "1" => K::Digit1,
        "2" => K::Digit2,
        "3" => K::Digit3,
        "4" => K::Digit4,
        "5" => K::Digit5,
        "6" => K::Digit6,
        "7" => K::Digit7,
        "8" => K::Digit8,
        "9" => K::Digit9,
        "0" => K::Digit0,
        "click" | "clic" => return Some(PilotKey::Mouse(MouseButton::Left)),
        "clickdx" | "clicdx" => return Some(PilotKey::Mouse(MouseButton::Right)),
        _ => return None,
    };
    Some(PilotKey::Key(k))
}

/// Campo dello stato JSON per `verifica`: nome breve oppure puntatore JSON (`/giocatore/vita`).
pub fn field<'a>(state: &'a Value, name: &str) -> Option<&'a Value> {
    let pointer = match name {
        "vita" => "/giocatore/vita",
        "x" => "/giocatore/pos/0",
        "y" => "/giocatore/pos/1",
        "z" => "/giocatore/pos/2",
        "sopra_terreno" => "/giocatore/sopra_terreno",
        "anim" => "/giocatore/anim",
        "quest" => "/quest/fase",
        "monete_quest" => "/quest/monete",
        "monete" => "/inventario/Coin",
        "dialogo" => "/dialogo/aperto",
        "parla" => "/dialogo/chi",
        "app" => "/app",
        "ora" => "/ora",
        "meteo" => "/meteo",
        "fps" => "/fps",
        "vicini" => "/vicini",
        "arma" => "/arma",
        p if p.starts_with('/') => p,
        // `npc:<nome>:<campo>`: il primo abitante vicino il cui nome contiene `<nome>`.
        p if p.starts_with("npc:") => {
            let mut parts = p[4..].splitn(2, ':');
            let who = parts.next()?.to_lowercase();
            let what = parts.next().unwrap_or("dist");
            return state
                .pointer("/vicini")?
                .as_array()?
                .iter()
                .find(|v| {
                    v.get("nome")
                        .and_then(Value::as_str)
                        .is_some_and(|n| n.to_lowercase().contains(&who))
                })?
                .get(what);
        }
        p => return state.pointer(&format!("/inventario/{p}")),
    };
    state.pointer(pointer)
}

/// Valuta `campo op valore` sullo stato.
pub fn evaluate(state: &Value, expr: &str) -> Result<(), String> {
    let words: Vec<&str> = expr.split_whitespace().collect();
    let [name, op, rest @ ..] = words.as_slice() else {
        return Err(format!("verifica malformata: «{expr}»"));
    };
    let wanted = rest.join(" ");
    let got = field(state, name).cloned().unwrap_or(Value::Null);
    let as_text = |v: &Value| match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let ok = match *op {
        "contiene" | "contains" => as_text(&got)
            .to_lowercase()
            .contains(&wanted.to_lowercase()),
        _ => {
            let num = (got.as_f64(), wanted.replace(',', ".").parse::<f64>().ok());
            match (num, *op) {
                ((Some(a), Some(b)), "=" | "==") => (a - b).abs() < 1e-3,
                ((Some(a), Some(b)), "!=") => (a - b).abs() >= 1e-3,
                ((Some(a), Some(b)), "<") => a < b,
                ((Some(a), Some(b)), "<=") => a <= b,
                ((Some(a), Some(b)), ">") => a > b,
                ((Some(a), Some(b)), ">=") => a >= b,
                (_, "=" | "==") => as_text(&got).eq_ignore_ascii_case(&wanted),
                (_, "!=") => !as_text(&got).eq_ignore_ascii_case(&wanted),
                _ => {
                    return Err(format!(
                        "confronto impossibile: {name} = {got} {op} {wanted}"
                    ));
                }
            }
        }
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{name} {op} {wanted}: valore {got}"))
    }
}

/// Sistema (PreUpdate, dopo l'input): tasti tenuti, meta di `vai` e prossimo comando.
#[allow(clippy::too_many_arguments)]
pub fn run_autopilot(
    mut pilot: ResMut<Autopilot>,
    time: Res<Time>,
    state: Res<State<crate::AppState>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut orbit: ResMut<OrbitCamera>,
    mut shell: ResMut<crate::shell::Shell>,
    mut dialog: ResMut<crate::entities::DialogState>,
    players: Query<&Transform, With<Player>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !pilot.active {
        return;
    }
    let dt = time.delta_secs();
    // Il copione parte quando la partita è in corso da un po' (scena e abitanti pronti).
    if !pilot.started {
        if *state.get() == crate::AppState::Playing {
            pilot.frames += 1;
        }
        if pilot.frames < 120 {
            return;
        }
        pilot.started = true;
        info!("[PILOTA] inizio");
    }
    // Tasti tenuti.
    let mut held = std::mem::take(&mut pilot.held);
    held.retain_mut(|(k, left)| {
        *left -= dt;
        let keep = *left > 0.0;
        match (*k, keep) {
            (PilotKey::Key(c), true) => keys.press(c),
            (PilotKey::Key(c), false) => keys.release(c),
            (PilotKey::Mouse(b), true) => mouse.press(b),
            (PilotKey::Mouse(b), false) => mouse.release(b),
        }
        keep
    });
    pilot.held = held;
    // Meta di `vai`: si orienta la camera verso la meta e si tiene premuto W.
    if let Some(mut goal) = pilot.goal.take() {
        let Ok(t) = players.single() else {
            pilot.goal = Some(goal);
            return;
        };
        let here = t.translation.xz();
        let to = goal.target - here;
        let dist = to.length();
        goal.elapsed += dt;
        if dist < goal.best - PROGRESS {
            goal.best = dist;
            goal.since_best = 0.0;
        } else {
            goal.since_best += dt;
        }
        let release = |keys: &mut ButtonInput<KeyCode>| {
            keys.release(KeyCode::KeyW);
            keys.release(KeyCode::ShiftLeft);
        };
        if dist < goal.arrive {
            release(&mut keys);
            info!(
                "[PILOTA] arrivato a {} in {:.1} s",
                goal.label, goal.elapsed
            );
        } else if goal.since_best > STUCK_AFTER || goal.elapsed > goal.timeout {
            release(&mut keys);
            let msg = format!(
                "vai {}: bloccato a ({:.1}, {:.1}, {:.1}), a {dist:.1} m dalla meta",
                goal.label, t.translation.x, t.translation.y, t.translation.z
            );
            warn!("[PILOTA] {msg}");
            pilot.failures.push(msg);
        } else {
            let dir = to / dist;
            // La camera guarda il giocatore da dietro: in avanti = -(sin yaw, cos yaw).
            orbit.yaw = (-dir.x).atan2(-dir.y);
            keys.press(KeyCode::KeyW);
            if goal.sprint {
                keys.press(KeyCode::ShiftLeft);
            }
            pilot.goal = Some(goal);
        }
        return;
    }
    // Mira: la camera gira attorno al giocatore, quindi si corregge per qualche fotogramma
    // finché il centro dello schermo passa sul punto.
    if let Some((target, frames)) = pilot.aim.take()
        && let Ok(cam) = cameras.single()
    {
        let to = (target - cam.translation()).normalize_or_zero();
        let off = (to - *cam.forward()).length();
        if off < 0.03 || frames > 90 {
            info!("[PILOTA] mira: scarto {off:.3} dopo {frames} fotogrammi");
        } else {
            orbit.yaw = (-to.x).atan2(-to.z);
            orbit.pitch = (-to.y).asin().clamp(-1.2, 1.4);
            pilot.aim = Some((target, frames + 1));
            return;
        }
    }
    // Verifica in attesa della fotografia aggiornata.
    if let Some((line, expr)) = pilot.check.take() {
        if pilot.dump.is_some() {
            pilot.check = Some((line, expr));
            return;
        }
        match pilot.last.as_ref().map(|s| evaluate(s, &expr)) {
            Some(Ok(())) => {
                pilot.passed += 1;
                info!("[VERIFICA] ok (riga {line}): {expr}");
            }
            Some(Err(e)) => pilot.fail(line, e),
            None => pilot.fail(line, "nessuno stato".into()),
        }
    }
    // `attendi`: si riprova la condizione ogni mezzo secondo fino al tempo massimo.
    if let Some((line, expr, left, next, never)) = pilot.until.take() {
        let (left, next) = (left - dt, next - dt);
        if pilot.dump.is_some() {
            pilot.until = Some((line, expr, left, next, never));
            return;
        }
        let fresh = next < 0.45;
        let holds = fresh
            && pilot
                .last
                .as_ref()
                .is_some_and(|s| evaluate(s, &expr).is_ok());
        let word = if never { "mai" } else { "attendi" };
        if holds && !never {
            pilot.passed += 1;
            info!("[VERIFICA] ok (riga {line}): attendi {expr}");
        } else if holds && never {
            let got = pilot.last.clone().unwrap_or_default();
            let name = expr.split_whitespace().next().unwrap_or("");
            let value = field(&got, name).cloned().unwrap_or(Value::Null);
            pilot.fail(line, format!("mai {expr}: successo (valore {value})"));
        } else if left <= 0.0 {
            if never {
                pilot.passed += 1;
                info!("[VERIFICA] ok (riga {line}): mai {expr}");
            } else {
                pilot.fail(line, format!("{word} {expr}: tempo scaduto"));
            }
        } else {
            if next <= 0.0 {
                pilot.dump = Some(None);
            }
            pilot.until = Some((
                line,
                expr,
                left,
                if next <= 0.0 { 0.5 } else { next },
                never,
            ));
            return;
        }
    }
    // Scelta del dialogo: si imposta e si conferma con Invio.
    if let Some(n) = pilot.choice.take() {
        if dialog.open && dialog.choosing && n < dialog.choices.len() {
            dialog.selected = n;
            keys.press(KeyCode::Enter);
            pilot.held.push((PilotKey::Key(KeyCode::Enter), 0.05));
        } else {
            let line = 0;
            pilot.fail(line, format!("scegli {}: nessuna scelta aperta", n + 1));
        }
    }
    if pilot.wait > 0.0 {
        pilot.wait -= dt;
        return;
    }
    // `avanza`: Invio finché compaiono le scelte o il dialogo si chiude.
    if pilot.advance > 0 {
        if dialog.open && !dialog.choosing {
            pilot.advance -= 1;
            keys.press(KeyCode::Enter);
            pilot.held.push((PilotKey::Key(KeyCode::Enter), 0.05));
            pilot.wait = 0.35;
            return;
        }
        pilot.advance = 0;
    }
    // La shell ha ancora comandi da eseguire, oppure un tasto è ancora tenuto.
    if !shell.pending.is_empty() || !pilot.held.is_empty() || pilot.photo.is_some() {
        return;
    }
    let Some((n, line)) = pilot.lines.pop_front() else {
        return;
    };
    info!("[PILOTA] > {line}");
    let words: Vec<&str> = line.split_whitespace().collect();
    let num = |i: usize| -> Option<f32> { words.get(i)?.replace(',', ".").parse().ok() };
    match words[0].to_lowercase().as_str() {
        "aspetta" | "wait" => pilot.wait = num(1).unwrap_or(1.0),
        "premi" | "press" => match words.get(1).and_then(|k| parse_key(k)) {
            Some(k) => {
                match k {
                    PilotKey::Key(c) => keys.press(c),
                    PilotKey::Mouse(b) => mouse.press(b),
                }
                pilot.held.push((k, num(2).unwrap_or(0.1)));
            }
            None => pilot.fail(n, format!("tasto sconosciuto in «{line}»")),
        },
        "scegli" | "choose" => match num(1) {
            Some(v) if v >= 1.0 => pilot.choice = Some(v as usize - 1),
            _ => pilot.fail(n, format!("uso: scegli <n> («{line}»)")),
        },
        "avanza" | "next" => pilot.advance = 30,
        "foto" | "shot" => {
            let name = words.get(1).copied().unwrap_or("foto");
            pilot.photo = Some(pilot.out.join(format!("{name}.png")));
        }
        "dump" => {
            let name = words.get(1).copied().unwrap_or("stato");
            pilot.dump = Some(Some(pilot.out.join(format!("stato-{name}.json"))));
        }
        "attendi" | "until" | "mai" | "never" => {
            let never = matches!(words[0].to_lowercase().as_str(), "mai" | "never");
            // L'ultima parola, se è un numero dopo il valore, è il tempo massimo.
            let mut expr: Vec<&str> = words[1..].to_vec();
            let max = if expr.len() > 3 {
                expr.last().and_then(|w| w.parse::<f32>().ok())
            } else {
                None
            };
            if max.is_some() {
                expr.pop();
            }
            pilot.dump = Some(None);
            pilot.until = Some((n, expr.join(" "), max.unwrap_or(30.0), 0.5, never));
        }
        "verifica" | "check" => {
            pilot.dump = Some(None);
            pilot.check = Some((n, words[1..].join(" ")));
        }
        "esci" | "quit" => {
            let report = pilot.report();
            let _ = std::fs::write(pilot.out.join("RAPPORTO.txt"), &report);
            for l in report.lines() {
                info!("[PILOTA] {l}");
            }
            pilot.active = false;
            exit.write(if pilot.failures.is_empty() {
                AppExit::Success
            } else {
                AppExit::error()
            });
        }
        _ => shell.pending.push(line),
    }
}

/// Sistema: screenshot richiesto dal pilota o dalla shell.
pub fn take_photos(mut commands: Commands, mut pilot: ResMut<Autopilot>) {
    if let Some(path) = pilot.photo.take() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        info!("[PILOTA] foto {}", path.display());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

/// Fotografia dello stato del gioco (JSON): giocatore, quest, dialogo, inventario, dintorni.
pub fn snapshot(world: &mut World) -> Value {
    let app = world
        .get_resource::<State<crate::AppState>>()
        .map(|s| format!("{:?}", s.get()));
    let clock = world.resource::<crate::daynight::GameClock>();
    let (hour, date) = (clock.hour, clock.date.label());
    let weather = world.resource::<crate::weather::WeatherState>();
    let (meteo, temp) = (format!("{:?}", weather.kind), weather.temp);
    let fps = world
        .get_resource::<DiagnosticsStore>()
        .and_then(|d| d.get(&FrameTimeDiagnosticsPlugin::FPS))
        .and_then(|d| d.smoothed());
    let ground = |world: &World, p: Vec3| {
        world
            .get_resource::<crate::terrain::TerrainField>()
            .map(|f| f.height_at(p.x, p.z))
    };
    let mut q = world.query_filtered::<(&Transform, &PlayerHealth), With<Player>>();
    let player = q
        .iter(world)
        .next()
        .map(|(t, hp)| (t.translation, hp.current, hp.max));
    let pos = player.map_or(Vec3::ZERO, |p| p.0);
    let mut anims =
        world.query_filtered::<&crate::animation::Animator, With<crate::player::PlayerVisual>>();
    let anim = anims.iter(world).next().map(|a| a.current_clip());
    let quest = world.resource::<crate::quests::denaro_rubato::QuestState>();
    let quest = json!({
        "fase": format!("{:?}", quest.stage),
        "monete": quest.coins_collected,
        "accettata": quest.accepted,
    });
    let inv = world.resource::<crate::inventory::Inventory>();
    let mut items = serde_json::Map::new();
    for s in &inv.slots {
        items.insert(format!("{:?}", s.kind), json!(s.qty));
    }
    let equip = world.resource::<crate::inventory::Equipment>();
    let arma = equip.weapon.map(|w| format!("{w:?}"));
    let d = world.resource::<crate::entities::DialogState>();
    let dialog = json!({
        "aperto": d.open,
        "chi": d.speaker,
        "riga": d.lines.get(d.index),
        "indice": d.index,
        "righe": d.lines.len(),
        "scelte": d.choices,
        "scelta": d.selected,
        "scegliendo": d.choosing,
    });
    // Dintorni (25 m): personaggi, nemici e oggetti con stato.
    let mut near: Vec<(f32, Value)> = Vec::new();
    let mut npcs = world.query::<(
        &GlobalTransform,
        &crate::entities::Npc,
        Option<&crate::villagers::Villager>,
        Option<&crate::animation::Animator>,
    )>();
    for (g, npc, v, a) in npcs.iter(world) {
        let p = g.translation();
        let dist = p.distance(pos);
        if dist < 25.0 {
            near.push((
                dist,
                json!({
                    "tipo": "npc",
                    "nome": npc.name,
                    "dist": (dist * 10.0).round() / 10.0,
                    "pos": [p.x, p.y, p.z],
                    "modo": format!("{:?}", npc.mode),
                    "fase": v.map(|v| v.phase_name()),
                    "anim": a.map(|a| a.current_clip()),
                }),
            ));
        }
    }
    let mut enemies = world.query::<(&GlobalTransform, &crate::combat::EnemyHealth)>();
    for (g, hp) in enemies.iter(world) {
        let p = g.translation();
        let dist = p.distance(pos);
        if dist < 40.0 {
            near.push((
                dist,
                json!({
                    "tipo": "nemico",
                    "dist": (dist * 10.0).round() / 10.0,
                    "pos": [p.x, p.y, p.z],
                    "vita": hp.current,
                }),
            ));
        }
    }
    let mut props = world.query::<(
        &GlobalTransform,
        &crate::hut::PropKind,
        &crate::hut::PropState,
        Option<&crate::hut::PropId>,
    )>();
    for (g, kind, st, id) in props.iter(world) {
        let p = g.translation();
        let dist = p.distance(pos);
        if dist < 8.0 {
            near.push((
                dist,
                json!({
                    "tipo": format!("{kind:?}"),
                    "id": id.map(|i| i.0.clone()),
                    "dist": (dist * 10.0).round() / 10.0,
                    "attivo": st.on,
                }),
            ));
        }
    }
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    let terrain = ground(world, pos);
    json!({
        "app": app,
        "ora": (hour * 100.0).round() / 100.0,
        "data": date,
        "meteo": meteo,
        "temperatura": temp,
        "fps": fps.map(|f| f.round()),
        "entita": world.entities().len(),
        "giocatore": {
            "pos": [pos.x, pos.y, pos.z],
            "vita": player.map(|p| p.1),
            "vita_max": player.map(|p| p.2),
            "terreno": terrain,
            "sopra_terreno": terrain.map(|h| pos.y - h),
            "anim": anim,
        },
        "arma": arma,
        "quest": quest,
        "inventario": items,
        "dialogo": dialog,
        "vicini": near.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
    })
}

/// Sistema esclusivo: esegue la fotografia dello stato richiesta.
pub fn dump_state(world: &mut World) {
    let Some(request) = world.resource_mut::<Autopilot>().dump.take() else {
        return;
    };
    let state = snapshot(world);
    if let Some(path) = request {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match serde_json::to_string_pretty(&state) {
            Ok(text) => {
                let _ = std::fs::write(&path, text);
                info!("[PILOTA] stato in {}", path.display());
            }
            Err(e) => warn!("[PILOTA] stato non scritto: {e}"),
        }
    }
    world.resource_mut::<Autopilot>().last = Some(state);
}

/// Avvio: carica il copione di `PLANET_ONE_SCRIPT`, la velocità del tempo `PLANET_ONE_SPEED`.
pub fn start_autopilot(mut pilot: ResMut<Autopilot>, mut time: ResMut<Time<Virtual>>) {
    if let Some(speed) = std::env::var("PLANET_ONE_SPEED")
        .ok()
        .and_then(|s| s.replace(',', ".").parse::<f32>().ok())
        .filter(|s| *s > 0.0)
    {
        time.set_relative_speed(speed);
        info!("[PILOTA] tempo accelerato ×{speed}");
    }
    let Ok(path) = std::env::var("PLANET_ONE_SCRIPT") else {
        return;
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => pilot.load(&path, &text),
        Err(e) => error!("[PILOTA] copione {path} non letto: {e}"),
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Autopilot>()
        .add_systems(Startup, start_autopilot)
        .add_systems(PreUpdate, run_autopilot.after(bevy::input::InputSystems))
        .add_systems(Update, (take_photos, dump_state));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> Value {
        json!({
            "app": "Playing",
            "giocatore": {"pos": [1.0, 2.0, 3.0], "vita": 80.0, "sopra_terreno": 0.9},
            "quest": {"fase": "SeekThief", "monete": 0},
            "inventario": {"Coin": 30, "Apple": 2},
            "dialogo": {"aperto": false, "chi": "Viandante"},
        })
    }

    #[test]
    fn checks_compare_numbers_and_text() {
        let s = state();
        assert!(evaluate(&s, "vita > 50").is_ok());
        assert!(evaluate(&s, "vita <= 50").is_err());
        assert!(evaluate(&s, "quest = seekthief").is_ok());
        assert!(evaluate(&s, "monete >= 30").is_ok());
        assert!(evaluate(&s, "Apple = 2").is_ok());
        assert!(evaluate(&s, "dialogo = false").is_ok());
        assert!(evaluate(&s, "parla contiene viand").is_ok());
        assert!(evaluate(&s, "/giocatore/pos/1 < 3").is_ok());
        assert!(evaluate(&s, "vita").is_err());
        let s = json!({"vicini": [{"nome": "Viandante", "modo": "Ready", "dist": 2.0}]});
        assert!(evaluate(&s, "npc:viandante:modo = ready").is_ok());
        assert!(evaluate(&s, "npc:viandante < 3").is_ok());
    }

    #[test]
    fn keys_have_names() {
        assert_eq!(parse_key("W"), Some(PilotKey::Key(KeyCode::KeyW)));
        assert_eq!(parse_key("invio"), Some(PilotKey::Key(KeyCode::Enter)));
        assert_eq!(parse_key("click"), Some(PilotKey::Mouse(MouseButton::Left)));
        assert_eq!(parse_key("boh"), None);
    }

    #[test]
    fn scripts_skip_comments_and_blanks() {
        let mut p = Autopilot {
            out: std::env::temp_dir().join("planet-one-pilota-test"),
            ..default()
        };
        p.load("prova.txt", "# titolo\n\ntp 1 2 3\n  aspetta 2  \n");
        let lines: Vec<_> = p.lines.iter().cloned().collect();
        assert_eq!(lines, vec![(3, "tp 1 2 3".into()), (4, "aspetta 2".into())]);
    }
}
