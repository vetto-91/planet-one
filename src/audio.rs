//! Colonna sonora e impostazioni audio.
//!
//! La musica di sottofondo parte all'avvio e gira in loop (non è parte della
//! scena: sopravvive al reset). Il volume delle sole musiche si regola dal
//! menu Esc e viene salvato in `~/.config/planet-one/settings.txt`.

use bevy::audio::Volume;
use bevy::prelude::*;
use std::path::PathBuf;

/// Brano di sottofondo (OGG Vorbis in `assets/`).
const MUSIC_PATH: &str = "audio/music/legacy_of_the_first_light.ogg";
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

/// Marker della traccia musicale.
#[derive(Component)]
pub struct MusicTrack;

/// `$XDG_CONFIG_HOME/planet-one/settings.txt` oppure `~/.config/planet-one/settings.txt`.
fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("planet-one").join("settings.txt"))
}

/// Avvia la musica di sottofondo in loop.
pub fn start_music(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<AudioSettings>,
) {
    commands.spawn((
        MusicTrack,
        AudioPlayer::new(asset_server.load(MUSIC_PATH)),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(settings.music_volume)),
    ));
    info!("[AUDIO] musica avviata: {MUSIC_PATH}");
}

/// Applica il volume delle musiche alla traccia quando cambia.
pub fn apply_music_volume(
    settings: Res<AudioSettings>,
    mut tracks: Query<&mut AudioSink, With<MusicTrack>>,
) {
    // Anche al primo frame in cui il sink compare (traccia appena caricata).
    for mut sink in &mut tracks {
        let target = Volume::Linear(settings.music_volume);
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
