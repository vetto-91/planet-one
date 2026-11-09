//! Riconoscimento vocale SOLO LOCALE (ADR 0022): whisper.cpp come programma esterno, nessuna rete.
//!
//! Il microfono è aperto solo mentre si tiene premuto il tasto nella chat. L'audio va in un WAV
//! temporaneo nella cartella dati dell'utente e viene cancellato subito dopo la trascrizione.

use super::AiChat;
use super::tts::{data_dir, pcm_to_wav};
use crate::AppState;
use crate::audio::AudioSettings;
use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream, StreamConfig};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// Frequenza richiesta da whisper e durata massima di una frase.
const WHISPER_RATE: u32 = 16_000;
const MAX_SECONDS: usize = 30;
/// Sotto questa durata (s) non vale la pena trascrivere.
const MIN_SECONDS: f32 = 0.3;
const SCAN_EVERY: f32 = 5.0;

/// Stato del riconoscimento vocale.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum SttStatus {
    /// Spento dal menu.
    #[default]
    Off,
    Ready,
    NoEngine,
    NoModel,
    Recording,
    Transcribing,
    /// Il microfono non si apre: il messaggio dice perché.
    Error(String),
}

impl SttStatus {
    pub fn label(&self) -> String {
        match self {
            SttStatus::Off => "spento".into(),
            SttStatus::Ready => "pronto (locale)".into(),
            SttStatus::NoEngine => "whisper.cpp non installato: setup_ai --stt".into(),
            SttStatus::NoModel => "modello non installato: setup_ai --stt".into(),
            SttStatus::Recording => "ti ascolto…".into(),
            SttStatus::Transcribing => "sto trascrivendo…".into(),
            SttStatus::Error(e) => format!("errore: {e}"),
        }
    }

    /// Si può iniziare a parlare (anche dopo un errore, per riprovare).
    fn can_listen(&self) -> bool {
        matches!(self, SttStatus::Ready | SttStatus::Error(_))
    }
}

enum Request {
    Start,
    Stop {
        id: u64,
        exe: PathBuf,
        model: PathBuf,
    },
    Cancel,
}

enum Event {
    Text(u64, String),
    Failed(String),
}

/// Stato e canali verso la thread che possiede il microfono.
#[derive(Resource)]
pub struct SttState {
    pub status: SttStatus,
    /// Modelli installati (nome del file senza estensione).
    pub models: Vec<String>,
    exe: Option<PathBuf>,
    models_dir: PathBuf,
    tx: mpsc::Sender<Request>,
    rx: Mutex<mpsc::Receiver<Event>>,
    scan_timer: f32,
    listening: bool,
    latest: u64,
}

impl Default for SttState {
    fn default() -> Self {
        let (tx, worker_rx) = mpsc::channel::<Request>();
        let (worker_tx, rx) = mpsc::channel::<Event>();
        std::thread::spawn(move || worker(worker_rx, worker_tx));
        Self {
            status: SttStatus::Off,
            models: Vec::new(),
            exe: None,
            models_dir: stt_dir().join("models"),
            tx,
            rx: Mutex::new(rx),
            scan_timer: 0.0,
            listening: false,
            latest: 0,
        }
    }
}

fn stt_dir() -> PathBuf {
    data_dir().join("ai").join("stt")
}

/// L'audio catturato: dal microfono, o da un WAV di prova (`PLANET_ONE_STT_MIC`).
enum Capture {
    Mic {
        _stream: Stream,
        samples: Arc<Mutex<Vec<f32>>>,
        rate: u32,
        channels: usize,
    },
    File(Vec<f32>),
}

impl Capture {
    /// Audio mono a 16 kHz.
    fn finish(self) -> Vec<f32> {
        match self {
            Capture::Mic {
                samples,
                rate,
                channels,
                ..
            } => {
                let raw = samples.lock().map(|s| s.clone()).unwrap_or_default();
                to_mono_16k(&raw, channels, rate)
            }
            Capture::File(samples) => samples,
        }
    }
}

fn worker(rx: mpsc::Receiver<Request>, tx: mpsc::Sender<Event>) {
    // Il flusso del microfono non si può spostare tra thread: resta qui per tutta la registrazione.
    let mut capture: Option<Capture> = None;
    for req in rx {
        match req {
            Request::Start => match open_capture() {
                Ok(c) => capture = Some(c),
                Err(e) => {
                    capture = None;
                    let _ = tx.send(Event::Failed(e));
                }
            },
            Request::Cancel => capture = None,
            Request::Stop { id, exe, model } => {
                let Some(c) = capture.take() else {
                    continue;
                };
                let audio = c.finish();
                let result = if (audio.len() as f32) < WHISPER_RATE as f32 * MIN_SECONDS {
                    Ok(String::new())
                } else {
                    transcribe(&exe, &model, &audio)
                };
                let _ = tx.send(match result {
                    Ok(text) => Event::Text(id, text),
                    Err(e) => Event::Failed(e),
                });
            }
        }
    }
}

fn open_capture() -> Result<Capture, String> {
    if let Ok(path) = std::env::var("PLANET_ONE_STT_MIC") {
        return read_test_wav(Path::new(&path)).map(Capture::File);
    }
    let device = cpal::default_host()
        .default_input_device()
        .ok_or("nessun microfono trovato")?;
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let config: StreamConfig = supported.clone().into();
    let samples = Arc::new(Mutex::new(Vec::new()));
    let limit = config.sample_rate as usize * config.channels as usize * MAX_SECONDS;
    let stream = match supported.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(&device, &config, &samples, limit),
        SampleFormat::I16 => build_stream::<i16>(&device, &config, &samples, limit),
        SampleFormat::U16 => build_stream::<u16>(&device, &config, &samples, limit),
        SampleFormat::I32 => build_stream::<i32>(&device, &config, &samples, limit),
        other => Err(format!("formato audio non gestito: {other}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(Capture::Mic {
        _stream: stream,
        samples,
        rate: config.sample_rate,
        channels: config.channels as usize,
    })
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    samples: &Arc<Mutex<Vec<f32>>>,
    limit: usize,
) -> Result<Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let sink = samples.clone();
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                if let Ok(mut out) = sink.lock()
                    && out.len() < limit
                {
                    out.extend(data.iter().map(|s| f32::from_sample_(*s)));
                }
            },
            |e| warn!("[STT] errore del microfono: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

/// Riduce a un canale e porta a 16 kHz (interpolazione lineare: basta per la voce).
pub fn to_mono_16k(raw: &[f32], channels: usize, rate: u32) -> Vec<f32> {
    let channels = channels.max(1);
    let mono: Vec<f32> = raw
        .chunks_exact(channels)
        .map(|f| f.iter().sum::<f32>() / channels as f32)
        .collect();
    if rate == WHISPER_RATE || mono.len() < 2 {
        return mono;
    }
    let step = rate as f32 / WHISPER_RATE as f32;
    let n = ((mono.len() - 1) as f32 / step) as usize;
    (0..n)
        .map(|i| {
            let pos = i as f32 * step;
            let k = pos as usize;
            let t = pos - k as f32;
            mono[k] * (1.0 - t) + mono[k + 1] * t
        })
        .collect()
}

/// PCM 16 bit dai campioni in virgola mobile.
fn to_pcm16(samples: &[f32]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
        .collect()
}

/// Solo per le prove automatiche: WAV PCM 16 bit, mono, 16 kHz.
fn read_test_wav(path: &Path) -> Result<Vec<f32>, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let start = data
        .windows(4)
        .position(|w| w == b"data")
        .ok_or("WAV di prova non valido")?
        + 8;
    Ok(data[start..]
        .chunks_exact(2)
        .map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0)
        .collect())
}

/// Toglie le annotazioni di whisper (`[BLANK_AUDIO]`, `(musica)`) e unisce le righe.
pub fn clean_transcript(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for c in text.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn transcribe(exe: &Path, model: &Path, audio: &[f32]) -> Result<String, String> {
    let wav = stt_dir().join(format!("registrazione-{}.wav", std::process::id()));
    std::fs::write(&wav, pcm_to_wav(&to_pcm16(audio), WHISPER_RATE)).map_err(|e| e.to_string())?;
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get().min(6))
        .to_string();
    let output = Command::new(exe)
        .arg("-m")
        .arg(model)
        .arg("-f")
        .arg(&wav)
        .args(["-l", "it", "-nt", "-np", "-t", &threads])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    // La voce non resta su disco, qualunque sia l'esito.
    let _ = std::fs::remove_file(&wav);
    let output = output.map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("whisper.cpp ha restituito un errore".into());
    }
    Ok(clean_transcript(&String::from_utf8_lossy(&output.stdout)))
}

fn find_engine() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "whisper-cli.exe"
    } else {
        "whisper-cli"
    };
    let path = stt_dir().join(name);
    path.is_file().then_some(path)
}

fn list_models(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            p.extension()
                .is_some_and(|x| x == "bin")
                .then(|| p.file_stem()?.to_str().map(str::to_string))
                .flatten()
        })
        .collect();
    v.sort();
    v
}

/// Sistema: cerca motore e modelli a intervalli.
pub fn stt_scan(time: Res<Time>, settings: Res<AudioSettings>, mut stt: ResMut<SttState>) {
    if !settings.stt_enabled {
        if stt.status != SttStatus::Off {
            stt.status = SttStatus::Off;
        }
        stt.scan_timer = 0.0;
        return;
    }
    if matches!(stt.status, SttStatus::Recording | SttStatus::Transcribing) {
        return;
    }
    stt.scan_timer -= time.delta_secs();
    if stt.scan_timer > 0.0 && stt.status != SttStatus::Off {
        return;
    }
    stt.scan_timer = SCAN_EVERY;
    stt.exe = find_engine();
    stt.models = list_models(&stt.models_dir);
    let found = match (&stt.exe, stt.models.is_empty()) {
        (None, _) => SttStatus::NoEngine,
        (Some(_), true) => SttStatus::NoModel,
        (Some(_), false) => SttStatus::Ready,
    };
    // Un errore del microfono resta a video finché non si riprova.
    if !matches!(stt.status, SttStatus::Error(_)) || found != SttStatus::Ready {
        stt.status = found;
    }
}

/// Sistema: nella chat, F2 tenuto premuto (o il pulsante «Parla») apre il microfono.
pub fn stt_push_to_talk(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    settings: Res<AudioSettings>,
    chat: Res<AiChat>,
    mut stt: ResMut<SttState>,
) {
    let in_chat = *state.get() == AppState::Chat;
    let held = in_chat && (keys.pressed(KeyCode::F2) || chat.mic_held);
    if held && !stt.listening && stt.status.can_listen() && !chat.thinking {
        stt.listening = true;
        stt.status = SttStatus::Recording;
        let _ = stt.tx.send(Request::Start);
        info!("[STT] microfono aperto");
    } else if !held && stt.listening {
        stt.listening = false;
        info!("[STT] microfono chiuso");
        let target = stt.exe.clone().zip(chosen_model(&stt, &settings));
        match target.filter(|_| in_chat) {
            Some((exe, model)) => {
                stt.latest += 1;
                let id = stt.latest;
                stt.status = SttStatus::Transcribing;
                let _ = stt.tx.send(Request::Stop { id, exe, model });
            }
            None => {
                stt.latest += 1;
                stt.status = SttStatus::Ready;
                let _ = stt.tx.send(Request::Cancel);
            }
        }
    }
}

/// Percorso del modello scelto in impostazioni, oppure del primo installato.
fn chosen_model(stt: &SttState, settings: &AudioSettings) -> Option<PathBuf> {
    let name = stt
        .models
        .iter()
        .find(|m| **m == settings.stt_model)
        .or_else(|| stt.models.first())?;
    Some(stt.models_dir.join(format!("{name}.bin")))
}

/// Sistema: il testo riconosciuto finisce nella casella della chat, da rileggere prima di inviarlo.
pub fn stt_pump(mut stt: ResMut<SttState>, mut chat: ResMut<AiChat>) {
    let events: Vec<Event> = {
        let Ok(rx) = stt.rx.lock() else {
            return;
        };
        rx.try_iter().collect()
    };
    for ev in events {
        match ev {
            Event::Text(id, text) => {
                if id != stt.latest {
                    continue;
                }
                stt.status = SttStatus::Ready;
                info!("[STT] trascrizione: {} caratteri", text.chars().count());
                if !text.is_empty() {
                    if !chat.input.trim().is_empty() {
                        chat.input.push(' ');
                    }
                    chat.input.push_str(&text);
                    chat.focus_input = true;
                }
            }
            Event::Failed(e) => {
                warn!("[STT] {e}");
                stt.listening = false;
                stt.status = SttStatus::Error(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_48k_becomes_mono_16k() {
        let raw: Vec<f32> = (0..4800).flat_map(|_| [0.5f32, -0.5]).collect();
        let out = to_mono_16k(&raw, 2, 48_000);
        assert!((1590..=1600).contains(&out.len()), "{}", out.len());
        assert!(out.iter().all(|s| s.abs() < 1e-5));
    }

    #[test]
    fn already_16k_mono_is_untouched() {
        let raw = [0.1f32, 0.2, 0.3];
        assert_eq!(to_mono_16k(&raw, 1, WHISPER_RATE), raw);
    }

    #[test]
    fn pcm_clamps_out_of_range_samples() {
        assert_eq!(to_pcm16(&[2.0]), 32767i16.to_le_bytes());
        assert_eq!(to_pcm16(&[-2.0]), (-32767i16).to_le_bytes());
    }

    #[test]
    fn annotations_are_removed_from_the_transcript() {
        assert_eq!(clean_transcript(" [BLANK_AUDIO]\n"), "");
        assert_eq!(
            clean_transcript(" Buongiorno (musica)\n a tutti.\n"),
            "Buongiorno a tutti."
        );
    }
}
