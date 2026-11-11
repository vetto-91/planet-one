//! Colonna sonora e impostazioni audio.
//!
//! Due brani in loop (non fanno parte della scena: sopravvivono al reset):
//! quello diurno e quello notturno, che parte al tramonto (18:00) e dura fino
//! a poco prima dell'alba; il passaggio è una dissolvenza incrociata. Il volume
//! delle sole musiche si regola dal
//! menu Esc e viene salvato in `planet-one/settings.txt` nella cartella di configurazione dell'utente.

use bevy::audio::Volume;
use bevy::prelude::*;
use std::path::PathBuf;

/// Brano diurno (OGG Vorbis in `assets/`).
const MUSIC_PATH: &str = "audio/music/legacy_of_the_first_light.ogg";
/// Brano notturno: dal tramonto fino al momento prima dell'alba.
const NIGHT_MUSIC_PATH: &str = "audio/music/lanterns_in_the_grove.ogg";
/// Durata (s) della dissolvenza incrociata.
const CROSSFADE_SECS: f32 = 4.0;

/// È l'ora della musica notturna? Dal tramonto a poco prima dell'alba (seguono la stagione).
fn is_night_music(clock: &crate::daynight::GameClock) -> bool {
    clock.hour >= clock.sunset || clock.hour < clock.sunrise - 0.1
}
/// Volume iniziale delle musiche (0..1).
const DEFAULT_MUSIC_VOLUME: f32 = 0.6;
const DEFAULT_AMBIENT_VOLUME: f32 = 0.6;
/// Modello e host predefiniti dei dialoghi IA (solo locale).
const DEFAULT_AI_MODEL: &str = "gemma4:e4b";
const DEFAULT_AI_HOST: &str = "http://127.0.0.1:11434";
/// Volume iniziale delle voci (0..1).
const DEFAULT_VOICE_VOLUME: f32 = 0.9;

/// Impostazioni audio (resource).
#[derive(Resource)]
pub struct AudioSettings {
    /// Volume delle musiche, 0..1.
    pub music_volume: f32,
    /// Volume dei suoni d'ambiente, del meteo, delle campane e dei passi, 0..1.
    pub ambient_volume: f32,
    /// Volume delle voci dei personaggi, 0..1.
    pub voice_volume: f32,
    /// Dialoghi con IA locale (Ollama) attivi?
    pub ai_enabled: bool,
    /// Modello Ollama usato per i dialoghi.
    pub ai_model: String,
    /// Indirizzo di Ollama: solo loopback (vedi `ai::ollama::local_base`).
    pub ai_host: String,
    /// Voce IA (Piper locale) attiva?
    pub tts_enabled: bool,
    /// Voce Piper da usare al posto di quella del personaggio (vuoto = automatica).
    pub tts_voice: String,
    /// Microfono nella chat (whisper.cpp locale): spento finché non lo si accende.
    pub stt_enabled: bool,
    /// Modello whisper da usare (vuoto = il primo installato).
    pub stt_model: String,
    /// Velocità del tempo di gioco: 1, 2, 4 o 8.
    pub time_speed: u32,
    /// Sensibilità del mouse (rad per pixel).
    pub mouse_sensitivity: f32,
    /// Contatore degli FPS a schermo.
    pub show_fps: bool,
    pub fullscreen: bool,
    /// Scala dell'interfaccia (0 = automatica dalla risoluzione, vedi `ui_style::ui_scale`).
    pub ui_scale: f32,
    /// Impostazioni grafiche (solo aspetto visivo).
    pub gfx: crate::graphics::GfxOptions,
    /// VSync e limite di FPS.
    pub fps: crate::graphics::FpsOptions,
    /// Da salvare su file appena il mouse viene rilasciato.
    pub dirty: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        let mut s = Self {
            music_volume: DEFAULT_MUSIC_VOLUME,
            ambient_volume: DEFAULT_AMBIENT_VOLUME,
            voice_volume: DEFAULT_VOICE_VOLUME,
            ai_enabled: true,
            ai_model: DEFAULT_AI_MODEL.to_string(),
            ai_host: DEFAULT_AI_HOST.to_string(),
            stt_enabled: false,
            stt_model: String::new(),
            tts_enabled: true,
            tts_voice: String::new(),
            time_speed: 1,
            mouse_sensitivity: 0.003,
            show_fps: true,
            fullscreen: true,
            ui_scale: 0.0,
            gfx: Default::default(),
            fps: Default::default(),
            dirty: false,
        };
        if let Some(text) = settings_path().and_then(|p| std::fs::read_to_string(p).ok()) {
            for line in text.lines() {
                if let Some(v) = line.trim().strip_prefix("music_volume=")
                    && let Ok(v) = v.trim().parse::<f32>()
                {
                    s.music_volume = v.clamp(0.0, 1.0);
                }
                if let Some(v) = line.trim().strip_prefix("ambient_volume=")
                    && let Ok(v) = v.trim().parse::<f32>()
                {
                    s.ambient_volume = v.clamp(0.0, 1.0);
                }
                if let Some(v) = line.trim().strip_prefix("voice_volume=")
                    && let Ok(v) = v.trim().parse::<f32>()
                {
                    s.voice_volume = v.clamp(0.0, 1.0);
                }
                if let Some(v) = line.trim().strip_prefix("ai_enabled=") {
                    s.ai_enabled = v.trim() != "0";
                }
                if let Some(v) = line.trim().strip_prefix("ai_model=")
                    && !v.trim().is_empty()
                {
                    s.ai_model = v.trim().to_string();
                }
                if let Some(v) = line.trim().strip_prefix("tts_enabled=") {
                    s.tts_enabled = v.trim() != "0";
                }
                if let Some(v) = line.trim().strip_prefix("tts_voice=") {
                    s.tts_voice = v.trim().to_string();
                }
                if let Some(v) = line.trim().strip_prefix("stt_enabled=") {
                    s.stt_enabled = v.trim() != "0";
                }
                if let Some(v) = line.trim().strip_prefix("stt_model=") {
                    s.stt_model = v.trim().to_string();
                }
                if let Some(v) = line.trim().strip_prefix("mouse_sensitivity=")
                    && let Ok(v) = v.trim().parse::<f32>()
                {
                    s.mouse_sensitivity = v.clamp(0.0005, 0.01);
                }
                if let Some(v) = line.trim().strip_prefix("show_fps=") {
                    s.show_fps = v.trim() != "0";
                }
                if let Some(v) = line.trim().strip_prefix("fullscreen=") {
                    s.fullscreen = v.trim() != "0";
                }
                if let Some(v) = line.trim().strip_prefix("ui_scale=")
                    && let Ok(v) = v.trim().parse::<f32>()
                {
                    s.ui_scale = if v > 0.0 { v.clamp(0.5, 3.0) } else { 0.0 };
                }
                s.gfx.read_line(line);
                s.fps.read_line(line);
                if let Some(v) = line.trim().strip_prefix("time_speed=")
                    && let Ok(v) = v.trim().parse::<u32>()
                    && [1, 2, 4, 8].contains(&v)
                {
                    s.time_speed = v;
                }
                if let Some(v) = line.trim().strip_prefix("ai_host=")
                    && !v.trim().is_empty()
                {
                    s.ai_host = v.trim().to_string();
                }
            }
            info!(
                "[AUDIO] impostazioni lette: musica {:.0}%",
                s.music_volume * 100.0
            );
        }
        // Prove: `PLANET_ONE_GFX=basso|medio|alto` sceglie la grafica senza toccare il file.
        if let Ok(v) = std::env::var("PLANET_ONE_GFX") {
            match v.to_lowercase().as_str() {
                "basso" | "low" => s.gfx = crate::graphics::GfxOptions::LOW,
                "medio" | "medium" => s.gfx = crate::graphics::GfxOptions::MEDIUM,
                "alto" | "high" => s.gfx = crate::graphics::GfxOptions::HIGH,
                other => warn!("[DEVTOOLS] PLANET_ONE_GFX={other} non valido: basso, medio o alto"),
            }
        }
        s
    }
}

/// Abbassamento temporaneo della musica mentre parla un personaggio (ducking).
#[derive(Resource)]
pub struct MusicDuck {
    /// Moltiplicatore corrente del volume della musica (1 = normale).
    pub level: f32,
}

impl Default for MusicDuck {
    fn default() -> Self {
        Self { level: 1.0 }
    }
}

/// Livello della musica mentre parla un personaggio.
const DUCK_LEVEL: f32 = 0.35;
/// Costante di tempo (s) della dissolvenza del ducking.
const DUCK_TAU: f32 = 0.25;

/// Sistema: porta `MusicDuck.level` verso `DUCK_LEVEL` mentre c'è una voce, altrimenti verso 1.
pub fn update_music_duck(
    time: Res<Time>,
    voice: Res<crate::voice::VoiceQueue>,
    clips: Query<(), With<crate::voice::VoiceClip>>,
    mut duck: ResMut<MusicDuck>,
) {
    let target = if voice.busy() || !clips.is_empty() {
        DUCK_LEVEL
    } else {
        1.0
    };
    let k = 1.0 - (-time.delta_secs() / DUCK_TAU).exp();
    duck.level += (target - duck.level) * k;
}

/// Traccia musicale: `night` = brano notturno; `gain` = dissolvenza corrente (0..1).
#[derive(Component)]
pub struct MusicTrack {
    night: bool,
    gain: f32,
}

/// Cartella di configurazione dell'utente secondo la piattaforma:
/// Windows `%APPDATA%`, macOS `~/Library/Application Support`, Linux `$XDG_CONFIG_HOME` o `~/.config`.
fn config_base() -> Option<PathBuf> {
    // Prove automatiche: impostazioni in una cartella a parte.
    if let Some(dir) = std::env::var_os("PLANET_ONE_DATA_DIR") {
        return Some(PathBuf::from(dir).join("config"));
    }
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }
}

/// `<config>/planet-one/settings.txt` (vedi [`config_base`]).
fn settings_path() -> Option<PathBuf> {
    Some(config_base()?.join("planet-one").join("settings.txt"))
}

/// Avvia i due brani in loop: quello dell'ora corrente a pieno volume, l'altro muto.
pub fn start_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<AudioSettings>,
    clock: Res<crate::daynight::GameClock>,
) {
    let night_now = is_night_music(&clock);
    for (night, path) in [(false, MUSIC_PATH), (true, NIGHT_MUSIC_PATH)] {
        let gain = if night == night_now { 1.0 } else { 0.0 };
        commands.spawn((
            MusicTrack { night, gain },
            AudioPlayer::new(asset_server.load(path)),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(settings.music_volume * gain)),
        ));
        info!("[AUDIO] musica caricata: {path}");
    }
}

/// Dissolvenza incrociata giorno/notte e volume delle musiche (impostazioni e ducking).
pub fn apply_music_volume(
    time: Res<Time>,
    clock: Res<crate::daynight::GameClock>,
    settings: Res<AudioSettings>,
    duck: Res<MusicDuck>,
    mut tracks: Query<(&mut MusicTrack, &mut AudioSink)>,
) {
    let night_now = is_night_music(&clock);
    let step = time.delta_secs() / CROSSFADE_SECS;
    for (mut track, mut sink) in &mut tracks {
        let want = if track.night == night_now { 1.0 } else { 0.0 };
        track.gain = if track.gain < want {
            (track.gain + step).min(want)
        } else {
            (track.gain - step).max(want)
        };
        // Il brano muto si ferma e riparte da dove era (non consuma e non "corre").
        if track.gain <= 0.001 {
            if !sink.is_paused() {
                sink.pause();
            }
        } else if sink.is_paused() {
            sink.play();
        }
        let target = Volume::Linear(settings.music_volume * duck.level * track.gain);
        if sink.volume() != target {
            sink.set_volume(target);
        }
    }
}

/// Salva le impostazioni quando cambiano e il mouse è rilasciato
/// (evita una scrittura per ogni pixel dello slider).
pub fn save_settings(mut settings: ResMut<AudioSettings>, mouse: Res<ButtonInput<MouseButton>>) {
    if !settings.dirty || mouse.pressed(MouseButton::Left) {
        return;
    }
    settings.dirty = false;
    let Some(path) = settings_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match std::fs::write(
        &path,
        format!(
            "music_volume={:.2}\nambient_volume={:.2}\nvoice_volume={:.2}\nai_enabled={}\nai_model={}\nai_host={}\ntts_enabled={}\ntts_voice={}\nstt_enabled={}\nstt_model={}\ntime_speed={}\nmouse_sensitivity={:.5}\nshow_fps={}\nfullscreen={}\nui_scale={:.2}\n{}{}",
            settings.music_volume,
            settings.ambient_volume,
            settings.voice_volume,
            u8::from(settings.ai_enabled),
            settings.ai_model,
            settings.ai_host,
            u8::from(settings.tts_enabled),
            settings.tts_voice,
            u8::from(settings.stt_enabled),
            settings.stt_model,
            settings.time_speed,
            settings.mouse_sensitivity,
            u8::from(settings.show_fps),
            u8::from(settings.fullscreen),
            settings.ui_scale,
            settings.gfx.to_lines(),
            settings.fps.to_lines()
        ),
    ) {
        Ok(()) => info!("[AUDIO] impostazioni salvate in {}", path.display()),
        Err(e) => warn!("[AUDIO] impossibile salvare le impostazioni: {e}"),
    }
}
