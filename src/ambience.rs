//! Suoni d'ambiente, tutti sintetizzati in locale (nessun file audio), ADR 0028.
//!
//! - Anelli continui con volume secondo il luogo: onde, ruscello, grilli, brusio della piazza,
//!   vento in quota.
//! - Suoni sparsi a caso: uccelli di giorno, gabbiani vicino al mare.
//! - Passi del giocatore secondo la superficie, misurati dallo spazio percorso.
//!
//! Tutto segue il volume «Ambiente» e si abbassa al coperto e mentre parla un personaggio.

use crate::AppState;
use crate::audio::{AudioSettings, MusicDuck};
use crate::daynight::{GameClock, Season};
use crate::player::{GroundedState, Player};
use crate::shell::Cheats;
use crate::terrain::{SEA_LEVEL, Surface, TerrainField};
use crate::village::VillageInfo;
use crate::weather::{Noise, RATE, WeatherState, make_loop, to_wav};
use bevy::audio::{AudioSink, AudioSource, Volume};
use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

/// Oltre questa distanza (m) la riva non si sente.
const WAVES_RANGE: f32 = 75.0;
const STREAM_RANGE: f32 = 22.0;
const MARKET_RANGE: f32 = 42.0;
/// Quota (m) da cui il vento comincia a farsi sentire di più.
const WIND_ALTITUDE: f32 = 5.0;
const WALK_STRIDE: f32 = 1.5;
const RUN_STRIDE: f32 = 2.3;
/// Velocità (m/s) oltre la quale il passo è di corsa.
const RUN_SPEED: f32 = 4.2;
/// Secondi in cui i volumi raggiungono il nuovo livello.
const FADE_RATE: f32 = 1.5;
/// Metà altezza della capsula: il centro del giocatore sta così sopra i piedi.
const FEET_BELOW_CENTER: f32 = 0.85;

/// Anelli continui.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bed {
    Waves,
    Stream,
    Crickets,
    Market,
    HighWind,
}

const BEDS: [(Bed, f32); 5] = [
    (Bed::Waves, 0.9),
    (Bed::Stream, 0.55),
    (Bed::Crickets, 0.4),
    // Registrazione vera (picchi a 0 dB, media −16 dB): più bassa del brusio sintetico di prima.
    (Bed::Market, 0.35),
    (Bed::HighWind, 0.55),
];

#[derive(Component)]
struct AmbientLoop {
    bed: Bed,
    gain: f32,
    level: f32,
}

/// Campioni pronti e dati del luogo.
#[derive(Resource)]
pub struct AmbienceSounds {
    birds: Vec<Handle<AudioSource>>,
    gulls: Vec<Handle<AudioSource>>,
    /// Tre varianti per ogni superficie, nell'ordine di [`Surface::ALL`].
    steps: Vec<Vec<Handle<AudioSource>>>,
    splash: Handle<AudioSource>,
    fish: Handle<AudioSource>,
}

// --- Sintesi -----------------------------------------------------------------------------------

/// Onde: tre frangenti in 12 s, ognuno con salita, schianto e lunga schiuma che si ritira, su un
/// brontolio grave continuo. Ciclo di 12 s.
pub fn synth_waves() -> Vec<u8> {
    let n = RATE as usize * 12;
    let c = RATE as usize / 2;
    let mut rng = Noise(0x51A5_EA11);
    let (mut lo, mut mid, mut hi) = (0.0f32, 0.0f32, 0.0f32);
    let mut buf = Vec::with_capacity(n + c);
    // Istanti dei frangenti (s) e loro forza.
    let waves = [(1.2f32, 1.0f32), (5.3, 0.75), (8.9, 0.9)];
    for i in 0..(n + c) {
        let t = i as f32 / RATE as f32;
        let w = rng.next();
        lo += (w - lo) * 0.02;
        mid += (w - mid) * 0.12;
        hi += (w - hi) * 0.4;
        let mut foam = 0.0f32;
        for (t0, force) in waves {
            // Il ciclo si richiude: l'ultima onda finisce dentro l'inizio.
            for shift in [0.0f32, 12.0] {
                let d = t - t0 + shift;
                let env = if d < 0.0 {
                    0.0
                } else if d < 1.4 {
                    (d / 1.4).powi(2)
                } else {
                    (-(d - 1.4) * 0.75).exp()
                };
                foam += env * force;
            }
        }
        let rumble = lo * 3.0 * (0.45 + 0.25 * (t * TAU / 6.0).sin());
        buf.push(rumble + (mid * 1.3 + (hi - mid) * 0.5) * foam * 0.9);
    }
    to_wav(&make_loop(&buf, n), 0.65)
}

/// Schizzo di chi entra in acqua: scroscio breve con bolle.
pub fn synth_splash() -> Vec<u8> {
    let n = (RATE as f32 * 0.75) as usize;
    let mut rng = Noise(0x5B1A_5521);
    let (mut a, mut b) = (0.0f32, 0.0f32);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        let w = rng.next();
        a += (w - a) * 0.35;
        b += (w - b) * 0.06;
        let env = (t * 90.0).min(1.0) * (-t * 5.5).exp();
        let bubble = (TAU * (380.0 * t + 1400.0 * t * t)).sin() * (-t * 16.0).exp();
        out.push((a - b) * env + bubble * 0.35);
    }
    to_wav(&out, 0.8)
}

/// Pesce che guizza fuori e rientra: due «plop» ravvicinati.
pub fn synth_fish() -> Vec<u8> {
    let n = (RATE as f32 * 0.5) as usize;
    let mut rng = Noise(0xF15_C0DE);
    let mut out = vec![0.0f32; n];
    for (start, f0) in [(0.0f32, 520.0f32), (0.13, 330.0)] {
        let s0 = (start * RATE as f32) as usize;
        for k in 0..(RATE as f32 * 0.22) as usize {
            if s0 + k >= n {
                break;
            }
            let t = k as f32 / RATE as f32;
            let tone = (TAU * (f0 * t + 1700.0 * t * t)).sin() * (-t * 26.0).exp();
            out[s0 + k] += tone * 0.7 + rng.next() * (-t * 60.0).exp() * 0.25;
        }
    }
    to_wav(&out, 0.7)
}

/// Ruscello: fruscio medio con bollicine che salgono di tono. Ciclo di 8 s.
pub fn synth_stream() -> Vec<u8> {
    let n = RATE as usize * 8;
    let c = RATE as usize / 2;
    let mut rng = Noise(0x0B0B_1E55);
    let (mut a, mut b) = (0.0f32, 0.0f32);
    let mut buf = vec![0.0f32; n + c];
    for s in buf.iter_mut() {
        let w = rng.next();
        a += (w - a) * 0.28;
        b += (w - b) * 0.05;
        *s = (a - b) * 0.45;
    }
    let mut at = 0usize;
    while at < n + c {
        let f = 450.0 + (rng.next() + 1.0) * 550.0;
        let amp = 0.2 + (rng.next() + 1.0) * 0.2;
        let len = (RATE as f32 * 0.05) as usize;
        for k in 0..len.min(n + c - at) {
            let t = k as f32 / RATE as f32;
            buf[at + k] += (TAU * (f * t + 2600.0 * t * t)).sin() * amp * (-t * 70.0).exp();
        }
        at += (RATE as f32 * (0.012 + (rng.next() + 1.0) * 0.03)) as usize;
    }
    to_wav(&make_loop(&buf, n), 0.55)
}

/// Grilli: tre insetti con ritmo diverso, sugli stessi 6 s così il ciclo non ha cuciture.
pub fn synth_crickets() -> Vec<u8> {
    let n = RATE as usize * 6;
    let mut buf = vec![0.0f32; n];
    let singers = [
        (4300.0f32, 15usize, 3usize, 1.0f32, 0.0f32),
        (4700.0, 12, 4, 0.7, 0.13),
        (3900.0, 18, 2, 0.6, 0.37),
    ];
    let pulse = (RATE as f32 * 0.024) as usize;
    for (freq, chirps, pulses, gain, phase) in singers {
        for k in 0..chirps {
            let start = (k as f32 + phase) / chirps as f32 * n as f32;
            for p in 0..pulses {
                let at = start as usize + p * (RATE as f32 * 0.04) as usize;
                for j in 0..pulse {
                    let t = j as f32 / RATE as f32;
                    let env = (j as f32 / pulse as f32 * PI).sin();
                    buf[(at + j) % n] += (TAU * freq * t).sin() * env * gain;
                }
            }
        }
    }
    to_wav(&buf, 0.5)
}

/// Canto di un uccello: qualche nota che sale o scende, a volte trillata.
pub fn synth_bird(seed: u32) -> Vec<u8> {
    let mut rng = Noise(seed | 1);
    let mut uniform = move || (rng.next() + 1.0) * 0.5;
    let base = 2300.0 + uniform() * 2600.0;
    let trill = uniform() > 0.5;
    let trill_hz = 28.0 + uniform() * 24.0;
    let mut out = Vec::new();
    for _ in 0..3 + (uniform() * 5.0) as usize {
        let f0 = base * (0.85 + uniform() * 0.4);
        let sweep = (uniform() - 0.5) * 0.7;
        let len = (RATE as f32 * (0.05 + uniform() * 0.1)) as usize;
        let mut phase = 0.0f32;
        for j in 0..len {
            let x = j as f32 / len as f32;
            phase = (phase + f0 * (1.0 + sweep * x) / RATE as f32).fract();
            let am = if trill {
                0.55 + 0.45 * (TAU * trill_hz * j as f32 / RATE as f32).sin()
            } else {
                1.0
            };
            out.push((TAU * phase).sin() * (x * PI).sin() * am);
        }
        out.extend(std::iter::repeat_n(
            0.0,
            (RATE as f32 * (0.03 + uniform() * 0.08)) as usize,
        ));
    }
    to_wav(&out, 0.7)
}

/// Grido di gabbiano: tre note nasali che scendono.
pub fn synth_gull(seed: u32) -> Vec<u8> {
    let mut rng = Noise(seed | 1);
    let mut uniform = move || (rng.next() + 1.0) * 0.5;
    let mut out = Vec::new();
    let base = 820.0 + uniform() * 260.0;
    let cries = 3 + (uniform() * 2.0) as usize;
    for c in 0..cries {
        let len = (RATE as f32 * (0.26 + uniform() * 0.14)) as usize;
        let drop = 1.0 - c as f32 * 0.07;
        let mut phase = 0.0f32;
        for j in 0..len {
            let x = j as f32 / len as f32;
            // Sale subito e poi cala, come un «kia».
            let f = base * drop * (1.0 + 0.55 * (x * PI).sin().powf(0.5) - 0.3 * x);
            phase = (phase + f / RATE as f32).fract();
            let tone = (TAU * phase).sin()
                + 0.6 * (2.0 * TAU * phase).sin()
                + 0.4 * (3.0 * TAU * phase).sin()
                + 0.2 * (5.0 * TAU * phase).sin();
            let rough = 0.8 + 0.2 * (TAU * 70.0 * j as f32 / RATE as f32).sin();
            out.push(tone * (x * PI).sin().powf(0.4) * rough);
        }
        out.extend(std::iter::repeat_n(0.0, (RATE as f32 * 0.09) as usize));
    }
    to_wav(&out, 0.7)
}

/// Un passo sulla superficie data; `seed` cambia la grana da una variante all'altra.
pub fn synth_step(surface: Surface, seed: u32) -> Vec<u8> {
    let mut rng = Noise(seed.wrapping_mul(2_654_435_761) | 1);
    let dur = match surface {
        Surface::Water => 0.34,
        Surface::Crops => 0.42,
        Surface::Sand | Surface::Snow => 0.26,
        _ => 0.2,
    };
    let n = (RATE as f32 * dur) as usize;
    let (mut lo, mut band, mut mid) = (0.0f32, 0.0f32, 0.0f32);
    let mut grain = 1.0f32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        let w = rng.next();
        lo += (w - lo) * 0.08;
        band += (w - band) * 0.45;
        mid += (w - mid) * 0.17;
        let hi = band - lo;
        if i % 90 == 0 {
            grain = 0.25 + (rng.next() + 1.0) * 0.5;
        }
        let thud = |f: f32, d: f32| (TAU * f * t).sin() * (-t * d).exp();
        let attack = (t * 500.0).min(1.0);
        let s = match surface {
            // Erba: fili che si piegano sotto il piede, un fruscio breve e acuto (sopra i 2 kHz)
            // con la grana degli steli; del tonfo resta appena un accenno. Prima era rumore
            // medio-basso con un tonfo a 62 Hz: un «tump» sordo che con l'erba non c'entrava.
            Surface::Grass => {
                let swish = (t * 70.0).min(1.0) * (-t * 24.0).exp();
                hi * swish * (0.55 + 0.45 * grain) + thud(80.0, 40.0) * 0.04
            }
            // Steli di grano o ortaggi: fruscio lungo e secco che sfrega sulle gambe.
            Surface::Crops => {
                let swish = (t * 18.0).min(1.0) * (-t * 4.0).exp();
                (hi * 1.2 + mid * 0.8) * swish * (0.55 + 0.45 * grain)
            }
            Surface::Dirt => lo * 2.2 * (-t * 30.0).exp() + thud(95.0, 38.0) * 0.5,
            Surface::Sand => hi * grain * (-t * 14.0).exp() * 0.7 + lo * (-t * 26.0).exp(),
            Surface::Snow => hi * grain * grain * (-t * 11.0).exp() * 0.9,
            Surface::Stone => {
                hi * (-t * 85.0).exp() * 0.9 + thud(150.0, 60.0) * 0.45 + thud(1250.0, 130.0) * 0.2
            }
            Surface::Wood => {
                thud(210.0, 20.0) * 0.8
                    + thud(430.0, 34.0) * 0.5
                    + thud(95.0, 16.0) * 0.4
                    + hi * (-t * 140.0).exp() * 0.5
            }
            Surface::Water => {
                let bubble = (TAU * (420.0 * t + 900.0 * t * t)).sin() * (-t * 14.0).exp();
                hi * (-t * 9.0).exp() * 0.9 + bubble * 0.3
            }
        };
        out.push(s * attack);
    }
    to_wav(&out, step_peak(surface))
}

/// Picco di un passo per superficie (`to_wav` porta ogni passo a questo picco): l'erba è
/// soffice e si sente meno della pietra o del legno.
fn step_peak(surface: Surface) -> f32 {
    match surface {
        Surface::Grass => 0.4,
        _ => 0.75,
    }
}

// --- Regole sui livelli ------------------------------------------------------------------------

/// Quanta luce c'è, 0 di notte e 1 di giorno, con tre quarti d'ora di passaggio.
pub fn daylight(hour: f32, sunrise: f32, sunset: f32) -> f32 {
    let up = ((hour - sunrise) / 0.75 + 0.5).clamp(0.0, 1.0);
    let down = ((sunset - hour) / 0.75 + 0.5).clamp(0.0, 1.0);
    up.min(down)
}

/// Attività degli uccelli: più forte all'alba, zitti con la pioggia e d'inverno più radi.
pub fn birds_level(hour: f32, sunrise: f32, sunset: f32, rain: f32, season: Season) -> f32 {
    let dawn = 1.0 + 0.6 * (1.0 - ((hour - sunrise - 0.75).abs() / 1.5).min(1.0));
    let year = match season {
        Season::Spring => 1.0,
        Season::Summer => 0.85,
        Season::Autumn => 0.55,
        Season::Winter => 0.25,
    };
    daylight(hour, sunrise, sunset) * dawn * year * (1.0 - rain * 1.4).max(0.0)
}

/// I grilli cantano nelle notti tiepide e asciutte.
pub fn crickets_level(light: f32, temp: f32, rain: f32, snow: f32) -> f32 {
    (1.0 - light)
        * ((temp - 9.0) / 7.0).clamp(0.0, 1.0)
        * (1.0 - rain * 2.0).max(0.0)
        * (1.0 - snow * 3.0).max(0.0)
}

/// Volume di una sorgente a distanza `d` con portata `range`.
pub fn falloff(d: f32, range: f32) -> f32 {
    let x = (1.0 - d / range).clamp(0.0, 1.0);
    x * x
}

/// Il brusio in piazza c'è dalle 8 alle 19 e cala con il maltempo.
pub fn market_level(d: f32, hour: f32, rain: f32) -> f32 {
    let open = ((hour - 7.5).min(19.5 - hour) * 2.0).clamp(0.0, 1.0);
    open * falloff(d, MARKET_RANGE) * (1.0 - rain * 1.2).max(0.1)
}

/// Ultima superficie dei passi (0 = nessun passo ancora), per lo stato del pilota automatico.
static LAST_STEP: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Nome dell'ultima superficie calpestata («Grass», «Water»…; vuoto prima del primo passo).
pub fn last_step() -> String {
    match LAST_STEP.load(std::sync::atomic::Ordering::Relaxed) {
        0 => String::new(),
        n => format!("{:?}", Surface::ALL[(n - 1) as usize]),
    }
}

/// Superficie sotto i piedi: `feet` è la quota del giocatore, `at_sea` dice se il punto è sul
/// mare aperto. Una conca dell'entroterra più bassa del mare (il fossato fuori dalle mura) resta
/// erba: conta la maschera del mare, non la quota.
pub fn step_surface(
    ground: Surface,
    feet: f32,
    terrain_y: f32,
    at_sea: bool,
    on_wall: bool,
    snow: f32,
) -> Surface {
    if at_sea && terrain_y < SEA_LEVEL {
        // Sopra il mare si sta su un molo o si è in acqua bassa.
        return if feet > SEA_LEVEL + 0.7 {
            Surface::Wood
        } else {
            Surface::Water
        };
    }
    if feet > terrain_y + 0.25 {
        return if on_wall {
            Surface::Stone
        } else {
            Surface::Wood
        };
    }
    if snow > 0.35 && matches!(ground, Surface::Grass | Surface::Dirt | Surface::Sand) {
        return Surface::Snow;
    }
    ground
}

// --- Sistemi -----------------------------------------------------------------------------------

/// Brusio del mercato: la registrazione «Vita tra i banchi» dell'autore del gioco, in ciclo
/// (160 s, coda e inizio sfumati l'una nell'altro). Beta v0.0.63, n. 35.
pub const MARKET_FILE: &str = "audio/ambience/vita_tra_i_banchi.ogg";

fn setup(
    mut commands: Commands,
    mut sources: ResMut<Assets<AudioSource>>,
    asset_server: Res<AssetServer>,
) {
    let mut add = |bytes: Vec<u8>| {
        sources.add(AudioSource {
            bytes: bytes.into(),
        })
    };
    let beds = [
        add(synth_waves()),
        add(synth_stream()),
        add(synth_crickets()),
        asset_server.load(MARKET_FILE),
        add(crate::weather::synth_wind()),
    ];
    let birds = (0..6).map(|i| add(synth_bird(0xB1D0 + i * 977))).collect();
    let gulls = (0..3).map(|i| add(synth_gull(0x6A11 + i * 613))).collect();
    let steps = Surface::ALL
        .iter()
        .map(|s| (0..3).map(|i| add(synth_step(*s, i + 1))).collect())
        .collect();
    for ((bed, gain), handle) in BEDS.into_iter().zip(beds) {
        commands.spawn((
            AmbientLoop {
                bed,
                gain,
                level: 0.0,
            },
            AudioPlayer::new(handle),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
        ));
    }
    let splash = add(synth_splash());
    let fish = add(synth_fish());
    commands.insert_resource(AmbienceSounds {
        birds,
        gulls,
        steps,
        splash,
        fish,
    });
}

#[derive(Default)]
struct Scan {
    timer: f32,
    shore: f32,
    water: f32,
    forest: f32,
    bird: f32,
    gull: f32,
    fish: f32,
    logged: [f32; 5],
    rng: u32,
}

impl Scan {
    fn uniform(&mut self) -> f32 {
        if self.rng == 0 {
            self.rng = 0x2545_F491;
        }
        (Noise::step(&mut self.rng) + 1.0) * 0.5
    }
}

/// Sistema: volume degli anelli e suoni sparsi.
#[allow(clippy::too_many_arguments)]
fn update_ambience(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<State<AppState>>,
    clock: Res<GameClock>,
    weather: Res<WeatherState>,
    settings: Res<AudioSettings>,
    duck: Res<MusicDuck>,
    field: Option<Res<TerrainField>>,
    village: Option<Res<VillageInfo>>,
    sounds: Option<Res<AmbienceSounds>>,
    player: Query<&Transform, With<Player>>,
    mut loops: Query<(&mut AmbientLoop, &mut AudioSink)>,
    mut st: Local<Scan>,
) {
    let dt = time.delta_secs();
    let in_world = !matches!(
        state.get(),
        AppState::MainMenu | AppState::Loading | AppState::Resetting
    );
    let (Some(field), Some(sounds), Ok(player)) = (field, sounds, player.single()) else {
        return;
    };
    let p = player.translation;
    let flat = p.xz();
    st.timer -= dt;
    if st.timer <= 0.0 {
        st.timer = 0.25;
        st.shore = field.distance_to_shore(flat);
        st.water = field.distance_to_fresh_water(flat);
        st.forest = crate::water::load_spec_cached()
            .forests
            .iter()
            .map(|f| 1.0 - ((flat.distance(f.center) / f.radius - 0.6) * 2.5).clamp(0.0, 1.0))
            .fold(0.0, f32::max);
    }
    let light = daylight(clock.hour, clock.sunrise, clock.sunset);
    let plaza = village.map_or(f32::MAX, |v| flat.distance(v.spec.plaza));
    // Dentro casa l'esterno si sente attutito.
    let inside = 1.0 - 0.75 * weather.sheltered;
    let master = settings.ambient_volume * duck.level * inside;
    let windy = 0.35 + (weather.wind / 9.0).min(1.0);

    let mut targets = [0.0f32; 5];
    if in_world {
        for (slot, (bed, _)) in targets.iter_mut().zip(BEDS) {
            *slot = match bed {
                Bed::Waves => falloff(st.shore, WAVES_RANGE) * (0.7 + 0.5 * windy),
                Bed::Stream => falloff(st.water, STREAM_RANGE),
                Bed::Crickets => {
                    crickets_level(light, weather.temp, weather.rain, weather.snow_cover)
                        * (0.45 + 0.55 * st.forest)
                }
                Bed::Market => market_level(plaza, clock.hour, weather.rain),
                Bed::HighWind => {
                    ((p.y - FEET_BELOW_CENTER - WIND_ALTITUDE) / 9.0).clamp(0.0, 1.0) * windy
                }
            };
        }
    }
    let k = 1.0 - (-dt * FADE_RATE).exp();
    for (mut bed, mut sink) in &mut loops {
        let i = BEDS.iter().position(|(b, _)| *b == bed.bed).unwrap_or(0);
        bed.level += (targets[i] - bed.level) * k;
        let target = Volume::Linear(master * bed.gain * bed.level);
        if sink.volume() != target {
            sink.set_volume(target);
        }
    }
    if targets
        .iter()
        .zip(st.logged)
        .any(|(a, b)| (a - b).abs() > 0.2)
    {
        st.logged = targets;
        info!(
            "[AMBIENTE] onde {:.2} ruscello {:.2} grilli {:.2} piazza {:.2} vento {:.2}",
            targets[0], targets[1], targets[2], targets[3], targets[4]
        );
    }
    if !in_world || *state.get() != AppState::Playing {
        return;
    }

    // Uccelli: più fitti nel bosco.
    let birds = birds_level(
        clock.hour,
        clock.sunrise,
        clock.sunset,
        weather.rain,
        clock.date.season(),
    ) * (0.35 + 0.65 * st.forest);
    st.bird -= dt;
    if st.bird <= 0.0 {
        let wait = 1.2 + st.uniform() * 5.0;
        st.bird = wait / (0.35 + birds);
        if birds > 0.03 {
            let i = (st.uniform() * sounds.birds.len() as f32) as usize % sounds.birds.len();
            let loud = 0.25 + st.uniform() * 0.5;
            // Da un albero qualsiasi attorno, a qualche metro d'altezza.
            let at = crate::audio::around(
                player.translation,
                st.uniform() * std::f32::consts::TAU,
                6.0 + st.uniform() * 14.0,
                3.0 + st.uniform() * 5.0,
            );
            commands.spawn((
                AudioPlayer::new(sounds.birds[i].clone()),
                crate::audio::spatial_once(master * birds.min(1.0) * loud * 0.55)
                    .with_speed(0.9 + st.uniform() * 0.25),
                at,
            ));
        }
    }
    // Pesci che guizzano: vicino al laghetto e al ruscello, di giorno.
    st.fish -= dt;
    if st.fish <= 0.0 {
        st.fish = 4.0 + st.uniform() * 8.0;
        let near = falloff(st.water, 18.0) * light;
        if near > 0.05 {
            let at = crate::audio::around(
                player.translation,
                st.uniform() * std::f32::consts::TAU,
                4.0 + st.uniform() * 8.0,
                -0.5,
            );
            commands.spawn((
                AudioPlayer::new(sounds.fish.clone()),
                crate::audio::spatial_once(master * near * 0.8)
                    .with_speed(0.9 + st.uniform() * 0.25),
                at,
            ));
        }
    }
    // Gabbiani: di giorno, vicino alla costa.
    st.gull -= dt;
    if st.gull <= 0.0 {
        st.gull = 3.0 + st.uniform() * 9.0;
        let near = falloff(st.shore, 60.0) * light * (1.0 - weather.rain).max(0.0);
        if near > 0.03 {
            let i = (st.uniform() * sounds.gulls.len() as f32) as usize % sounds.gulls.len();
            let loud = 0.4 + st.uniform() * 0.5;
            // In volo, in alto e più lontani degli uccelli.
            let at = crate::audio::around(
                player.translation,
                st.uniform() * std::f32::consts::TAU,
                15.0 + st.uniform() * 25.0,
                10.0 + st.uniform() * 10.0,
            );
            commands.spawn((
                AudioPlayer::new(sounds.gulls[i].clone()),
                crate::audio::spatial_once(master * near * loud * 0.5)
                    .with_speed(0.92 + st.uniform() * 0.2),
                at,
            ));
        }
    }
}

#[derive(Default)]
struct Steps {
    last: Option<Vec3>,
    walked: f32,
    speed: f32,
    air: f32,
    variant: usize,
    surface: Option<Surface>,
    rng: u32,
}

/// Sistema: passi del giocatore secondo lo spazio percorso e la superficie.
#[allow(clippy::too_many_arguments)]
fn footsteps(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<AudioSettings>,
    weather: Res<WeatherState>,
    cheats: Res<Cheats>,
    field: Option<Res<TerrainField>>,
    village: Option<Res<VillageInfo>>,
    sounds: Option<Res<AmbienceSounds>>,
    dbg: Res<crate::debug_place::DebugPlace>,
    buildings: Query<(&GlobalTransform, &crate::world_props::BuildingFootprint)>,
    player: Query<(&Transform, &GroundedState), With<Player>>,
    mut st: Local<Steps>,
) {
    let (Some(field), Some(sounds), Ok((t, grounded))) = (field, sounds, player.single()) else {
        return;
    };
    let dt = time.delta_secs().max(1e-4);
    let p = t.translation;
    let moved = st.last.map_or(0.0, |l| (p - l).xz().length());
    st.last = Some(p);
    if moved > 4.0 || cheats.noclip {
        // Teletrasporto, ricarica o volo: nessun passo.
        st.walked = 0.0;
        return;
    }
    st.speed += (moved / dt - st.speed) * (1.0 - (-dt * 8.0).exp());
    let landed = grounded.grounded_now && st.air > 0.3;
    st.air = if grounded.grounded_now {
        0.0
    } else {
        st.air + dt
    };
    if !grounded.grounded_now {
        return;
    }
    st.walked += moved;
    let running = st.speed > RUN_SPEED;
    let stride = if running { RUN_STRIDE } else { WALK_STRIDE };
    if st.walked < stride && !landed {
        return;
    }
    st.walked = 0.0;
    let on_wall = village.is_some_and(|v| v.in_wall_band(p.xz()));
    let terrain_y = field.height_at(p.x, p.z);
    // All'atterraggio la capsula è ancora un soffio sopra il suolo.
    let feet = p.y - FEET_BELOW_CENTER - if landed { 0.3 } else { 0.0 };
    let surface = step_surface(
        field.surface_at(p.x, p.z),
        feet,
        terrain_y,
        field.in_sea(p.x, p.z),
        on_wall,
        weather.snow_cover,
    );
    let surface = if field.in_fresh_water(p.x, p.z) && feet < terrain_y + 0.25 {
        Surface::Water
    } else {
        surface
    };
    // Dentro un edificio si cammina su assi di legno, anche se il pavimento sta al livello del suolo.
    let inside = buildings.iter().any(|(g, f)| {
        let l = g.affine().inverse().transform_point3(p);
        l.x.abs() <= f.half.x && l.z.abs() <= f.half.y && (l.y - FEET_BELOW_CENTER) > -0.5
    });
    let surface = match surface {
        Surface::Grass | Surface::Dirt | Surface::Sand | Surface::Snow if inside => Surface::Wood,
        Surface::Grass | Surface::Dirt
            if dbg
                .items
                .iter()
                .filter(|i| matches!(i.kind.as_str(), "campo" | "orto"))
                .any(|i| i.points.iter().any(|q| q.distance(p.xz()) < 0.9)) =>
        {
            Surface::Crops
        }
        s => s,
    };
    if st.surface != Some(surface) {
        if surface == Surface::Water {
            commands.spawn((
                AudioPlayer::new(sounds.splash.clone()),
                PlaybackSettings::DESPAWN
                    .with_volume(Volume::Linear(settings.ambient_volume * 0.9)),
            ));
        }
        st.surface = Some(surface);
        LAST_STEP.store(surface as u8 + 1, std::sync::atomic::Ordering::Relaxed);
        info!("[PASSI] superficie: {surface:?}");
    }
    let Some(set) = Surface::ALL
        .iter()
        .position(|s| *s == surface)
        .and_then(|i| sounds.steps.get(i))
    else {
        return;
    };
    st.variant = (st.variant + 1) % set.len();
    if st.rng == 0 {
        st.rng = 0x1F2E_3D4C;
    }
    let pace = 0.92 + (Noise::step(&mut st.rng) + 1.0) * 0.08;
    let loud = if landed {
        1.0
    } else if running {
        0.75
    } else {
        0.5
    };
    commands.spawn((
        AudioPlayer::new(set[st.variant].clone()),
        PlaybackSettings::DESPAWN
            .with_volume(Volume::Linear(settings.ambient_volume * loud * 0.7))
            .with_speed(pace),
    ));
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, setup).add_systems(
        Update,
        (
            update_ambience,
            footsteps.run_if(in_state(AppState::Playing)),
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(wav: &[u8]) -> Vec<i16> {
        let (chunks, _) = wav[44..].as_chunks::<2>();
        chunks.iter().map(|c| i16::from_le_bytes(*c)).collect()
    }

    fn audible(wav: &[u8]) {
        assert_eq!(&wav[..4], b"RIFF");
        let s = pcm(wav);
        let rms = (s.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / s.len() as f64).sqrt();
        assert!(rms > 300.0, "troppo piano: {rms}");
        let peak = s.iter().map(|v| v.unsigned_abs()).max().unwrap();
        assert!(peak < 32_000, "satura: {peak}");
    }

    #[test]
    fn every_sound_is_a_valid_audible_wav() {
        for wav in [
            synth_waves(),
            synth_stream(),
            synth_crickets(),
            synth_bird(3),
            synth_gull(5),
        ] {
            audible(&wav);
        }
        for s in Surface::ALL {
            audible(&synth_step(s, 1));
        }
    }

    /// Quota dell'energia sotto i 250 Hz (FFT a mano, solo per i test).
    fn bass_share(s: &[i16]) -> f64 {
        let n = s.len().min(4096);
        let (mut low, mut all) = (0.0f64, 0.0f64);
        for k in 1..n / 2 {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (i, v) in s[..n].iter().enumerate() {
                let a = std::f64::consts::TAU * (k * i) as f64 / n as f64;
                re += *v as f64 * a.cos();
                im -= *v as f64 * a.sin();
            }
            let e = re * re + im * im;
            all += e;
            if (k as f64) * (RATE as f64) / (n as f64) < 250.0 {
                low += e;
            }
        }
        low / all
    }

    #[test]
    fn grass_steps_are_soft_rustles() {
        let grass = pcm(&synth_step(Surface::Grass, 1));
        let stone = pcm(&synth_step(Surface::Stone, 1));
        let peak = |s: &[i16]| s.iter().map(|v| v.unsigned_abs() as u32).max().unwrap();
        // Più piano della pietra.
        assert!(
            peak(&grass) < peak(&stone) * 6 / 10,
            "{} {}",
            peak(&grass),
            peak(&stone)
        );
        // Niente tonfo: quasi tutta l'energia sopra i 250 Hz.
        let bass = bass_share(&grass);
        assert!(bass < 0.1, "bassi {bass:.2}");
    }

    /// Scrive un passo per superficie in `PLANET_ONE_OUT` (WAV) per ascoltarli con `scripts/ascolta.py`:
    /// `PLANET_ONE_OUT=cartella cargo test write_step_samples -- --ignored`.
    #[test]
    #[ignore]
    fn write_step_samples() {
        let dir =
            std::path::PathBuf::from(std::env::var("PLANET_ONE_OUT").unwrap_or("passi".into()));
        std::fs::create_dir_all(&dir).unwrap();
        for s in Surface::ALL {
            let mut all = Vec::new();
            for v in 1..=4 {
                all.extend(pcm(&synth_step(s, v)));
                all.extend(std::iter::repeat_n(0i16, RATE as usize / 3));
            }
            let bytes: Vec<u8> = all.iter().flat_map(|v| v.to_le_bytes()).collect();
            std::fs::write(
                dir.join(format!("{s:?}.wav").to_lowercase()),
                crate::ai::tts::pcm_to_wav(&bytes, RATE),
            )
            .unwrap();
        }
    }

    #[test]
    fn step_variants_differ() {
        assert_ne!(synth_step(Surface::Sand, 1), synth_step(Surface::Sand, 2));
    }

    #[test]
    fn crickets_loop_without_a_seam() {
        let s = pcm(&synth_crickets());
        let jump = (s[0] as i32 - s[s.len() - 1] as i32).abs();
        assert!(jump < 9_000, "salto {jump}");
    }

    #[test]
    fn birds_sing_by_day_and_stop_in_the_rain() {
        let day = birds_level(12.0, 6.0, 20.0, 0.0, Season::Spring);
        assert!(day > 0.9);
        assert_eq!(birds_level(23.0, 6.0, 20.0, 0.0, Season::Spring), 0.0);
        assert_eq!(birds_level(12.0, 6.0, 20.0, 1.0, Season::Spring), 0.0);
        assert!(birds_level(6.8, 6.0, 20.0, 0.0, Season::Spring) > day);
        assert!(birds_level(12.0, 6.0, 20.0, 0.0, Season::Winter) < day);
    }

    #[test]
    fn crickets_need_a_warm_dry_night() {
        assert!(crickets_level(0.0, 20.0, 0.0, 0.0) > 0.9);
        assert_eq!(crickets_level(1.0, 20.0, 0.0, 0.0), 0.0);
        assert_eq!(crickets_level(0.0, 2.0, 0.0, 0.0), 0.0);
        assert_eq!(crickets_level(0.0, 20.0, 0.6, 0.0), 0.0);
    }

    #[test]
    fn the_market_recording_is_shipped() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(MARKET_FILE);
        let bytes = std::fs::read(&path).expect("registrazione del mercato");
        assert_eq!(
            &bytes[..4],
            b"OggS",
            "deve essere Ogg Vorbis (niente mp3 in Bevy)"
        );
    }

    #[test]
    fn market_closes_at_night_and_fades_with_distance() {
        assert!(market_level(0.0, 12.0, 0.0) > 0.9);
        assert_eq!(market_level(0.0, 23.0, 0.0), 0.0);
        assert_eq!(market_level(MARKET_RANGE + 1.0, 12.0, 0.0), 0.0);
    }

    #[test]
    fn surface_follows_where_the_feet_are() {
        use Surface::*;
        assert_eq!(step_surface(Grass, 3.0, 3.0, false, false, 0.0), Grass);
        assert_eq!(step_surface(Grass, 3.0, 3.0, false, false, 0.8), Snow);
        assert_eq!(step_surface(Stone, 3.0, 3.0, false, false, 0.8), Stone);
        assert_eq!(step_surface(Grass, 3.5, 3.0, false, false, 0.8), Wood);
        assert_eq!(step_surface(Grass, 8.0, 3.0, false, true, 0.0), Stone);
        assert_eq!(step_surface(Sand, 0.1, -2.0, true, false, 0.0), Wood);
        assert_eq!(step_surface(Sand, -1.4, -2.0, true, false, 0.0), Water);
        // Beta v0.0.63, n. 26: il fossato erboso fuori dalle mura è sotto il livello del mare
        // (terreno a -1,4 m), ma non è mare: si sente l'erba, non lo sciabordio.
        assert_eq!(step_surface(Grass, -1.3, -1.4, false, false, 0.0), Grass);
    }
}
