//! Campana della chiesa: suono sintetizzato in locale (nessun file audio) e rintocchi alle ore.
//!
//! Dalle 7 alle 21, a ogni ora piena la campana batte tante volte quante sono le ore (1–12).
//! Il volume dipende dalla distanza del giocatore dal campanile (si sente in tutto il villaggio e
//! oltre le mura, più piano). Il timbro è una somma di parziali non armonici che decadono.

use crate::audio::AudioSettings;
use crate::daynight::GameClock;
use crate::debug_place::DebugPlace;
use crate::player::Player;
use bevy::audio::{AudioSinkPlayback, AudioSource, SpatialAudioSink, Volume};
use bevy::prelude::*;

/// Distanza (m) oltre cui la campana non si sente.
const HEARING_RANGE: f32 = 190.0;
/// Secondi reali tra un rintocco e il successivo.
const STRIKE_GAP: f32 = 3.0;
/// Fra queste ore (incluse) la campana suona.
/// Rintocchi pieni solo a mezzogiorno e a mezzanotte; un colpo solo all'alba e al tramonto.
const FULL_HOURS: [i32; 2] = [0, 12];
/// Posizione del campanile rispetto al centro della chiesa (locale: davanti = +Z).
const BELFRY_LOCAL: Vec3 = Vec3::new(0.0, 9.3, 5.2);

#[derive(Resource)]
pub struct BellSound(Handle<AudioSource>);

/// Punto da cui suona un rintocco.
#[derive(Component)]
pub struct BellStrike(Vec3);

/// Sintetizza un rintocco di campana (WAV mono 16 bit, 5 s).
pub fn synth_bell() -> Vec<u8> {
    let rate = 22_050u32;
    let n = (rate as f32 * 5.0) as usize;
    let base = 330.0f32;
    // (rapporto, ampiezza, decadimento 1/s): hum, prima, terza minore, quinta, nominale, ...
    let partials = [
        (0.5, 0.5, 0.45),
        (1.0, 0.9, 0.7),
        (1.19, 0.6, 0.9),
        (1.5, 0.35, 1.1),
        (2.0, 0.5, 1.3),
        (2.51, 0.25, 1.7),
        (3.0, 0.18, 2.1),
        (4.2, 0.1, 2.9),
    ];
    let mut samples = vec![0.0f32; n];
    for (ratio, amp, decay) in partials {
        for (detune, a) in [(0.0, 1.0), (0.004, 0.6)] {
            let f = base * ratio * (1.0 + detune);
            for (i, s) in samples.iter_mut().enumerate() {
                let t = i as f32 / rate as f32;
                *s += (t * f * std::f32::consts::TAU).sin() * amp * a * (-t * decay).exp();
            }
        }
    }
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-3);
    let mut pcm = Vec::with_capacity(n * 2);
    for (i, s) in samples.iter().enumerate() {
        let t = i as f32 / rate as f32;
        let attack = (t * 400.0).min(1.0);
        let v = s / peak * 0.8 * attack;
        pcm.extend_from_slice(&((v * i16::MAX as f32) as i16).to_le_bytes());
    }
    crate::ai::tts::pcm_to_wav(&pcm, rate)
}

fn make_bell(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let handle = sources.add(AudioSource {
        bytes: synth_bell().into(),
    });
    commands.insert_resource(BellSound(handle));
}

#[derive(Default)]
pub struct RingState {
    last_hour: Option<i32>,
    remaining: u32,
    timer: f32,
}

/// Quanti rintocchi suonano in questo istante: a mezzogiorno e mezzanotte 12, poco prima
/// dell'alba e del tramonto uno solo (all'ora piena che precede il sorgere o il calare del sole).
pub fn strikes_at(hour: i32, sunrise: f32, sunset: f32) -> u32 {
    if FULL_HOURS.contains(&hour) {
        return 12;
    }
    let before = |t: f32| hour == t.floor() as i32 - 1 || (t < 1.0 && hour == 23);
    if before(sunrise) || before(sunset) {
        1
    } else {
        0
    }
}

/// Sistema: avvia e scandisce i rintocchi alle ore piene.
pub fn ring_bells(
    mut commands: Commands,
    time: Res<Time>,
    clock: Res<GameClock>,
    sound: Option<Res<BellSound>>,
    dbg: Res<DebugPlace>,
    mut st: Local<RingState>,
) {
    let Some(sound) = sound else {
        return;
    };
    let hour = clock.hour.floor() as i32;
    if st.last_hour != Some(hour) {
        let strikes = strikes_at(hour, clock.sunrise, clock.sunset);
        if st.last_hour.is_some() && strikes > 0 {
            st.remaining = strikes;
            st.timer = 0.0;
            info!("[CAMPANE] ore {hour}: {} rintocchi", st.remaining);
        }
        st.last_hour = Some(hour);
    }
    if st.remaining == 0 {
        return;
    }
    st.timer -= time.delta_secs();
    if st.timer > 0.0 {
        return;
    }
    // Il campanile è quello della prima chiesa piazzata.
    let Some(church) = dbg.items.iter().find(|p| p.kind == "chiesa") else {
        st.remaining = 0;
        return;
    };
    let Some(&p) = church.points.first() else {
        st.remaining = 0;
        return;
    };
    let yaw = church.yaw(0);
    let at = Vec3::new(p.x, 0.0, p.y) + Quat::from_rotation_y(yaw) * BELFRY_LOCAL;
    commands.spawn((
        BellStrike(at),
        AudioPlayer::new(sound.0.clone()),
        // Dal campanile: si sente da che parte sta la chiesa.
        PlaybackSettings::DESPAWN
            .with_spatial(true)
            .with_volume(Volume::Linear(0.0)),
        Transform::from_translation(at),
    ));
    st.remaining -= 1;
    st.timer = STRIKE_GAP;
}

/// Volume di un rintocco in base alla distanza dal giocatore.
pub fn bell_gain(distance: f32) -> f32 {
    (1.0 - distance / HEARING_RANGE).clamp(0.0, 1.0).powi(2)
}

/// Sistema: aggiorna il volume dei rintocchi secondo la distanza (e il volume ambiente).
pub fn update_bell_volume(
    settings: Res<AudioSettings>,
    players: Query<&GlobalTransform, With<Player>>,
    mut bells: Query<(&BellStrike, &mut SpatialAudioSink)>,
) {
    let Ok(p) = players.single() else {
        return;
    };
    for (b, mut sink) in &mut bells {
        let d = p.translation().distance(b.0);
        let target = Volume::Linear(
            settings.ambient_volume * 1.4 * crate::audio::SPATIAL_GAIN * bell_gain(d),
        );
        if sink.volume() != target {
            sink.set_volume(target);
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, make_bell)
        .add_systems(Update, (ring_bells, update_bell_volume));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strikes_follow_the_clock() {
        assert_eq!(strikes_at(12, 6.5, 19.5), 12);
        assert_eq!(strikes_at(0, 6.5, 19.5), 12);
        assert_eq!(strikes_at(5, 6.5, 19.5), 1);
        assert_eq!(strikes_at(18, 6.5, 19.5), 1);
        assert_eq!(strikes_at(9, 6.5, 19.5), 0);
    }

    #[test]
    fn bell_is_audible_only_nearby() {
        assert!(bell_gain(5.0) > 0.9);
        assert!(bell_gain(100.0) > 0.1);
        assert_eq!(bell_gain(HEARING_RANGE + 10.0), 0.0);
    }

    #[test]
    fn synth_is_a_valid_wav() {
        let w = synth_bell();
        assert_eq!(&w[0..4], b"RIFF");
        assert!(w.len() > 100_000);
    }
}
