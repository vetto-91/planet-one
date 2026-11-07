//! Colonna sonora e impostazioni audio.
//!
//! Due brani in loop (non fanno parte della scena: sopravvivono al reset):
//! quello diurno e quello notturno, che parte al tramonto (18:00) e dura fino
//! a poco prima dell'alba; il passaggio è una dissolvenza incrociata. Il volume
//! delle sole musiche si regola dal
//! menu Esc e viene salvato in `~/.config/planet-one/settings.txt`.

use bevy::audio::Volume;
use bevy::prelude::*;
use std::path::PathBuf;

/// Brano diurno (OGG Vorbis in `assets/`).
const MUSIC_PATH: &str = "audio/music/legacy_of_the_first_light.ogg";
/// Brano notturno: dal tramonto fino al momento prima dell'alba.
const NIGHT_MUSIC_PATH: &str = "audio/music/lanterns_in_the_grove.ogg";
/// Ora di inizio e di fine (esclusa) della musica notturna.
const NIGHT_START: f32 = 18.0;
const NIGHT_END: f32 = 5.9;
/// Durata (s) della dissolvenza incrociata.
const CROSSFADE_SECS: f32 = 4.0;

/// È l'ora della musica notturna?
fn is_night_music(hour: f32) -> bool {
    hour >= NIGHT_START || hour < NIGHT_END
}
/// Volume iniziale delle musiche (0..1).
const DEFAULT_MUSIC_VOLUME: f32 = 0.6;
/// Volume iniziale delle voci (0..1).
const DEFAULT_VOICE_VOLUME: f32 = 0.9;

/// Impostazioni audio (resource).
#[derive(Resource)]
pub struct AudioSettings {
    /// Volume delle musiche, 0..1.
    pub music_volume: f32,
    /// Volume delle voci dei personaggi, 0..1.
    pub voice_volume: f32,
    /// Da salvare su file appena il mouse viene rilasciato.
    pub dirty: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        let mut s = Self {
            music_volume: DEFAULT_MUSIC_VOLUME,
            voice_volume: DEFAULT_VOICE_VOLUME,
            dirty: false,
        };
        if let Some(text) = settings_path().and_then(|p| std::fs::read_to_string(p).ok()) {
            for line in text.lines() {
                if let Some(v) = line.trim().strip_prefix("music_volume=")
                    && let Ok(v) = v.trim().parse::<f32>()
                {
                    s.music_volume = v.clamp(0.0, 1.0);
                }
                if let Some(v) = line.trim().strip_prefix("voice_volume=")
                    && let Ok(v) = v.trim().parse::<f32>()
                {
                    s.voice_volume = v.clamp(0.0, 1.0);
                }
            }
            info!(
                "[AUDIO] impostazioni lette: musica {:.0}%",
                s.music_volume * 100.0
            );
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

/// `$XDG_CONFIG_HOME/planet-one/settings.txt` oppure `~/.config/planet-one/settings.txt`.
fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("planet-one").join("settings.txt"))
}

/// Avvia i due brani in loop: quello dell'ora corrente a pieno volume, l'altro muto.
pub fn start_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<AudioSettings>,
    clock: Res<crate::daynight::GameClock>,
) {
    let night_now = is_night_music(clock.hour);
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
    let night_now = is_night_music(clock.hour);
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
            "music_volume={:.2}\nvoice_volume={:.2}\n",
            settings.music_volume, settings.voice_volume
        ),
    ) {
        Ok(()) => info!("[AUDIO] impostazioni salvate in {}", path.display()),
        Err(e) => warn!("[AUDIO] impossibile salvare le impostazioni: {e}"),
    }
}
