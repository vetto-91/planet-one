//! Pilota automatico per il beta test: esegue un copione di comandi (`PLANET_ONE_SCRIPT=file`)
//! senza tastiera né mouse, fotografa lo stato del gioco in JSON e controlla condizioni.
//!
//! Ogni riga del copione è un comando; le righe vuote e quelle che iniziano con `#` si saltano.
//! Comandi propri del pilota:
//! - `aspetta <s>`: attende `s` secondi di gioco;
//! - `premi <tasto> [s]`: tiene premuto un tasto (`w a s d spazio shift ctrl e invio esc tab su giu
//!   sinistra destra 0-9 i j m t c v f5 f9 click clickdx`) per `s` secondi (predefinito 0,1);
//! - `avanza`: fa scorrere il dialogo (Invio) finché compaiono le scelte o si chiude;
//! - `tieni <tasto> <s>`: come `premi`, ma il copione prosegue subito (foto in movimento);
//! - `scegli <n o testo>`: nel dialogo sceglie la risposta (numero da 1 o parte del testo) e conferma;
//! - `foto <nome>`: screenshot del gioco in `<uscita>/<nome>.png`;
//! - `dump [nome]`: stato del gioco in JSON in `<uscita>/stato-<nome>.json`;
//! - `verifica <campo> <op> <valore>`: confronto sullo stato (`=`, `!=`, `<`, `<=`, `>`, `>=`,
//!   `contiene`); i campi sono in [`field`]. Un fallimento scrive `[VERIFICA] FALLITA` nel log;
//! - `attendi <campo> <op> <valore> [s]`: aspetta che la condizione sia vera (massimo `s`, 30 s);
//! - `mai <campo> <op> <valore> [s]`: la condizione non deve mai essere vera per `s` secondi;
//! - `mira <tipo o id>`: mette nel mirino l'oggetto con stato più vicino (porta, scuri, mobile…);
//! - `compra <n o nome>`, `prendi` (tutto dal mobile aperto), `chiudi` (torna al gioco): i pannelli
//!   egui non ricevono i tasti simulati, quindi il pilota passa dalla logica del gioco;
//! - `di <testo>`: nella chat con l'IA locale manda una frase;
//! - `registra <nome>` / `ferma`: registra in `<uscita>/<nome>.wav` ciò che esce dagli altoparlanti
//!   (solo Linux, `parec`); `scripts/ascolta.py` lo analizza (volume, picchi, silenzi, spettrogramma, parlato);
//! - `camera <yaw> <pitch> [distanza]`, `ruota <gradi>`, `prima-persona [off]`: punto di vista;
//! - `giro-foto <nome> <n> [pitch] [distanza]`: n foto tutt'attorno al giocatore (una ogni 360/n gradi);
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

/// Un frame più lungo di così è un intoppo visibile (meno di 10 FPS per un attimo).
const SLOW_FRAME_MS: f32 = 100.0;

/// Stato del pilota e richieste dalla shell (`foto`, `dump`).
#[derive(Resource, Default)]
pub struct Autopilot {
    lines: VecDeque<(usize, String)>,
    pub active: bool,
    started: bool,
    frames: u32,
    wait: f32,
    /// Tasti tenuti: tasto, secondi rimasti, il copione aspetta che finisca (`premi`) o no (`tieni`).
    held: Vec<(PilotKey, f32, bool)>,
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
    /// Scelta del dialogo da impostare prima della conferma: numero (da 1) o parte del testo.
    choice: Option<String>,
    /// `avanza`: righe del dialogo ancora da far scorrere (fino alle scelte o alla chiusura).
    advance: u32,
    pub failures: Vec<String>,
    /// FPS misurati una volta al secondo durante il copione (prestazioni nel rapporto).
    fps: Vec<f32>,
    fps_timer: f32,
    /// Risoluzione vera della finestra (il gestore delle finestre può ridurre quella chiesta).
    resolution: Option<(u32, u32)>,
    /// Ultimo comando eseguito (riga, testo): a chi attribuire un frame lento.
    current: (usize, String),
    /// Frame più lenti di `SLOW_FRAME_MS`: (durata in ms, riga, comando).
    slow: Vec<(f32, usize, String)>,
    /// Registrazione dell'audio del sistema in corso (`registra`), con il file di uscita.
    recorder: Option<(std::process::Child, PathBuf)>,
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
        let (w, h) = crate::devtools::window_size();
        let (rw, rh) = self.resolution.unwrap_or((w, h));
        let note = if (rw, rh) != (w, h) {
            format!(" (chiesta {w}x{h})")
        } else {
            String::new()
        };
        let mut s = format!(
            "Copione: {}\nRisoluzione: {rw}x{rh}{note}\nVerifiche passate: {}\nFallimenti: {}\n",
            self.script,
            self.passed,
            self.failures.len()
        );
        if let Some((avg, low, min)) = fps_summary(&self.fps) {
            s.push_str(&format!(
                "FPS: medio {avg:.0}, 1% peggiore {low:.0}, minimo {min:.0} ({} campioni)\n",
                self.fps.len()
            ));
        }
        if let Some((ms, n, cmd)) = self.slow.iter().max_by(|a, b| a.0.total_cmp(&b.0)) {
            s.push_str(&format!(
                "Frame lenti (oltre {SLOW_FRAME_MS:.0} ms): {}, il peggiore {ms:.0} ms alla riga {n} ({cmd})\n",
                self.slow.len()
            ));
        }
        for f in &self.failures {
            s.push_str(&format!("- {f}\n"));
        }
        s
    }
}

/// Avvia la registrazione di ciò che esce dagli altoparlanti (monitor della scheda audio
/// predefinita, PulseAudio o PipeWire) in un file WAV. Solo Linux: `parec` e `pactl`.
/// Con `PULSE_SINK` (le prove di `beta_check.sh` suonano in una scheda virtuale, a casse mute)
/// registra quella.
fn start_recording(path: &std::path::Path) -> Result<std::process::Child, String> {
    let sink = match std::env::var("PULSE_SINK") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            let out = std::process::Command::new("pactl")
                .arg("get-default-sink")
                .output()
                .map_err(|e| format!("pactl: {e}"))?;
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
    };
    if sink.is_empty() {
        return Err("nessuna scheda audio predefinita".into());
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::process::Command::new("parec")
        .args([
            "-d",
            &format!("{sink}.monitor"),
            "--file-format=wav",
            "--rate=44100",
            "--channels=2",
        ])
        .arg(path)
        .spawn()
        .map_err(|e| format!("parec: {e}"))
}

/// Ferma la registrazione: SIGINT perché `parec` chiuda bene l'intestazione del WAV.
fn stop_recording(rec: Option<(std::process::Child, PathBuf)>) {
    let Some((mut child, path)) = rec else {
        return;
    };
    #[cfg(unix)]
    // SAFETY: si manda un segnale al processo figlio di cui si conosce il pid.
    unsafe {
        libc::kill(child.id() as libc::pid_t, libc::SIGINT);
    }
    #[cfg(not(unix))]
    let _ = child.kill();
    let _ = child.wait();
    info!("[PILOTA] registrazione salvata in {}", path.display());
}

/// FPS medio, media dell'1% peggiore e minimo (scartati i primi 5 s, di caricamento).
pub fn fps_summary(samples: &[f32]) -> Option<(f32, f32, f32)> {
    let s: Vec<f32> = samples
        .iter()
        .skip(5)
        .copied()
        .filter(|f| *f > 0.0)
        .collect();
    if s.is_empty() {
        return None;
    }
    let avg = s.iter().sum::<f32>() / s.len() as f32;
    let mut sorted = s.clone();
    sorted.sort_by(f32::total_cmp);
    let n = (sorted.len() / 100).max(1);
    let low = sorted[..n].iter().sum::<f32>() / n as f32;
    Some((avg, low, sorted[0]))
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
        "q" => K::KeyQ,
        "z" => K::KeyZ,
        "f" => K::KeyF,
        "g" => K::KeyG,
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

/// Indice della scelta: numero da 1 oppure parte del testo (le scelte possono cambiare, per esempio
/// «Parliamo d'altro» c'è solo con l'IA locale attiva).
pub fn pick_choice(choices: &[String], wanted: &str) -> Option<usize> {
    if let Ok(n) = wanted.parse::<usize>() {
        return (n >= 1 && n <= choices.len()).then(|| n - 1);
    }
    let wanted = wanted.to_lowercase();
    choices
        .iter()
        .position(|c| c.to_lowercase().contains(&wanted))
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
        "regione" => "/regione",
        "ora" => "/ora",
        "meteo" => "/meteo",
        "fps" => "/fps",
        "vicini" => "/vicini",
        "arma" => "/arma",
        p if p.starts_with('/') => p,
        // `npc:<nome>:<campo>`: il primo abitante vicino il cui nome contiene `<nome>`.
        // `prop:<id>:<campo>`: l'oggetto con stato vicino il cui id contiene `<id>`.
        p if p.starts_with("prop:") => {
            let (who, what) = p[5..].rsplit_once(':')?;
            return state
                .pointer("/vicini")?
                .as_array()?
                .iter()
                .find(|v| {
                    v.get("id")
                        .and_then(Value::as_str)
                        .is_some_and(|n| n.contains(who))
                })?
                .get(what);
        }
        p if p.starts_with("npc:") => {
            let mut parts = p[4..].splitn(2, ':');
            // Nei nomi di due parole il trattino basso sta per lo spazio («guardia_gino»).
            let who = parts.next()?.to_lowercase().replace('_', " ");
            let what = parts.next().unwrap_or("dist");
            // Il nome esatto vince su quello che lo contiene (Gino, non la Guardia Gino).
            let rank = |v: &Value| {
                v.get("nome")
                    .and_then(Value::as_str)
                    .map_or(0, |n| crate::shell::name_rank(n, &who))
            };
            return state
                .pointer("/vicini")?
                .as_array()?
                .iter()
                .max_by_key(|v| rank(v))
                .filter(|v| rank(v) > 0)?
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

/// Pannelli egui che il pilota comanda passando dalla logica del gioco (egui non riceve i tasti
/// simulati in `ButtonInput`): negozio, mobili, chiusura dei pannelli.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PilotUi<'w, 's> {
    shop: ResMut<'w, crate::shop::ShopState>,
    containers: ResMut<'w, crate::containers::Containers>,
    inventory: ResMut<'w, crate::inventory::Inventory>,
    next: ResMut<'w, NextState<crate::AppState>>,
    chat: ResMut<'w, crate::ai::AiChat>,
    diagnostics: Res<'w, DiagnosticsStore>,
    windows: Query<'w, 's, &'static Window>,
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
    props: Query<(
        &GlobalTransform,
        &crate::hut::PropKind,
        Option<&crate::hut::PropId>,
    )>,
    mut exit: MessageWriter<AppExit>,
    mut ui: PilotUi,
) {
    if !pilot.active {
        return;
    }
    let dt = time.delta_secs();
    // Prestazioni: un campione di FPS al secondo (tempo reale del gioco).
    pilot.fps_timer -= dt;
    if pilot.fps_timer <= 0.0 {
        pilot.fps_timer = 1.0;
        if let Some(win) = ui.windows.iter().next() {
            pilot.resolution = Some((win.physical_width(), win.physical_height()));
        }
        if let Some(f) = ui
            .diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.smoothed())
        {
            pilot.fps.push(f as f32);
        }
    }
    // Intoppi: ogni frame lento, con il comando che stava girando.
    if pilot.started
        && let Some(ms) = ui
            .diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
            .and_then(|d| d.value())
        && ms as f32 > SLOW_FRAME_MS
        // La foto stessa (copia dell'immagine dalla scheda video) non è un intoppo del gioco.
        && !pilot.current.1.starts_with("foto")
        && !pilot.current.1.starts_with("giro-foto")
    {
        let (n, cmd) = pilot.current.clone();
        warn!("[PILOTA] frame lento: {ms:.0} ms (riga {n}: {cmd})");
        pilot.slow.push((ms as f32, n, cmd));
    }
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
    held.retain_mut(|(k, left, _)| {
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
    if let Some(wanted) = pilot.choice.take() {
        let index = pick_choice(&dialog.choices, &wanted);
        match index.filter(|_| dialog.open && dialog.choosing) {
            Some(n) => {
                dialog.selected = n;
                keys.press(KeyCode::Enter);
                pilot.held.push((PilotKey::Key(KeyCode::Enter), 0.05, true));
            }
            None => pilot.fail(0, format!("scegli {wanted}: nessuna scelta così")),
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
            pilot.held.push((PilotKey::Key(KeyCode::Enter), 0.05, true));
            pilot.wait = 0.35;
            return;
        }
        pilot.advance = 0;
    }
    // La shell ha ancora comandi da eseguire, oppure un tasto è ancora tenuto.
    if !shell.pending.is_empty() || pilot.held.iter().any(|h| h.2) || pilot.photo.is_some() {
        return;
    }
    let Some((n, line)) = pilot.lines.pop_front() else {
        return;
    };
    info!("[PILOTA] > {line}");
    pilot.current = (n, line.clone());
    let words: Vec<&str> = line.split_whitespace().collect();
    let num = |i: usize| -> Option<f32> { words.get(i)?.replace(',', ".").parse().ok() };
    match words[0].to_lowercase().as_str() {
        "aspetta" | "wait" => pilot.wait = num(1).unwrap_or(1.0),
        "premi" | "press" | "tieni" | "hold" => match words.get(1).and_then(|k| parse_key(k)) {
            Some(k) => {
                match k {
                    PilotKey::Key(c) => keys.press(c),
                    PilotKey::Mouse(b) => mouse.press(b),
                }
                // `tieni` non blocca il copione: si può fotografare mentre il tasto è premuto.
                let blocking = matches!(words[0].to_lowercase().as_str(), "premi" | "press");
                pilot.held.push((k, num(2).unwrap_or(0.1), blocking));
            }
            None => pilot.fail(n, format!("tasto sconosciuto in «{line}»")),
        },
        "scegli" | "choose" => {
            if words.len() > 1 {
                pilot.choice = Some(words[1..].join(" "));
            } else {
                pilot.fail(n, format!("uso: scegli <n o testo> («{line}»)"));
            }
        }
        "avanza" | "next" => pilot.advance = 30,
        "camera" => match (num(1), num(2)) {
            // Angoli assoluti in gradi (yaw 0 = camera a sud del giocatore, guarda verso nord).
            (Some(yaw), Some(pitch)) => {
                orbit.yaw = yaw.to_radians();
                orbit.pitch = pitch.to_radians();
                if let Some(d) = num(3) {
                    orbit.distance = d.clamp(1.5, 30.0);
                }
            }
            _ => pilot.fail(
                n,
                format!("uso: camera <yaw> <pitch> [distanza] («{line}»)"),
            ),
        },
        "ruota" | "rotate" => match num(1) {
            Some(deg) => orbit.yaw += deg.to_radians(),
            None => pilot.fail(n, format!("uso: ruota <gradi> («{line}»)")),
        },
        "prima-persona" | "first-person" => {
            let on = words.get(1).is_none_or(|w| *w != "off");
            orbit.first_person_lock = on;
            orbit.mode = if on {
                crate::camera::CameraMode::FirstPerson
            } else {
                crate::camera::CameraMode::ThirdPerson
            };
        }
        "giro-foto" | "orbit-shots" => {
            // `giro-foto <nome> <n> [pitch] [distanza]`: n foto attorno al giocatore, a passi uguali.
            let name = words.get(1).copied().unwrap_or("giro");
            let count = num(2).map_or(8, |c| c.clamp(1.0, 36.0) as usize);
            let pitch = num(3).unwrap_or(15.0);
            let dist = num(4).map_or(String::new(), |d| format!(" {d}"));
            let mut expanded: Vec<(usize, String)> = Vec::new();
            for i in 0..count {
                let yaw = i as f32 * 360.0 / count as f32;
                expanded.push((n, format!("camera {yaw:.0} {pitch}{dist}")));
                expanded.push((n, "aspetta 0.4".to_string()));
                expanded.push((n, format!("foto {name}-{:03.0}", yaw)));
            }
            for l in expanded.into_iter().rev() {
                pilot.lines.push_front(l);
            }
        }
        "compra" | "buy" => {
            let wanted = words[1..].join(" ").to_lowercase();
            let stock = crate::shop::stock(&ui.shop.key);
            let found = wanted
                .parse::<usize>()
                .ok()
                .and_then(|i| i.checked_sub(1))
                .filter(|i| *i < stock.len())
                .or_else(|| {
                    stock
                        .iter()
                        .position(|s| s.kind.name().to_lowercase().contains(&wanted))
                });
            match (found, *state.get() == crate::AppState::Shop) {
                (Some(i), true) => ui.shop.pending = Some(i),
                (_, false) => pilot.fail(n, "compra: nessun negozio aperto".into()),
                (None, true) => pilot.fail(n, format!("compra: «{wanted}» non è in vendita")),
            }
        }
        "prendi" | "take" => {
            let moved = ui.containers.take_all_open(&mut ui.inventory);
            info!("[PILOTA] presi {moved} oggetti dal mobile");
        }
        "chiudi" | "close" => ui.next.set(crate::AppState::Playing),
        "di" | "say" => {
            if *state.get() == crate::AppState::Chat {
                ui.chat.action = Some(crate::ai::ChatAction::Send(words[1..].join(" ")));
            } else {
                pilot.fail(n, "di: nessuna chat aperta".into());
            }
        }
        "mira" | "aim" => {
            // L'oggetto con stato più vicino (entro 6 m) il cui tipo o id contiene la parola.
            let wanted = words[1..].join(" ").to_lowercase();
            let here = players.single().map(|t| t.translation).unwrap_or_default();
            let target = props
                .iter()
                .filter(|(_, kind, id)| {
                    format!("{kind:?}").to_lowercase().contains(&wanted)
                        || id.is_some_and(|i| i.0.to_lowercase().contains(&wanted))
                })
                .map(|(g, ..)| g.translation())
                .filter(|p| p.distance(here) < 6.0)
                .min_by(|a, b| a.distance(here).total_cmp(&b.distance(here)));
            match target {
                Some(p) => pilot.aim = Some((p, 0)),
                None => pilot.fail(n, format!("mira {wanted}: niente entro 6 m")),
            }
        }
        "foto" | "shot" => {
            let name = words.get(1).copied().unwrap_or("foto");
            pilot.photo = Some(pilot.out.join(format!("{name}.png")));
            // Il salvataggio è asincrono: si lascia il tempo di finire prima di `esci`.
            pilot.wait = 0.5;
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
        "registra" | "record" => {
            stop_recording(pilot.recorder.take());
            let name = words.get(1).copied().unwrap_or("audio");
            let path = pilot.out.join(format!("{name}.wav"));
            match start_recording(&path) {
                Ok(child) => {
                    info!("[PILOTA] registro l'audio in {}", path.display());
                    pilot.recorder = Some((child, path));
                }
                Err(e) => pilot.fail(n, format!("registra: {e}")),
            }
        }
        "ferma" | "stop" => stop_recording(pilot.recorder.take()),
        "esci" | "quit" => {
            stop_recording(pilot.recorder.take());
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
pub fn take_photos(
    mut commands: Commands,
    mut pilot: ResMut<Autopilot>,
    headless: Option<Res<crate::devtools::HeadlessTarget>>,
) {
    if let Some(path) = pilot.photo.take() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        info!("[PILOTA] foto {}", path.display());
        // Senza finestra si fotografa l'immagine su cui disegna la camera (senza HUD né pannelli).
        let shot = match &headless {
            Some(target) => Screenshot::image(target.0.clone()),
            None => Screenshot::primary_window(),
        };
        commands.spawn(shot).observe(save_to_disk(path));
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
        "sorte": quest.fate.key(),
        "tavoletta": quest.tablet,
        "tenute": quest.kept,
        "moneta_madre": quest.mother_coin,
    });
    // Missioni del Prologo: tappa corrente (da 1) di ognuna, e ciò che hanno sbloccato.
    let missioni = {
        let m = world.resource::<crate::quests::missions::Missions>();
        let mut map = serde_json::Map::new();
        map.insert("attive".into(), json!(m.enabled));
        map.insert("continente".into(), json!(m.unlocked("continente")));
        for (id, step) in m.snapshot() {
            map.insert(id, json!(step + 1));
        }
        Value::Object(map)
    };
    let inv = world.resource::<crate::inventory::Inventory>();
    let mut items = serde_json::Map::new();
    for s in &inv.slots {
        items.insert(format!("{:?}", s.kind), json!(s.qty));
    }
    let equip = world.resource::<crate::inventory::Equipment>();
    let arma = equip.weapon.map(|w| format!("{w:?}"));
    let shop = world.resource::<crate::shop::ShopState>();
    let negozio = json!({
        "venditore": shop.vendor,
        "messaggio": shop.message,
        "articoli": crate::shop::stock(&shop.key)
            .iter()
            .map(|s| format!("{} ({} monete)", s.kind.name(), s.price))
            .collect::<Vec<_>>(),
    });
    let mobile = world
        .resource::<crate::containers::Containers>()
        .open_items()
        .map(|(key, items)| {
            json!({
                "chiave": key,
                "oggetti": items
                    .iter()
                    .map(|s| format!("{:?} x{}", s.kind, s.qty))
                    .collect::<Vec<_>>(),
            })
        });
    let c = world.resource::<crate::ai::AiChat>();
    let chat = json!({
        "con": c.npc_name,
        "pensa": c.thinking,
        "errore": c.error,
        "turni": c.history.len(),
        "risposta": c.history.iter().rev().find(|t| !t.player).map(|t| t.text.clone()),
    });
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
    // Tutti gli abitanti, ovunque siano: servono a trovare chi resta bloccato.
    let mut folks = world.query::<(
        &GlobalTransform,
        &crate::entities::Npc,
        &crate::villagers::Villager,
    )>();
    let abitanti: Vec<Value> = folks
        .iter(world)
        .map(|(g, npc, v)| {
            let p = g.translation();
            let (hidden, path, indoor) = v.debug_route();
            json!({
                "nome": npc.name,
                "pos": [p.x, p.y, p.z],
                "fase": v.phase_name(),
                "modo": format!("{:?}", npc.mode),
                "nascosto": hidden,
                "strada": path,
                "casa": indoor,
            })
        })
        .collect();
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
    let progress = world.resource::<crate::skills::Progress>().clone();
    let abilita: serde_json::Map<String, serde_json::Value> = crate::skills::Skill::ALL
        .into_iter()
        .flat_map(|s| {
            [
                (s.key().to_string(), json!(progress.grade(s))),
                (format!("talento_{}", s.key()), json!(progress.perk_rank(s))),
            ]
        })
        .collect();
    json!({
        "app": app,
        "ora": (hour * 100.0).round() / 100.0,
        "data": date,
        "meteo": meteo,
        "temperatura": temp,
        "fps": fps.map(|f| f.round()),
        "entita": world.entities().len(),
        "regione": crate::region::current().key(),
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
        "missioni": missioni,
        "abilita": abilita,
        "esplorati": world
            .resource::<crate::places::Exploration>()
            .0
            .iter()
            .map(|(k, e)| (k.clone(), json!(e.label())))
            .collect::<serde_json::Map<String, Value>>(),
        "vista": {
            "yaw": (world.resource::<OrbitCamera>().yaw.to_degrees() * 10.0).round() / 10.0,
            "pitch": (world.resource::<OrbitCamera>().pitch.to_degrees() * 10.0).round() / 10.0,
            "prima_persona": world.resource::<OrbitCamera>().mode
                == crate::camera::CameraMode::FirstPerson,
        },
        "anelli": world.resource::<crate::archery::Trial>().hits,
        "prova_arco": world.resource::<crate::archery::Trial>().passed,
        "casa": world.resource::<crate::house::House>().stage,
        "incantesimo": world.resource::<crate::magic::Magic>().selected.key(),
        "scudo_etereo": world.resource::<crate::magic::Magic>().ward > 0.0,
        "sfera_di_luce": world.resource::<crate::magic::Magic>().light > 0.0,
        "karma": world.resource::<crate::karma::Karma>().global,
        "fiducia": world
            .resource::<crate::karma::Karma>()
            .trust
            .iter()
            .map(|(k, v)| (k.replace(' ', "_"), json!(v)))
            .collect::<serde_json::Map<String, Value>>(),
        "segreti": world
            .resource::<crate::karma::Karma>()
            .revealed
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        "serrature": world.resource::<crate::thievery::Thievery>().picked,
        "infilate": world
            .resource::<crate::thievery::Thievery>()
            .slipped
            .values()
            .sum::<u32>(),
        "vigore": world.resource::<crate::vitals::Vitals>().vigor.round(),
        "magia": world.resource::<crate::vitals::Vitals>().magic.round(),
        "livello": progress.level,
        "punti_statistica": progress.stat_points,
        "nodi": progress.nodes,
        "luoghi": world
            .resource::<crate::places::Known>()
            .0
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        "inventario": items,
        "dialogo": dialog,
        "negozio": negozio,
        "chat": chat,
        "mobile": mobile,
        "vicini": near.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
        "abitanti": abitanti,
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
        .add_systems(
            Update,
            (
                take_photos,
                // Dopo la morte: lo stato non mostra mai una vita negativa di passaggio.
                dump_state.after(crate::combat::player_death),
            ),
        );
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
        let s = json!({"vicini": [{"id": "casetta:scuri:0", "attivo": true}]});
        assert!(evaluate(&s, "prop:casetta:scuri:0:attivo = true").is_ok());
    }

    #[test]
    fn choices_by_number_or_text() {
        let c = vec!["Vorrei comprare".to_string(), "Arrivederci".to_string()];
        assert_eq!(pick_choice(&c, "2"), Some(1));
        assert_eq!(pick_choice(&c, "3"), None);
        assert_eq!(pick_choice(&c, "arrived"), Some(1));
        assert_eq!(pick_choice(&c, "boh"), None);
    }

    #[test]
    fn fps_summary_skips_loading_and_finds_the_worst() {
        let mut s = vec![5.0; 5];
        s.extend((0..200).map(|i| if i == 10 { 12.0 } else { 60.0 }));
        let (avg, low, min) = fps_summary(&s).unwrap();
        assert!(avg > 59.0 && avg < 60.0);
        assert_eq!(min, 12.0);
        assert!(low < 40.0);
        assert!(fps_summary(&[1.0, 2.0]).is_none());
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
