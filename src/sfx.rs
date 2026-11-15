//! Piccoli suoni del gioco, sintetizzati in locale (ADR 0050): l'incantesimo lanciato, l'anello della
//! Prova dell'Arco centrato, il perno della serratura e la serratura che cede.
//!
//! Chi li fa suonare chiama [`play`]; un sistema li mette in onda al frame dopo, col volume
//! dell'ambiente.

use std::f32::consts::TAU;
use std::sync::Mutex;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::weather::{RATE, to_wav};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    Spell,
    Ring,
    Pin,
    Unlock,
    /// Lastra di pietra che scorre (la Porta Sommersa).
    Grind,
}

static QUEUE: Mutex<Vec<Sfx>> = Mutex::new(Vec::new());

/// Fa suonare un suono al prossimo frame.
pub fn play(sfx: Sfx) {
    if let Ok(mut q) = QUEUE.lock() {
        q.push(sfx);
    }
}

/// Incantesimo: un accordo di tre seni che sale e si spegne in un leggero scintillio.
pub fn synth_spell() -> Vec<u8> {
    let n = (RATE as f32 * 0.9) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let env = (t * 30.0).min(1.0) * (-t * 3.2).exp();
            let glide = 1.0 + 0.25 * t;
            let chord: f32 = [523.0f32, 659.0, 784.0]
                .iter()
                .map(|f| (TAU * f * glide * t).sin())
                .sum();
            let sparkle = (TAU * 2093.0 * t).sin() * (t * 7.0).sin().abs() * 0.3;
            (chord / 3.0 + sparkle) * env
        })
        .collect::<Vec<_>>()
        .pipe(|s| to_wav(&s, 0.55))
}

/// Anello di ferro colpito: un «ting» metallico con armoniche non intere che vibrano a lungo.
pub fn synth_ring() -> Vec<u8> {
    let n = (RATE as f32 * 1.4) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let env = (t * 400.0).min(1.0);
            [
                (880.0f32, 1.0f32, 2.2f32),
                (2420.0, 0.5, 4.0),
                (4370.0, 0.25, 7.0),
            ]
            .iter()
            .map(|(f, a, d)| (TAU * f * t).sin() * a * (-t * d).exp())
            .sum::<f32>()
                * env
        })
        .collect::<Vec<_>>()
        .pipe(|s| to_wav(&s, 0.5))
}

/// Perno della serratura: un clic secco e breve.
pub fn synth_pin() -> Vec<u8> {
    let n = (RATE as f32 * 0.08) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            (TAU * 3200.0 * t).sin() * (-t * 120.0).exp()
                + (TAU * 1100.0 * t).sin() * (-t * 60.0).exp() * 0.5
        })
        .collect::<Vec<_>>()
        .pipe(|s| to_wav(&s, 0.6))
}

/// Serratura che cede: due scatti e un tonfo sordo del coperchio.
pub fn synth_unlock() -> Vec<u8> {
    let n = (RATE as f32 * 0.6) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let click = |t0: f32| {
                let d = t - t0;
                if d < 0.0 {
                    0.0
                } else {
                    (TAU * 2600.0 * d).sin() * (-d * 90.0).exp()
                }
            };
            let thud = if t > 0.22 {
                let d = t - 0.22;
                (TAU * 140.0 * d).sin() * (-d * 14.0).exp() * 0.9
            } else {
                0.0
            };
            click(0.0) + click(0.11) * 0.8 + thud
        })
        .collect::<Vec<_>>()
        .pipe(|s| to_wav(&s, 0.6))
}

/// Pietra che scorre: un rombo basso e granuloso che cresce e si spegne in un tonfo.
pub fn synth_grind() -> Vec<u8> {
    let n = (RATE as f32 * 1.8) as usize;
    let mut seed: u32 = 0x9E37_79B9;
    let mut low = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let noise = seed as f32 / u32::MAX as f32 * 2.0 - 1.0;
            low += (noise - low) * 0.04; // rumore filtrato: sabbia e pietra
            let env = (t * 3.0).min(1.0) * (1.0 - (t - 1.3).max(0.0) * 2.0).max(0.0);
            let rumble = (TAU * 48.0 * t).sin() * 0.5 + (TAU * 71.0 * t).sin() * 0.3;
            let thud = if t > 1.35 {
                let d = t - 1.35;
                (TAU * 90.0 * d).sin() * (-d * 10.0).exp()
            } else {
                0.0
            };
            (low * 3.0 + rumble * 0.4) * env + thud
        })
        .collect::<Vec<_>>()
        .pipe(|s| to_wav(&s, 0.7))
}

/// Ronzio della Luce Viva: un'ora di elettricità in un secondo che si ripete senza scatti (100 Hz e
/// armoniche, con un battito lento come un polso).
pub fn synth_hum() -> Vec<u8> {
    let n = RATE as usize; // un secondo esatto: 100 Hz chiude il giro, nessuno scatto al ripetersi
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let pulse = 0.75 + 0.25 * (TAU * 1.0 * t).sin();
            let tone = (TAU * 100.0 * t).sin() * 0.5
                + (TAU * 200.0 * t).sin() * 0.3
                + (TAU * 300.0 * t).sin() * 0.12
                + (TAU * 1000.0 * t).sin() * 0.04;
            tone * pulse * 0.6
        })
        .collect::<Vec<_>>()
        .pipe(|s| to_wav(&s, 0.5))
}

/// Scrive in modo scorrevole `x.pipe(f)` invece di `f(x)`.
trait Pipe: Sized {
    fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}

impl SfxSounds {
    /// Il ronzio del lampione azzurro (EC-07), da far suonare in loop dove serve.
    pub fn hum(&self) -> Handle<AudioSource> {
        self.0[5].clone()
    }
}

/// I suoni pronti.
#[derive(Resource)]
pub struct SfxSounds([Handle<AudioSource>; 6]);

fn setup(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let mut add = |bytes: Vec<u8>| {
        sources.add(AudioSource {
            bytes: bytes.into(),
        })
    };
    commands.insert_resource(SfxSounds([
        add(synth_spell()),
        add(synth_ring()),
        add(synth_pin()),
        add(synth_unlock()),
        add(synth_grind()),
        add(synth_hum()),
    ]));
}

/// Sistema: mette in onda i suoni chiesti nel frame.
pub fn play_queued(
    mut commands: Commands,
    sounds: Option<Res<SfxSounds>>,
    settings: Res<crate::audio::AudioSettings>,
) {
    let pending = QUEUE
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default();
    let Some(sounds) = sounds else {
        return;
    };
    for sfx in pending {
        let (handle, volume) = match sfx {
            Sfx::Spell => (&sounds.0[0], 0.7),
            Sfx::Ring => (&sounds.0[1], 0.8),
            Sfx::Pin => (&sounds.0[2], 0.6),
            Sfx::Unlock => (&sounds.0[3], 0.8),
            Sfx::Grind => (&sounds.0[4], 0.9),
        };
        commands.spawn((
            AudioPlayer::new(handle.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(settings.ambient_volume * volume)),
        ));
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, play_queued);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_is_a_short_wav() {
        for bytes in [
            synth_spell(),
            synth_ring(),
            synth_pin(),
            synth_unlock(),
            synth_grind(),
            synth_hum(),
        ] {
            assert_eq!(&bytes[..4], b"RIFF");
            let seconds = (bytes.len() - 44) as f32 / (RATE as f32 * 2.0);
            assert!(seconds > 0.05 && seconds < 2.0, "{seconds}");
        }
    }
}
