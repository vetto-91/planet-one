//! Tasto F12 «Segnala bug»: salva in una cartella lo screenshot, lo stato del gioco in JSON
//! (come `dump` del pilota automatico), le ultime righe del log e la nota scritta nel gioco.
//!
//! La cartella è `<dati>/segnalazioni/<data-ora>/` (Linux: `~/.local/share/planet-one/`).

use bevy::prelude::*;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Mutex;

/// Righe di log tenute in memoria per le segnalazioni.
const KEEP: usize = 400;

static RECENT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// Scrittore del log: stampa su stdout e tiene le ultime righe (senza colori) in memoria.
pub struct TeeWriter;

impl Write for TeeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = std::io::stdout().write(buf)?;
        if let Ok(mut recent) = RECENT.lock() {
            let text = strip_ansi(&String::from_utf8_lossy(&buf[..n]));
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                recent.push_back(line.to_string());
            }
            let excess = recent.len().saturating_sub(KEEP);
            recent.drain(..excess);
        }
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

/// Costruttore dello scrittore per `tracing_subscriber::fmt::layer().with_writer(...)`.
pub fn tee() -> TeeWriter {
    TeeWriter
}

/// Toglie le sequenze di colore ANSI (`ESC [ ... m`).
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            for d in chars.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Ultime righe del log.
pub fn recent_log() -> Vec<String> {
    RECENT
        .lock()
        .map(|r| r.iter().cloned().collect())
        .unwrap_or_default()
}

/// La segnalazione in corso (F12): la cartella dell'ultima, la foto appena fatta e la nota.
///
/// Ogni F12 apre una casella per scrivere cosa non va. Una foto con la nota vuota è un'altra
/// inquadratura della segnalazione precedente e resta nella sua cartella (`schermo-2.png`, …).
/// Una foto con una nota nuova apre una segnalazione nuova, con la sua cartella.
#[derive(Resource, Default)]
pub struct BugReport {
    /// Cartella dell'ultima segnalazione di questa partita.
    dir: Option<std::path::PathBuf>,
    /// Inquadrature già salvate in quella cartella.
    shots: u32,
    /// La foto e lo stato appena salvati (in attesa della nota), con il loro numero.
    taken: Option<(std::path::PathBuf, std::path::PathBuf, u32)>,
    /// Testo della nota mentre la si scrive.
    pub text: String,
    /// Fotogrammi da aspettare prima di aprire la casella (la foto non deve mostrarla).
    wait: u8,
    /// Dove tornare chiudendo la casella.
    back: Option<crate::AppState>,
    /// File da spostare appena la foto è scritta su disco: (da, a, secondi rimasti).
    moves: Vec<(std::path::PathBuf, std::path::PathBuf, f32)>,
}

/// Il nome dei file di un'inquadratura: la prima è `schermo.png`, poi `schermo-2.png`…
pub fn shot_names(n: u32) -> (String, String) {
    if n <= 1 {
        ("schermo.png".into(), "stato.json".into())
    } else {
        (format!("schermo-{n}.png"), format!("stato-{n}.json"))
    }
}

fn new_report_dir() -> Option<std::path::PathBuf> {
    let (y, m, d, h, mi, s) = crate::localtime::now();
    let dir = crate::ai::tts::data_dir()
        .join("segnalazioni")
        .join(format!("{y:04}-{m:02}-{d:02}_{h:02}-{mi:02}-{s:02}"));
    match std::fs::create_dir_all(&dir) {
        Ok(()) => Some(dir),
        Err(e) => {
            warn!("[BUG] cartella {} non creata: {e}", dir.display());
            None
        }
    }
}

/// Sistema: F12 fa la foto e lo stato, poi apre la casella della nota.
pub fn report_bug(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<crate::AppState>>,
    mut next: ResMut<NextState<crate::AppState>>,
    mut pilot: ResMut<crate::autopilot::Autopilot>,
    mut report: ResMut<BugReport>,
) {
    if report.wait > 0 {
        report.wait -= 1;
        if report.wait == 0 {
            report.back = Some(state.get().clone());
            report.text.clear();
            next.set(crate::AppState::BugNote);
        }
        return;
    }
    if !keys.just_pressed(KeyCode::F12) || *state.get() == crate::AppState::BugNote {
        return;
    }
    // Un'altra inquadratura va nella cartella dell'ultima segnalazione; se poi le si scrive una
    // nota, la si sposta in una segnalazione nuova.
    let (dir, n) = match report.dir.clone() {
        Some(dir) if dir.is_dir() => (dir, report.shots + 1),
        _ => match new_report_dir() {
            Some(dir) => (dir, 1),
            None => return,
        },
    };
    let (shot, dump) = shot_names(n);
    let (shot, dump) = (dir.join(shot), dir.join(dump));
    pilot.photo = Some(shot.clone());
    pilot.dump = Some(Some(dump.clone()));
    if n == 1 {
        write_log_and_note(&dir, "");
    }
    report.dir = Some(dir.clone());
    report.shots = n;
    report.taken = Some((shot, dump, n));
    report.wait = 3;
    info!("[BUG] foto {n} in {}", dir.display());
}

fn write_log_and_note(dir: &std::path::Path, text: &str) {
    let _ = std::fs::write(dir.join("log.txt"), recent_log().join("\n"));
    let _ = std::fs::write(
        dir.join("nota.txt"),
        format!(
            "Versione: {}\n\n{}\n",
            crate::version::GAME_VERSION,
            text.trim()
        ),
    );
}

/// Aggiunge la nota a quella della segnalazione (le inquadrature successive possono precisarla).
fn append_note(dir: &std::path::Path, n: u32, text: &str) {
    let path = dir.join("nota.txt");
    let mut note = std::fs::read_to_string(&path).unwrap_or_default();
    note.push_str(&format!("\nInquadratura {n}: {}\n", text.trim()));
    let _ = std::fs::write(path, note);
}

/// Chiude la casella. Con un testo, la foto appena fatta diventa una segnalazione a sé (se era finita
/// nella cartella della precedente) e la nota si scrive; senza testo resta un'altra inquadratura.
pub fn finish_note(report: &mut BugReport, session: &mut crate::save::Session) {
    let text = report.text.trim().to_string();
    report.text.clear();
    let Some((shot, dump, n)) = report.taken.take() else {
        return;
    };
    let Some(dir) = report.dir.clone() else {
        return;
    };
    if text.is_empty() {
        session.notify(if n == 1 {
            format!("Segnalazione salvata in {}", dir.display())
        } else {
            format!(
                "Inquadratura {n} aggiunta alla segnalazione in {}",
                dir.display()
            )
        });
        return;
    }
    if n == 1 {
        write_log_and_note(&dir, &text);
        session.notify(format!("Segnalazione salvata in {}", dir.display()));
        return;
    }
    // Una nota nuova: la foto appena fatta apre una segnalazione a sé.
    let Some(fresh) = new_report_dir().filter(|d| *d != dir) else {
        append_note(&dir, n, &text);
        session.notify(format!(
            "Nota aggiunta alla segnalazione in {}",
            dir.display()
        ));
        return;
    };
    let (first_shot, first_dump) = shot_names(1);
    report.moves.push((shot, fresh.join(first_shot), MOVE_WAIT));
    report.moves.push((dump, fresh.join(first_dump), MOVE_WAIT));
    write_log_and_note(&fresh, &text);
    report.dir = Some(fresh.clone());
    report.shots = 1;
    session.notify(format!("Segnalazione salvata in {}", fresh.display()));
}

/// Secondi entro cui la foto deve comparire su disco per essere spostata.
const MOVE_WAIT: f32 = 10.0;

/// Sistema: sposta le foto e gli stati appena il gioco li ha scritti.
pub fn move_shots(time: Res<Time<Real>>, mut report: ResMut<BugReport>) {
    if report.moves.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    report.moves.retain_mut(|(from, to, left)| {
        if from.exists() && std::fs::rename(&*from, &*to).is_ok() {
            return false;
        }
        *left -= dt;
        if *left <= 0.0 {
            warn!("[BUG] {} non spostato in {}", from.display(), to.display());
            return false;
        }
        true
    });
}

/// Sistema (egui): la casella della nota, dopo la foto. Invio con Ctrl salva; Esc chiude senza testo.
pub fn bug_note_window(
    mut contexts: bevy_egui::EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    mut report: ResMut<BugReport>,
    mut session: ResMut<crate::save::Session>,
    mut next: ResMut<NextState<crate::AppState>>,
) {
    use bevy_egui::egui;
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let n = report.taken.as_ref().map_or(1, |t| t.2);
    let mut done = keys.just_pressed(KeyCode::Escape);
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter)) {
        done = true;
    }
    let mut clear = false;
    egui::Window::new("Segnala un problema")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(520.0);
            ui.label(if n == 1 {
                "Foto e stato salvati. Scrivi cosa non va:".to_string()
            } else {
                format!(
                    "Inquadratura {n}. Lascia vuoto se è lo stesso problema di prima, \
                     oppure scrivi una nota per una segnalazione nuova:"
                )
            });
            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::multiline(&mut report.text)
                    .desired_rows(5)
                    .desired_width(f32::INFINITY)
                    .hint_text("Cosa è successo, cosa ti aspettavi, se si ripete…"),
            )
            .request_focus();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Salva (Ctrl+Invio)").clicked() {
                    done = true;
                }
                if ui.button("Senza nota (Esc)").clicked() {
                    clear = true;
                    done = true;
                }
            });
        });
    if keys.just_pressed(KeyCode::Escape) {
        clear = true;
    }
    if !done {
        return;
    }
    if clear {
        report.text.clear();
    }
    finish_note(&mut report, &mut session);
    next.set(report.back.take().unwrap_or(crate::AppState::Playing));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_shots_are_numbered() {
        assert_eq!(shot_names(1).0, "schermo.png");
        assert_eq!(
            shot_names(3),
            ("schermo-3.png".into(), "stato-3.json".into())
        );
    }

    #[test]
    fn colours_are_removed() {
        assert_eq!(
            strip_ansi("\u{1b}[2m10:00\u{1b}[0m INFO ok"),
            "10:00 INFO ok"
        );
        assert_eq!(strip_ansi("senza colori"), "senza colori");
    }
}
