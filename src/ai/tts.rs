//! Voce dei personaggi con TTS **solo locale** (Piper, programma esterno).
//!
//! Vincolo del progetto: tutto gira sul PC dell'utente. Piper non usa la rete: il testo va
//! sul suo stdin e torna audio PCM sul stdout, che qui diventa un WAV in memoria (nessun
//! file temporaneo) e viene riprodotto come una voce qualsiasi (ducking della musica incluso).
//! Programma e voci si installano con `scripts/setup_ai.sh --tts` (una tantum, con consenso).

use crate::audio::AudioSettings;
use crate::voice::VoiceClip;
use bevy::audio::{AudioSource, Volume};
use bevy::prelude::*;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

/// Guadagno rispetto allo slider delle voci (come i clip registrati).
const VOICE_GAIN: f32 = 1.4;
/// Tempo massimo (s) di una sintesi.
const SYNTH_TIMEOUT: u64 = 30;
/// Ogni quanti secondi si ricerca Piper e le voci.
const SCAN_EVERY: f32 = 15.0;

/// Stato del motore vocale.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum TtsStatus {
    #[default]
    Off,
    Ready,
    NoPiper,
    NoVoice,
}

impl TtsStatus {
    pub fn label(&self) -> &'static str {
        match self {
            TtsStatus::Off => "spenta",
            TtsStatus::Ready => "pronta (locale)",
            TtsStatus::NoPiper => "Piper non trovato: ./scripts/setup_ai.sh --tts",
            TtsStatus::NoVoice => "nessuna voce italiana: ./scripts/setup_ai.sh --tts",
        }
    }
}

/// Marker della voce IA in riproduzione.
#[derive(Component)]
pub struct AiVoice;

enum Request {
    Speak {
        id: u64,
        exe: PathBuf,
        model: PathBuf,
        length_scale: f32,
        text: String,
    },
}

enum Event {
    Audio(u64, Vec<u8>),
    Failed(u64, String),
}

/// Stato del TTS e canali verso la thread worker.
#[derive(Resource)]
pub struct TtsState {
    pub status: TtsStatus,
    /// Nomi delle voci installate (file `.onnx` senza estensione).
    pub voices: Vec<String>,
    exe: Option<PathBuf>,
    voices_dir: PathBuf,
    tx: mpsc::Sender<Request>,
    rx: Mutex<mpsc::Receiver<Event>>,
    latest: u64,
    stop: bool,
    scan_timer: f32,
    mock: bool,
}

impl Default for TtsState {
    fn default() -> Self {
        let mock = std::env::var("PLANET_ONE_TTS_MOCK").is_ok_and(|v| v != "0");
        let (tx, worker_rx) = mpsc::channel::<Request>();
        let (worker_tx, rx) = mpsc::channel::<Event>();
        std::thread::spawn(move || worker(worker_rx, worker_tx, mock));
        if mock {
            info!("[TTS] modalità MOCK: tono di prova, nessun Piper");
        }
        Self {
            status: TtsStatus::Off,
            voices: Vec::new(),
            exe: None,
            voices_dir: data_dir().join("ai").join("voices"),
            tx,
            rx: Mutex::new(rx),
            latest: 0,
            stop: false,
            scan_timer: 0.0,
            mock,
        }
    }
}

impl TtsState {
    /// Legge `text` con la voce `voice` (nome del file `.onnx`; vuoto = la prima installata).
    pub fn speak(&mut self, text: &str, voice: &str, speed: f32) {
        if self.status != TtsStatus::Ready {
            return;
        }
        self.latest += 1;
        self.stop = true; // la voce precedente si interrompe
        let name = if self.voices.iter().any(|v| v == voice) {
            voice.to_string()
        } else {
            self.voices.first().cloned().unwrap_or_default()
        };
        let _ = self.tx.send(Request::Speak {
            id: self.latest,
            exe: self.exe.clone().unwrap_or_default(),
            model: self.voices_dir.join(format!("{name}.onnx")),
            length_scale: speed.clamp(0.5, 2.0),
            text: text.to_string(),
        });
    }

    /// Ferma la voce in corso e scarta quelle in arrivo.
    pub fn stop(&mut self) {
        self.latest += 1;
        self.stop = true;
    }
}

/// Cartella dati dell'utente per piattaforma (Linux `~/.local/share`, macOS `~/Library/Application
/// Support`, Windows `%LOCALAPPDATA%`) + `planet-one`.
pub fn data_dir() -> PathBuf {
    // Prove automatiche: salvataggi e dati in una cartella a parte.
    if let Some(dir) = std::env::var_os("PLANET_ONE_DATA_DIR") {
        return PathBuf::from(dir).join("data");
    }
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("planet-one")
}

/// Cerca l'eseguibile di Piper: `PLANET_ONE_PIPER`, cartella dati, poi `PATH`.
fn find_piper() -> Option<PathBuf> {
    let exe = if cfg!(target_os = "windows") {
        "piper.exe"
    } else {
        "piper"
    };
    if let Some(p) = std::env::var_os("PLANET_ONE_PIPER").map(PathBuf::from)
        && p.is_file()
    {
        return Some(p);
    }
    let dir = data_dir().join("ai").join("piper");
    for c in [dir.join("piper").join(exe), dir.join(exe)] {
        if c.is_file() {
            return Some(c);
        }
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(exe))
            .find(|c| c.is_file())
    })
}

fn list_voices(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            (p.extension().is_some_and(|x| x == "onnx") && p.with_extension("onnx.json").exists())
                .then(|| p.file_stem()?.to_str().map(str::to_string))
                .flatten()
        })
        .collect();
    v.sort();
    v
}

/// Avvolge PCM 16 bit mono in un WAV (intestazione RIFF da 44 byte).
pub fn pcm_to_wav(pcm: &[u8], sample_rate: u32) -> Vec<u8> {
    let mut w = Vec::with_capacity(44 + pcm.len());
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // mono
    w.extend_from_slice(&sample_rate.to_le_bytes());
    w.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    w.extend_from_slice(pcm);
    w
}

/// Frequenza di campionamento dalla configurazione della voce (`<voce>.onnx.json`).
fn sample_rate_of(model: &Path) -> u32 {
    let cfg = model.with_extension("onnx.json");
    std::fs::read_to_string(cfg)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v["audio"]["sample_rate"].as_u64())
        .map_or(22_050, |r| r as u32)
}

/// Tono di prova (modalità MOCK): 0,8 s a 440 Hz con inviluppo.
fn mock_wav() -> Vec<u8> {
    let rate = 22_050u32;
    let n = (rate as f32 * 0.8) as usize;
    let mut pcm = Vec::with_capacity(n * 2);
    for i in 0..n {
        let t = i as f32 / rate as f32;
        let env = (t * 20.0).min(1.0) * (1.0 - t / 0.8).max(0.0);
        let s = (t * 440.0 * std::f32::consts::TAU).sin() * 0.25 * env;
        pcm.extend_from_slice(&((s * i16::MAX as f32) as i16).to_le_bytes());
    }
    pcm_to_wav(&pcm, rate)
}

/// Esegue Piper (processo locale, senza rete) e restituisce il WAV.
fn synthesize(exe: &Path, model: &Path, length_scale: f32, text: &str) -> Result<Vec<u8>, String> {
    let mut child = Command::new(exe)
        .arg("--model")
        .arg(model)
        .arg("--output-raw")
        .arg("--length_scale")
        .arg(format!("{length_scale:.2}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("avvio di Piper fallito: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
        let _ = stdin.write_all(b"\n");
    } // stdin chiuso: Piper finisce
    let mut stdout = child.stdout.take().ok_or("niente stdout")?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > Duration::from_secs(SYNTH_TIMEOUT) => {
                let _ = child.kill();
                return Err("Piper troppo lento".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let pcm = reader.join().map_err(|_| "lettura fallita")?;
    if pcm.len() < 200 {
        return Err("Piper non ha prodotto audio".into());
    }
    Ok(pcm_to_wav(&pcm, sample_rate_of(model)))
}

fn worker(rx: mpsc::Receiver<Request>, tx: mpsc::Sender<Event>, mock: bool) {
    while let Ok(first) = rx.recv() {
        // Se nel frattempo ne sono arrivate altre, conta solo l'ultima.
        let mut req = first;
        while let Ok(next) = rx.try_recv() {
            req = next;
        }
        let Request::Speak {
            id,
            exe,
            model,
            length_scale,
            text,
        } = req;
        let r = if mock {
            std::thread::sleep(Duration::from_millis(300));
            Ok(mock_wav())
        } else {
            synthesize(&exe, &model, length_scale, &text)
        };
        let _ = tx.send(match r {
            Ok(wav) => Event::Audio(id, wav),
            Err(e) => Event::Failed(id, e),
        });
    }
}

/// Sistema: ricerca Piper e le voci installate (poco costosa, ogni 15 s).
pub fn tts_scan(time: Res<Time>, settings: Res<AudioSettings>, mut tts: ResMut<TtsState>) {
    if !settings.ai_enabled || !settings.tts_enabled {
        tts.status = TtsStatus::Off;
        tts.scan_timer = 0.0;
        return;
    }
    tts.scan_timer -= time.delta_secs();
    if tts.scan_timer > 0.0 && tts.status != TtsStatus::Off {
        return;
    }
    tts.scan_timer = SCAN_EVERY;
    if tts.mock {
        tts.voices = vec!["mock".into()];
        tts.status = TtsStatus::Ready;
        return;
    }
    tts.exe = find_piper();
    tts.voices = list_voices(&tts.voices_dir);
    tts.status = match (&tts.exe, tts.voices.is_empty()) {
        (None, _) => TtsStatus::NoPiper,
        (Some(_), true) => TtsStatus::NoVoice,
        (Some(_), false) => TtsStatus::Ready,
    };
}

/// Sistema: riceve l'audio sintetizzato e lo riproduce; ferma la voce quando richiesto.
pub fn tts_pump(
    mut commands: Commands,
    mut tts: ResMut<TtsState>,
    mut sources: ResMut<Assets<AudioSource>>,
    settings: Res<AudioSettings>,
    playing: Query<Entity, With<AiVoice>>,
) {
    if std::mem::take(&mut tts.stop) {
        for e in &playing {
            commands.entity(e).despawn();
        }
    }
    let events: Vec<Event> = {
        let Ok(rx) = tts.rx.lock() else {
            return;
        };
        rx.try_iter().collect()
    };
    for ev in events {
        match ev {
            Event::Audio(id, wav) if id == tts.latest => {
                for e in &playing {
                    commands.entity(e).despawn();
                }
                let handle = sources.add(AudioSource { bytes: wav.into() });
                commands.spawn((
                    AiVoice,
                    VoiceClip,
                    AudioPlayer::new(handle),
                    PlaybackSettings::DESPAWN
                        .with_volume(Volume::Linear(settings.voice_volume * VOICE_GAIN)),
                ));
                info!("[TTS] voce avviata");
            }
            Event::Failed(id, e) if id == tts.latest => warn!("[TTS] errore: {e}"),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_is_valid() {
        let w = pcm_to_wav(&[0u8; 100], 22_050);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes(w[4..8].try_into().unwrap()), 136);
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), 22_050);
        assert_eq!(u32::from_le_bytes(w[40..44].try_into().unwrap()), 100);
        assert_eq!(w.len(), 144);
    }

    #[test]
    fn mock_tone_is_not_silent() {
        let w = mock_wav();
        assert!(w[44..].iter().any(|b| *b != 0));
    }
}
