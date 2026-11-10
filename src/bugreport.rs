//! Tasto F12 «Segnala bug»: salva in una cartella lo screenshot, lo stato del gioco in JSON
//! (come `dump` del pilota automatico), le ultime righe del log e una nota da compilare.
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

/// Sistema: F12 salva una segnalazione (screenshot, stato, log, nota).
pub fn report_bug(
    keys: Res<ButtonInput<KeyCode>>,
    mut pilot: ResMut<crate::autopilot::Autopilot>,
    mut session: ResMut<crate::save::Session>,
) {
    if !keys.just_pressed(KeyCode::F12) {
        return;
    }
    let (y, m, d, h, mi, s) = crate::localtime::now();
    let dir = crate::ai::tts::data_dir()
        .join("segnalazioni")
        .join(format!("{y:04}-{m:02}-{d:02}_{h:02}-{mi:02}-{s:02}"));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        warn!("[BUG] cartella {} non creata: {e}", dir.display());
        return;
    }
    pilot.photo = Some(dir.join("schermo.png"));
    pilot.dump = Some(Some(dir.join("stato.json")));
    let _ = std::fs::write(dir.join("log.txt"), recent_log().join("\n"));
    let _ = std::fs::write(
        dir.join("nota.txt"),
        format!(
            "Versione: {}\n\
             Cosa è successo:\n\n\
             Cosa ti aspettavi:\n\n\
             Si ripete? (sì / no / a volte):\n\n\
             Cosa stavi facendo prima:\n",
            crate::version::GAME_VERSION
        ),
    );
    info!("[BUG] segnalazione salvata in {}", dir.display());
    session.notify(format!(
        "Segnalazione salvata (scrivi la nota in nota.txt): {}",
        dir.display()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_are_removed() {
        assert_eq!(
            strip_ansi("\u{1b}[2m10:00\u{1b}[0m INFO ok"),
            "10:00 INFO ok"
        );
        assert_eq!(strip_ansi("senza colori"), "senza colori");
    }
}
