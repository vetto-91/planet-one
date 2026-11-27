//! Ollama gestito dal gioco (ADR 0063).
//!
//! Se nella cartella dati c'è l'Ollama scaricato da `scripts/setup_ai.sh --ollama`, il gioco lo
//! avvia da sé all'avvio (con la IA accesa) e lo chiude all'uscita o quando la IA si spegne:
//! - ascolta solo su `127.0.0.1`, su una porta libera scelta a ogni avvio, non sulla 11434;
//! - tiene i modelli nella sua cartella (`OLLAMA_MODELS`), separati da un Ollama di sistema;
//! - muore col gioco anche se questo va in crash, insieme ai runner che tengono il modello in
//!   memoria: su Linux e macOS sta in un gruppo di processi suo, sorvegliato da un guardiano (il
//!   gioco stesso lanciato con `--ollama-guard`) che chiude il gruppo appena il gioco sparisce;
//!   su Linux c'è in più il segnale di morte del padre; su Windows un job object che si chiude
//!   con l'ultimo handle del gioco;
//!   all'avvio si chiudono gli Ollama rimasti da una partita finita male (Linux, `kill_orphans`).
//!
//! Senza l'Ollama del gioco resta il comportamento di prima: si usa `ai_host` (un Ollama già
//! avviato sul PC, sempre e solo in loopback).

use bevy::prelude::*;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

/// Quanto si aspetta che il server risponda dopo l'avvio.
const READY_TIMEOUT: Duration = Duration::from_secs(60);
/// Variabile che marca gli Ollama avviati dal gioco, col pid del gioco.
const OWNER_VAR: &str = "PLANET_ONE_OLLAMA_OWNER";
/// Quanto si aspetta la chiusura gentile prima di uccidere il processo.
const STOP_GRACE: Duration = Duration::from_secs(3);

/// Stato del server gestito.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ServerState {
    /// Nessun Ollama del gioco installato: si usa `ai_host`.
    #[default]
    Missing,
    /// Installato ma fermo (IA spenta).
    Stopped,
    Starting,
    /// In ascolto su questo indirizzo (`http://127.0.0.1:<porta>`).
    Ready(String),
    /// Non è partito o si è chiuso da solo (dettagli nel log e in `server.log`).
    Failed,
}

enum Cmd {
    Start,
    Stop(mpsc::Sender<()>),
}

/// Canale verso la thread che possiede il processo (vive quanto il gioco).
static SUPERVISOR: Mutex<Option<mpsc::Sender<Cmd>>> = Mutex::new(None);
/// Stato condiviso fra la thread e i sistemi.
static STATE: Mutex<ServerState> = Mutex::new(ServerState::Missing);

fn set_state(s: ServerState) {
    if let Ok(mut st) = STATE.lock() {
        *st = s;
    }
}

/// Stato attuale del server gestito.
pub fn state() -> ServerState {
    STATE.lock().map(|s| s.clone()).unwrap_or_default()
}

/// Cartella dell'Ollama del gioco: programma in `bin/` (Linux) o alla radice, modelli in `models/`.
pub fn ollama_dir() -> PathBuf {
    super::tts::data_dir().join("ai").join("ollama")
}

/// L'eseguibile dell'Ollama del gioco: `PLANET_ONE_OLLAMA`, poi la cartella dati. Non si cerca nel
/// `PATH`: un Ollama di sistema si usa com'è, già avviato, tramite `ai_host`.
pub fn find_ollama() -> Option<PathBuf> {
    if let Some(v) = std::env::var_os("PLANET_ONE_OLLAMA") {
        let p = PathBuf::from(v);
        return p.is_file().then_some(p);
    }
    let exe = if cfg!(target_os = "windows") {
        "ollama.exe"
    } else {
        "ollama"
    };
    let dir = ollama_dir();
    [dir.join("bin").join(exe), dir.join(exe)]
        .into_iter()
        .find(|p| p.is_file())
}

/// Una porta libera su loopback (la sceglie il sistema).
fn free_port() -> std::io::Result<u16> {
    let l = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(l.local_addr()?.port())
}

/// Chiede al sistema di avviare il server (se installato e non già acceso).
pub fn start() {
    send(Cmd::Start, true);
}

/// Ferma il server e aspetta che sia chiuso (pochi secondi al massimo). La chiama anche `main`
/// all'uscita del gioco.
pub fn stop() {
    let (tx, rx) = mpsc::channel();
    if send(Cmd::Stop(tx), false) {
        let _ = rx.recv_timeout(STOP_GRACE + Duration::from_secs(2));
    }
}

/// Manda un comando alla thread del server; `create` la fa nascere se manca.
fn send(cmd: Cmd, create: bool) -> bool {
    let Ok(mut guard) = SUPERVISOR.lock() else {
        return false;
    };
    if guard.is_none() && create {
        let Some(exe) = find_ollama() else {
            return false;
        };
        let (tx, rx) = mpsc::channel();
        // Una thread dedicata, che non finisce mai: su Linux il segnale di morte del padre
        // scatta quando muore la thread che ha creato il processo, non solo il gioco.
        let spawned = std::thread::Builder::new()
            .name("ollama-server".into())
            .spawn(move || supervise(exe, rx));
        if spawned.is_err() {
            return false;
        }
        // Da qui il gioco aspetta il suo Ollama invece di provare `ai_host`.
        set_state(ServerState::Stopped);
        *guard = Some(tx);
    }
    guard.as_ref().is_some_and(|tx| tx.send(cmd).is_ok())
}

struct Running {
    child: Child,
    /// Windows: il job object che contiene Ollama e i suoi runner (handle come intero).
    job: usize,
    /// Unix: il guardiano che chiude il gruppo se il gioco muore.
    guard: Option<Child>,
    started: Instant,
    ready: bool,
    base: String,
}

fn supervise(exe: PathBuf, rx: mpsc::Receiver<Cmd>) {
    kill_orphans(&exe);
    set_state(ServerState::Stopped);
    let mut run: Option<Running> = None;
    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(Cmd::Start) => {
                if run.is_none() {
                    run = launch(&exe);
                }
            }
            Ok(Cmd::Stop(ack)) => {
                if let Some(r) = run.take() {
                    shutdown(r);
                }
                set_state(ServerState::Stopped);
                let _ = ack.send(());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
        // Controlli periodici: pronto? chiuso da solo?
        if let Some(r) = run.as_mut() {
            if let Ok(Some(status)) = r.child.try_wait() {
                warn!(
                    "[AI] Ollama del gioco si è chiuso da solo ({status}); dettagli in {}",
                    ollama_dir().join("server.log").display()
                );
                // Restano da chiudere i runner del modello e il guardiano.
                if let Some(r) = run.take() {
                    shutdown(r);
                }
                set_state(ServerState::Failed);
            } else if !r.ready {
                if super::ollama::is_up(&r.base) {
                    r.ready = true;
                    info!(
                        "[AI] Ollama del gioco pronto su {} in {:.1} s",
                        r.base,
                        r.started.elapsed().as_secs_f32()
                    );
                    set_state(ServerState::Ready(r.base.clone()));
                } else if r.started.elapsed() > READY_TIMEOUT {
                    warn!("[AI] Ollama del gioco non risponde dopo {READY_TIMEOUT:?}: lo chiudo");
                    if let Some(r) = run.take() {
                        shutdown(r);
                    }
                    set_state(ServerState::Failed);
                }
            }
        }
    }
}

fn launch(exe: &std::path::Path) -> Option<Running> {
    let port = match free_port() {
        Ok(p) => p,
        Err(e) => {
            warn!("[AI] nessuna porta libera per Ollama: {e}");
            set_state(ServerState::Failed);
            return None;
        }
    };
    let dir = ollama_dir();
    let models = dir.join("models");
    let _ = std::fs::create_dir_all(&models);
    let log = std::fs::File::create(dir.join("server.log"));
    let mut cmd = Command::new(exe);
    cmd.arg("serve")
        .env("OLLAMA_HOST", format!("127.0.0.1:{port}"))
        .env("OLLAMA_MODELS", &models)
        // Niente modelli nel cloud di Ollama: tutto resta su questo PC (ADR 0005).
        .env("OLLAMA_NO_CLOUD", "1")
        // Il segno del padrone, per riconoscere gli orfani dopo un crash (`kill_orphans`).
        .env(OWNER_VAR, std::process::id().to_string())
        // Le origini consentite restano quelle predefinite (solo locali), non quelle del sistema.
        .env_remove("OLLAMA_ORIGINS")
        .stdin(Stdio::null());
    match log {
        Ok(f) => {
            let err = f
                .try_clone()
                .map(Stdio::from)
                .unwrap_or_else(|_| Stdio::null());
            cmd.stdout(Stdio::from(f)).stderr(err);
        }
        Err(_) => {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
    }
    tie_to_game(&mut cmd);
    set_state(ServerState::Starting);
    match cmd.spawn() {
        Ok(child) => {
            let job = bind_to_game(&child);
            let guard = spawn_guard(&child);
            info!(
                "[AI] avvio Ollama del gioco ({}) su 127.0.0.1:{port}, modelli in {}",
                exe.display(),
                models.display()
            );
            Some(Running {
                child,
                job,
                guard,
                started: Instant::now(),
                ready: false,
                base: format!("http://127.0.0.1:{port}"),
            })
        }
        Err(e) => {
            warn!("[AI] Ollama del gioco non parte: {e}");
            set_state(ServerState::Failed);
            None
        }
    }
}

/// Chiusura: prima gentile (i runner del modello si chiudono con lui), poi forzata.
fn shutdown(mut r: Running) {
    terminate(&r.child, r.job);
    let until = Instant::now() + STOP_GRACE;
    while Instant::now() < until {
        if let Ok(Some(_)) = r.child.try_wait() {
            kill_group(&r.child);
            end_guard(r.guard.take());
            info!("[AI] Ollama del gioco chiuso");
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    kill_group(&r.child);
    let _ = r.child.kill();
    let _ = r.child.wait();
    end_guard(r.guard.take());
    info!("[AI] Ollama del gioco chiuso (forzato)");
}

/// Il guardiano non serve più: chiudergli lo stdin lo fa uscire.
fn end_guard(guard: Option<Child>) {
    if let Some(mut g) = guard {
        drop(g.stdin.take());
        let until = Instant::now() + Duration::from_secs(1);
        while Instant::now() < until {
            if let Ok(Some(_)) = g.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = g.kill();
        let _ = g.wait();
    }
}

/// Argomento con cui il gioco parte come guardiano del suo Ollama.
const GUARD_ARG: &str = "--ollama-guard";

/// Unix: lancia il guardiano, cioè il gioco stesso con `--ollama-guard <gruppo>`. Ha lo stdin
/// collegato al gioco: quando il gioco muore, in qualunque modo, lo stdin si chiude e il
/// guardiano ferma tutto il gruppo di Ollama. Sta in un gruppo suo, così Ctrl+C nel terminale
/// non lo uccide prima del tempo.
#[cfg(unix)]
fn spawn_guard(child: &Child) -> Option<Child> {
    use std::os::unix::process::CommandExt;
    let exe = std::env::current_exe().ok()?;
    let spawned = Command::new(exe)
        .arg(GUARD_ARG)
        .arg(child.id().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn();
    match spawned {
        Ok(g) => Some(g),
        Err(e) => {
            warn!("[AI] guardiano di Ollama non avviato: {e}");
            None
        }
    }
}

#[cfg(not(unix))]
fn spawn_guard(_: &Child) -> Option<Child> {
    None
}

/// Da chiamare per prima cosa in `main`: se il gioco è stato lanciato come guardiano fa solo
/// quello e restituisce `true` (il processo deve uscire subito, senza aprire nulla).
pub fn run_guard() -> bool {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some(GUARD_ARG) {
        return false;
    }
    #[cfg(unix)]
    if let Some(group) = args.next().and_then(|a| a.parse::<libc::pid_t>().ok())
        && group > 1
    {
        use std::io::Read;
        // Si resta qui finché il gioco tiene aperto lo stdin: si chiude quando il gioco esce,
        // anche per un crash o un `kill -9`.
        let mut sink = [0u8; 64];
        let mut stdin = std::io::stdin();
        while matches!(stdin.read(&mut sink), Ok(n) if n > 0) {}
        // SAFETY: segnali al gruppo di Ollama creato dal gioco (pgid = pid del server).
        unsafe {
            if libc::kill(-group, 0) == 0 {
                libc::kill(-group, libc::SIGTERM);
                for _ in 0..40 {
                    std::thread::sleep(Duration::from_millis(50));
                    if libc::kill(-group, 0) != 0 {
                        return true;
                    }
                }
                libc::kill(-group, libc::SIGKILL);
            }
        }
    }
    true
}

// ---------- Legame col processo del gioco, per sistema ----------

#[cfg(target_os = "linux")]
fn tie_to_game(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    let parent = std::process::id() as libc::pid_t;
    // SAFETY: fra fork ed exec solo chiamate async-signal-safe (setpgid, prctl, getppid).
    unsafe {
        cmd.pre_exec(move || {
            // Gruppo suo: alla chiusura si ferma anche il runner del modello.
            libc::setpgid(0, 0);
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
            // Il gioco è morto prima del prctl: non partire orfano.
            if libc::getppid() != parent {
                libc::_exit(1);
            }
            Ok(())
        });
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
fn tie_to_game(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

#[cfg(windows)]
fn tie_to_game(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(any(unix, windows)))]
fn tie_to_game(_: &mut Command) {}

#[cfg(windows)]
fn bind_to_game(child: &Child) -> usize {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };
    // SAFETY: chiamate Win32 con puntatori validi; il job resta aperto finché vive il gioco e
    // si chiude col processo, portandosi dietro Ollama e i suoi runner.
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            warn!("[AI] job object non creato: Ollama potrebbe sopravvivere a un crash");
            return 0;
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if ok == 0 || AssignProcessToJobObject(job, child.as_raw_handle() as _) == 0 {
            warn!("[AI] Ollama non legato al gioco: potrebbe sopravvivere a un crash");
        }
        job as usize
    }
}

#[cfg(not(windows))]
fn bind_to_game(_: &Child) -> usize {
    0
}

#[cfg(unix)]
fn terminate(child: &Child, _job: usize) {
    // SAFETY: segnale al gruppo che il figlio ha creato per sé (pgid = pid del figlio).
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGTERM);
    }
}

/// Windows: chiude il job, cioè Ollama e i runner del modello insieme.
#[cfg(windows)]
fn terminate(_: &Child, job: usize) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::TerminateJobObject;
    if job != 0 {
        // SAFETY: handle creato da `bind_to_game` e chiuso solo qui.
        unsafe {
            TerminateJobObject(job as _, 0);
            CloseHandle(job as _);
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn terminate(_: &Child, _: usize) {}

#[cfg(unix)]
fn kill_group(child: &Child) {
    // SAFETY: come sopra; il gruppo può essere già vuoto.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn kill_group(_: &Child) {}

/// Linux: chiude gli Ollama rimasti da una partita finita male (per esempio i runner del modello
/// dopo un crash). Sono riconoscibili dalla variabile `PLANET_ONE_OLLAMA_OWNER`, che il gioco dà al
/// suo Ollama e che passa ai runner: si chiude solo chi ha un padrone che non è più un Planet One
/// vivo. Un Ollama di sistema, o quello dello script di installazione, non la ha.
#[cfg(target_os = "linux")]
fn kill_orphans(exe: &std::path::Path) {
    let Ok(exe) = exe.canonicalize() else {
        return;
    };
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return;
    };
    let game_alive = |owner: &str| {
        std::fs::read_link(format!("/proc/{owner}/exe"))
            .ok()
            .and_then(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("planet-one"))
            })
            .unwrap_or(false)
    };
    for entry in dir.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<i32>().ok())
        else {
            continue;
        };
        if std::fs::read_link(entry.path().join("exe")).ok().as_deref() != Some(exe.as_path()) {
            continue;
        }
        let Ok(env) = std::fs::read(entry.path().join("environ")) else {
            continue;
        };
        let owner = env
            .split(|b| *b == 0)
            .find_map(|v| v.strip_prefix(OWNER_VAR.as_bytes())?.strip_prefix(b"="))
            .map(|v| String::from_utf8_lossy(v).into_owned());
        if let Some(owner) = owner
            && !game_alive(&owner)
        {
            warn!("[AI] chiudo un Ollama rimasto da una partita precedente (pid {pid})");
            // SAFETY: processo del nostro eseguibile, avviato da un gioco che non c'è più.
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn kill_orphans(_: &std::path::Path) {}

/// Sistema: accende e spegne il server insieme alla IA (menu Esc o shell).
pub fn follow_ai_switch(settings: Res<crate::audio::AudioSettings>, mut last: Local<Option<bool>>) {
    let before = last.replace(settings.ai_enabled);
    if before == Some(settings.ai_enabled) {
        return;
    }
    if settings.ai_enabled {
        start();
    } else if before.is_some() {
        // Fermarlo aspetta qualche istante: lo si fa fuori dal frame.
        std::thread::spawn(stop);
    }
}
