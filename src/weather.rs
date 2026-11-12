//! Meteo (ADR 0014): sereno, nuvoloso, pioggia, temporale, nebbia e neve, deterministici per data,
//! con temperatura, vento, nebbia di distanza, particelle e suoni sintetizzati in locale.
//!
//! Il tipo di meteo del giorno e la sua finestra oraria derivano da un hash della data e dalle
//! probabilità del mese; le intensità (nuvole, pioggia, neve, nebbia) si avvicinano ai valori
//! obiettivo in modo morbido, così non ci sono scatti all'inizio e alla fine di un fenomeno.

use crate::audio::AudioSettings;
use crate::daynight::{GameClock, GameDate};
use crate::seasons::Wind;
use avian3d::prelude::*;
use bevy::audio::{AudioSink, AudioSource, Volume};
use bevy::light::NotShadowCaster;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;

/// Tipo di meteo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Fog,
    Snow,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Clear => "Sereno",
            Kind::Cloudy => "Nuvoloso",
            Kind::Rain => "Pioggia",
            Kind::Storm => "Temporale",
            Kind::Fog => "Nebbia",
            Kind::Snow => "Neve",
        }
    }

    /// Nome scritto nei salvataggi (lo stesso che `parse` riconosce).
    pub fn save_name(self) -> &'static str {
        match self {
            Kind::Clear => "sereno",
            Kind::Cloudy => "nuvoloso",
            Kind::Rain => "pioggia",
            Kind::Storm => "temporale",
            Kind::Fog => "nebbia",
            Kind::Snow => "neve",
        }
    }

    /// Legge il nome da `PLANET_ONE_WEATHER`.
    pub fn parse(text: &str) -> Option<Kind> {
        match text.trim().to_lowercase().as_str() {
            "sereno" | "clear" => Some(Kind::Clear),
            "nuvoloso" | "cloudy" => Some(Kind::Cloudy),
            "pioggia" | "rain" => Some(Kind::Rain),
            "temporale" | "storm" => Some(Kind::Storm),
            "nebbia" | "fog" => Some(Kind::Fog),
            "neve" | "snow" => Some(Kind::Snow),
            _ => None,
        }
    }
}

/// Pesi per mese (gen..dic): sereno, nuvoloso, pioggia, temporale, nebbia, neve.
const WEIGHTS: [[f32; 6]; 12] = [
    [30.0, 25.0, 5.0, 0.0, 15.0, 25.0],
    [32.0, 25.0, 8.0, 0.0, 10.0, 25.0],
    [35.0, 25.0, 17.0, 2.0, 8.0, 13.0],
    [38.0, 25.0, 22.0, 5.0, 6.0, 4.0],
    [40.0, 22.0, 22.0, 10.0, 4.0, 2.0],
    [45.0, 20.0, 15.0, 15.0, 3.0, 0.0],
    [55.0, 15.0, 10.0, 15.0, 3.0, 0.0],
    [55.0, 15.0, 10.0, 15.0, 4.0, 0.0],
    [45.0, 22.0, 16.0, 7.0, 10.0, 0.0],
    [35.0, 25.0, 20.0, 3.0, 17.0, 0.0],
    [28.0, 28.0, 17.0, 0.0, 17.0, 10.0],
    [27.0, 25.0, 8.0, 0.0, 15.0, 25.0],
];
const KINDS: [Kind; 6] = [
    Kind::Clear,
    Kind::Cloudy,
    Kind::Rain,
    Kind::Storm,
    Kind::Fog,
    Kind::Snow,
];

/// Meteo di un giorno: tipo e finestra oraria in cui il fenomeno è attivo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DayWeather {
    pub kind: Kind,
    pub start: f32,
    pub len: f32,
}

fn hash01(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B).rotate_left(13);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFF_FFFF) as f32 / 0x100_0000 as f32
}

/// Temperatura (°C): media stagionale (≈0 °C a gennaio, ≈20 °C a luglio) ± 4 °C nel giorno, più l'effetto del meteo.
pub fn temperature(doy: f32, hour: f32, kind: Kind) -> f32 {
    let season = 10.0 - 10.0 * ((doy - 20.0) / 365.0 * std::f32::consts::TAU).cos();
    let daily = 4.0 * ((hour - 9.0) / 24.0 * std::f32::consts::TAU).sin();
    let offset = match kind {
        Kind::Clear => 1.0,
        Kind::Cloudy => -1.0,
        Kind::Rain => -2.0,
        Kind::Storm => -3.0,
        Kind::Fog => -1.0,
        Kind::Snow => -4.0,
    };
    season + daily + offset
}

/// Meteo del giorno `date`: scelta pesata dal mese, finestra casuale; pioggia ↔ neve secondo la temperatura.
pub fn day_weather(date: &GameDate) -> DayWeather {
    let seed = date.year.wrapping_mul(400).wrapping_add(date.day_of_year());
    let w = &WEIGHTS[date.month as usize - 1];
    let total: f32 = w.iter().sum();
    let mut pick = hash01(seed, 1) * total;
    let mut kind = Kind::Clear;
    for (k, weight) in KINDS.iter().zip(w) {
        if pick < *weight {
            kind = *k;
            break;
        }
        pick -= weight;
    }
    let r = |n: u32| hash01(seed, 10 + n);
    let (start, len) = match kind {
        Kind::Clear => (0.0, 0.0),
        Kind::Cloudy => (0.0, 24.0),
        Kind::Rain => (r(0) * 24.0, 5.0 + r(1) * 6.0),
        Kind::Storm => (8.0 + r(0) * 12.0, 2.0 + r(1) * 3.0),
        Kind::Fog => (3.0 + r(0) * 5.0, 4.0 + r(1) * 4.0),
        Kind::Snow => (r(0) * 24.0, 5.0 + r(1) * 7.0),
    };
    // Pioggia in giorni gelati diventa neve e viceversa.
    let mid = temperature(date.day_of_year() as f32, start + len / 2.0, Kind::Clear);
    let kind = match kind {
        Kind::Rain if mid < 1.5 => Kind::Snow,
        Kind::Snow if mid > 3.0 => Kind::Rain,
        k => k,
    };
    DayWeather { kind, start, len }
}

impl DayWeather {
    /// Il fenomeno è in corso a quest'ora?
    pub fn active(&self, hour: f32) -> bool {
        match self.kind {
            Kind::Clear => false,
            Kind::Cloudy => true,
            _ => (hour - self.start).rem_euclid(24.0) < self.len,
        }
    }
}

/// Stato corrente del meteo (valori morbidi 0..1).
#[derive(Resource)]
pub struct WeatherState {
    pub kind: Kind,
    pub active: bool,
    pub temp: f32,
    pub cloud: f32,
    pub rain: f32,
    pub snow: f32,
    pub fog: f32,
    /// Bagliore del lampo (0..1).
    pub flash: f32,
    /// Vento (m/s).
    pub wind: f32,
    /// Il giocatore è al coperto (sopra la testa c'è un tetto), 0..1.
    pub sheltered: f32,
    /// Gli abitanti si riparano in casa (con isteresi).
    pub shelter: bool,
    /// Neve accumulata a terra (0..1), si scioglie sopra lo zero.
    pub snow_cover: f32,
    /// Meteo forzato (`PLANET_ONE_WEATHER` o shell).
    pub forced: Option<Kind>,
    /// Tuoni in arrivo: secondi di ritardo.
    pub thunder: Vec<f32>,
    next_flash: f32,
    flash_t: f32,
    first: bool,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            kind: Kind::Clear,
            active: false,
            temp: 10.0,
            cloud: 0.0,
            rain: 0.0,
            snow: 0.0,
            fog: 0.0,
            flash: 0.0,
            wind: 1.0,
            sheltered: 0.0,
            shelter: false,
            snow_cover: 0.0,
            thunder: Vec::new(),
            next_flash: 6.0,
            flash_t: 99.0,
            first: true,
            forced: None,
        }
    }
}

impl WeatherState {
    /// "Pioggia, 9 °C" (sereno/nuvoloso se non c'è un fenomeno in corso).
    pub fn label(&self) -> String {
        let kind = if self.active || self.kind == Kind::Cloudy {
            self.kind
        } else if self.cloud > 0.35 {
            Kind::Cloudy
        } else {
            Kind::Clear
        };
        format!("{}, {:.0} °C", kind.name(), self.temp)
    }
}

/// Neve a terra (quantizzata, così `seasons::apply_season` non gira a ogni frame).
#[derive(Resource, Default)]
pub struct SnowCover(pub f32);

/// Obiettivi (nuvole, pioggia, neve, nebbia) per un tipo di meteo.
fn targets(kind: Kind, active: bool) -> (f32, f32, f32, f32) {
    match (kind, active) {
        (Kind::Clear, _) => (0.05, 0.0, 0.0, 0.0),
        (Kind::Cloudy, _) => (0.55, 0.0, 0.0, 0.0),
        (Kind::Rain, true) => (0.85, 0.7, 0.0, 0.25),
        (Kind::Storm, true) => (1.0, 1.0, 0.0, 0.3),
        (Kind::Fog, true) => (0.5, 0.0, 0.0, 1.0),
        (Kind::Snow, true) => (0.8, 0.0, 1.0, 0.35),
        (_, false) => (0.25, 0.0, 0.0, 0.0),
    }
}

/// Sistema: aggiorna il meteo dal calendario e dall'ora (valori morbidi, vento, lampi, neve a terra).
#[allow(clippy::too_many_arguments)]
pub fn update_weather(
    time: Res<Time>,
    clock: Res<GameClock>,
    settings: Res<AudioSettings>,
    mut state: ResMut<WeatherState>,
    mut cover: ResMut<SnowCover>,
    mut wind: ResMut<Wind>,
    spatial: SpatialQuery,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    players: Query<(), With<crate::player::Player>>,
    mut read_env: Local<bool>,
) {
    if !*read_env {
        *read_env = true;
        state.forced = std::env::var("PLANET_ONE_WEATHER")
            .ok()
            .and_then(|v| Kind::parse(&v));
    }
    let forced = state.forced;
    let dt = time.delta_secs().min(0.1);
    let speed = settings.time_speed.max(1) as f32;
    let hour = clock.hour;
    let doy = clock.date.day_of_year() as f32;

    let dw = day_weather(&clock.date);
    let (kind, active) = match forced {
        Some(k) => (k, k != Kind::Clear),
        None => (dw.kind, dw.active(hour)),
    };
    state.kind = kind;
    state.active = active;
    state.temp = temperature(doy, hour, if active { kind } else { Kind::Clear });

    let (tc, tr, ts, tf) = targets(kind, active);
    // Piana delle Brume (Atto I): finché non la si dirada, la nebbia è perenne, con una pioggia
    // sottile; solo d'inverno diventa neve. Un meteo forzato (shell, prove) ha la precedenza.
    let (tc, tr, ts, tf) = if forced.is_none()
        && crate::region::current() == crate::region::Region::Nord
        && !(kind == Kind::Snow && active)
    {
        state.kind = Kind::Fog;
        state.active = true;
        (0.75, 0.3, 0.0, 0.8)
    } else {
        (tc, tr, ts, tf)
    };
    let k = if state.first {
        1.0
    } else {
        1.0 - (-dt * speed / 8.0).exp()
    };
    state.first = false;
    state.cloud += (tc - state.cloud) * k;
    state.rain += (tr - state.rain) * k;
    state.snow += (ts - state.snow) * k;
    state.fog += (tf - state.fog) * k;
    let wind_target = 1.0
        + match (kind, active) {
            (Kind::Storm, true) => 7.0,
            (Kind::Rain, true) => 2.5,
            (Kind::Snow, true) => 1.5,
            _ => 0.0,
        };
    state.wind += (wind_target - state.wind) * k;
    let dir = if wind.0.length() > 0.01 {
        wind.0.normalize()
    } else {
        Vec2::new(0.93, 0.36)
    };
    wind.0 = dir * state.wind;

    // Lampi e tuoni durante il temporale.
    if kind == Kind::Storm && active && state.rain > 0.5 {
        state.next_flash -= dt;
        if state.next_flash <= 0.0 {
            state.next_flash =
                7.0 + hash01(clock.date.day as u32, (time.elapsed_secs() * 10.0) as u32) * 18.0;
            state.flash_t = 0.0;
            let delay = 0.5 + hash01(7, (time.elapsed_secs() * 7.0) as u32) * 3.5;
            state.thunder.push(delay);
        }
    }
    state.flash_t += dt;
    let t = state.flash_t;
    state.flash = if t < 0.08 {
        1.0
    } else if t < 0.16 {
        0.2
    } else if t < 0.24 {
        0.9
    } else if t < 0.7 {
        0.9 * (1.0 - (t - 0.24) / 0.46)
    } else {
        0.0
    };

    // Neve a terra: cresce nevicando, si scioglie sopra lo zero.
    let mut c = state.snow_cover;
    c += state.snow * dt * speed / 150.0;
    if state.temp > 0.5 {
        c -= (state.temp + 1.0) * dt * speed / 1500.0;
    }
    state.snow_cover = c.clamp(0.0, 1.0);
    if (state.snow_cover - cover.0).abs() >= 0.01 || (state.snow_cover == 0.0 && cover.0 != 0.0) {
        cover.0 = state.snow_cover;
    }

    // Gli abitanti si riparano con pioggia o neve forti (isteresi).
    let precip = state.rain.max(state.snow);
    if precip > 0.55 {
        state.shelter = true;
    } else if precip < 0.3 {
        state.shelter = false;
    }

    // Al coperto? Un raggio verso l'alto dalla camera (escluso il giocatore) trova un tetto.
    if let Ok(cam) = cameras.single() {
        let covered = spatial
            .cast_ray_predicate(
                cam.translation(),
                Dir3::Y,
                12.0,
                true,
                &SpatialQueryFilter::default(),
                &|e| !players.contains(e),
            )
            .is_some();
        let target = if covered { 1.0 } else { 0.0 };
        state.sheltered += (target - state.sheltered) * (1.0 - (-dt * 3.0).exp());
    }
}

// ---------------------------------------------------------------------------------------------
// Nebbia
// ---------------------------------------------------------------------------------------------

/// Sistema: nebbia di distanza sulla camera (colore secondo l'ora, densità secondo il meteo).
pub fn update_fog(
    mut commands: Commands,
    state: Res<WeatherState>,
    clock: Res<GameClock>,
    mut cameras: Query<(Entity, Option<&mut DistanceFog>), With<Camera3d>>,
) {
    let elev = crate::daynight::sun_dir(clock.day_fraction().floor(), clock.hour).y;
    let day = ((elev + 0.1) / 0.35).clamp(0.0, 1.0);
    let base = Vec3::new(0.03, 0.04, 0.07).lerp(Vec3::new(0.74, 0.78, 0.84), day);
    let grey = base * (1.0 - 0.3 * state.cloud) + Vec3::splat(state.flash * 0.6);
    let density = state.fog * 0.04 + state.rain * 0.008 + state.snow * 0.012 + state.cloud * 0.0015;
    let color = Color::srgb(grey.x, grey.y, grey.z);
    for (e, fog) in &mut cameras {
        let new = DistanceFog {
            color,
            directional_light_color: Color::NONE,
            directional_light_exponent: 8.0,
            falloff: FogFalloff::Exponential { density },
        };
        match fog {
            Some(mut f) => *f = new,
            None => {
                commands.entity(e).insert(new);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Particelle di pioggia e neve (coordinate di mondo)
// ---------------------------------------------------------------------------------------------

const RAIN_POOL: usize = 900;
/// La neve scende lenta e i fiocchi sono piccoli: ne servono molti, in un cerchio più stretto
/// e in uno strato più basso della pioggia, perché la nevicata si veda fitta.
const SNOW_POOL: usize = 2000;
const PRECIP_RADIUS: f32 = 15.0;
const SNOW_RADIUS: f32 = 11.0;
/// Altezza sopra la camera da cui partono gocce e fiocchi (m).
const RAIN_TOP: f32 = 8.0;
const SNOW_TOP: f32 = 4.0;
/// Il cerchio delle particelle è spostato in avanti, dove guarda la camera, di questa
/// frazione del raggio: dietro la camera le particelle non si vedrebbero.
const AHEAD: f32 = 0.6;
/// Le particelle più vicine di così alla camera non si disegnano: a pochi centimetri
/// dall'obiettivo una goccia o un fiocco riempirebbero mezzo schermo.
const PRECIP_NEAR: f32 = 2.5;

/// Una particella è troppo vicina alla camera per essere disegnata.
fn too_close(pos: Vec3, cam: Vec3) -> bool {
    pos.distance_squared(cam) < PRECIP_NEAR * PRECIP_NEAR
}

/// Particella di pioggia o neve.
#[derive(Component)]
pub struct Precip {
    snow: bool,
    index: usize,
    seed: f32,
    counter: u32,
    fresh: bool,
}

fn rnd(seed: f32, k: u32) -> f32 {
    ((seed * 12.9898 + k as f32 * 78.233).sin() * 43_758.545).rem_euclid(1.0)
}

/// Avvio: crea i due gruppi di particelle (nascoste finché non piove o nevica).
pub fn spawn_precip(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let drop = meshes.add(Cuboid::new(0.016, 0.6, 0.016));
    // Fiocco tondo (un cubo da vicino sembrava un quadratino bianco).
    let flake = meshes.add(Sphere::new(0.025).mesh().ico(1).expect("ico"));
    let drop_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.75, 0.82, 0.95, 0.55),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let flake_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });
    for i in 0..RAIN_POOL {
        commands.spawn((
            Precip {
                snow: false,
                index: i,
                seed: i as f32 * 1.37 + 0.11,
                counter: 0,
                fresh: true,
            },
            NotShadowCaster,
            Mesh3d(drop.clone()),
            MeshMaterial3d(drop_mat.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    for i in 0..SNOW_POOL {
        commands.spawn((
            Precip {
                snow: true,
                index: i,
                seed: i as f32 * 2.17 + 0.53,
                counter: 0,
                fresh: true,
            },
            NotShadowCaster,
            Mesh3d(flake.clone()),
            MeshMaterial3d(flake_mat.clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

/// Sistema: muove la pioggia e la neve in coordinate di mondo, riciclandole a terra o fuori raggio.
pub fn update_precip(
    time: Res<Time>,
    settings: Res<AudioSettings>,
    state: Res<WeatherState>,
    wind: Res<Wind>,
    field: Option<Res<crate::terrain::TerrainField>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut parts: Query<(&mut Precip, &mut Transform, &mut Visibility), Without<Camera3d>>,
) {
    let (Ok(cam_t), Some(field)) = (cameras.single(), field) else {
        return;
    };
    let cam = cam_t.translation();
    let ahead = {
        let f = cam_t.forward();
        Vec2::new(f.x, f.z).normalize_or_zero()
    };
    let dt = time.delta_secs().min(0.1);
    let t = time.elapsed_secs();
    let open = (1.0 - 0.9 * state.sheltered) * settings.gfx.particle_scale();
    let rain_n = (state.rain * RAIN_POOL as f32 * open) as usize;
    let snow_n = (state.snow * SNOW_POOL as f32 * open) as usize;
    for (mut p, mut tf, mut vis) in &mut parts {
        let n = if p.snow { snow_n } else { rain_n };
        if p.index >= n {
            *vis = Visibility::Hidden;
            p.fresh = true;
            continue;
        }
        let pos = tf.translation;
        *vis = if too_close(pos, cam) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        let ground = field.height_at(pos.x, pos.z);
        let (radius, above) = if p.snow {
            (SNOW_RADIUS, SNOW_TOP)
        } else {
            (PRECIP_RADIUS, RAIN_TOP)
        };
        let center = Vec2::new(cam.x, cam.z) + ahead * radius * AHEAD;
        let out = Vec2::new(pos.x, pos.z).distance(center) > radius;
        if p.fresh || out || pos.y < ground + 0.02 {
            p.counter += 1;
            let ang = rnd(p.seed, p.counter * 3) * std::f32::consts::TAU;
            let r = radius * rnd(p.seed, p.counter * 3 + 1).sqrt();
            let (x, z) = (center.x + ang.cos() * r, center.y + ang.sin() * r);
            let g = field.height_at(x, z);
            let top = cam.y.max(g) + above;
            // Nuove o rimaste fuori dal cerchio (la camera si è girata): a un'altezza
            // qualsiasi, così la vista non resta vuota mentre ricadono dall'alto.
            let y = if p.fresh || out {
                g + rnd(p.seed, p.counter * 3 + 2) * (top - g)
            } else {
                top + rnd(p.seed, p.counter * 3 + 2) * 2.0
            };
            p.fresh = false;
            tf.translation = Vec3::new(x, y, z);
            continue;
        }
        let w = Vec3::new(wind.0.x, 0.0, wind.0.y);
        if p.snow {
            let sway = Vec3::new(
                (t * 1.1 + p.seed).sin(),
                0.0,
                (t * 0.8 + p.seed * 2.0).cos(),
            ) * 0.4;
            tf.translation += (w * 0.7 + sway + Vec3::NEG_Y * 1.4) * dt;
        } else {
            let vel = w * 0.9 + Vec3::NEG_Y * 15.0;
            tf.translation += vel * dt;
            tf.rotation = Quat::from_rotation_arc(Vec3::Y, -vel.normalize());
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Audio sintetizzato in locale (nessun file, nessuna rete)
// ---------------------------------------------------------------------------------------------

pub(crate) const RATE: u32 = 22_050;

pub(crate) struct Noise(pub(crate) u32);

impl Noise {
    /// Un passo del generatore con lo stato tenuto altrove.
    pub(crate) fn step(state: &mut u32) -> f32 {
        let mut n = Noise(*state);
        let v = n.next();
        *state = n.0;
        v
    }

    pub(crate) fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Rende un buffer ciclico senza cucitura: gli ultimi `c` campioni sfumano nell'inizio.
pub(crate) fn make_loop(buf: &[f32], n: usize) -> Vec<f32> {
    let c = buf.len() - n;
    let mut out = buf[..n].to_vec();
    for i in 0..c {
        let k = i as f32 / c as f32;
        out[i] = buf[i] * k + buf[n + i] * (1.0 - k);
    }
    out
}

pub(crate) fn to_wav(samples: &[f32], gain: f32) -> Vec<u8> {
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-3);
    let mut pcm = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        pcm.extend_from_slice(&(((s / peak) * gain * i16::MAX as f32) as i16).to_le_bytes());
    }
    crate::ai::tts::pcm_to_wav(&pcm, RATE)
}

/// Pioggia: rumore filtrato (banda medio-alta) con piccoli picchi di gocce. Ciclo di 8 s.
pub fn synth_rain() -> Vec<u8> {
    let n = RATE as usize * 8;
    let c = RATE as usize / 2;
    let mut rng = Noise(0x1234_5678);
    let (mut lp_a, mut lp_b) = (0.0f32, 0.0f32);
    let mut buf = Vec::with_capacity(n + c);
    for _ in 0..(n + c) {
        let w = rng.next();
        lp_a += (w - lp_a) * 0.55;
        lp_b += (w - lp_b) * 0.04;
        let mut s = (lp_a - lp_b) * 0.8;
        if rng.next() > 0.995 {
            s += rng.next() * 1.2;
        }
        buf.push(s);
    }
    to_wav(&make_loop(&buf, n), 0.6)
}

/// Vento: rumore a bassa frequenza con soffi lenti e un leggero fischio. Ciclo di 10 s.
pub fn synth_wind() -> Vec<u8> {
    let n = RATE as usize * 10;
    let c = RATE as usize / 2;
    let mut rng = Noise(0x9E37_79B9);
    let (mut lo, mut hi) = (0.0f32, 0.0f32);
    let mut buf = Vec::with_capacity(n + c);
    for i in 0..(n + c) {
        let t = i as f32 / RATE as f32;
        let w = rng.next();
        lo += (w - lo) * 0.012;
        hi += (w - hi) * 0.07;
        // Modulazioni con periodi che dividono i 10 s del ciclo.
        let gust = 0.55
            + 0.45
                * (t * std::f32::consts::TAU * 0.3).sin()
                * (t * std::f32::consts::TAU * 0.2).cos();
        buf.push((lo * 4.0 + hi * 0.6) * gust);
    }
    to_wav(&make_loop(&buf, n), 0.6)
}

/// Tuono: rombo di rumore a bassissima frequenza con attacco secco e coda lunga (5 s).
pub fn synth_thunder() -> Vec<u8> {
    let n = RATE as usize * 5;
    let mut rng = Noise(0xC0FF_EE11);
    let (mut a, mut b) = (0.0f32, 0.0f32);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        let w = rng.next();
        a += (w - a) * 0.02;
        b += (a - b) * 0.05;
        let env = (t * 30.0).min(1.0) * (-t * 0.9).exp();
        let rumble = 0.6 + 0.4 * (t * 9.0).sin() * (t * 3.7).cos();
        out.push((a * 3.0 + b * 4.0) * env * rumble);
    }
    to_wav(&out, 0.85)
}

#[derive(Resource)]
pub struct WeatherSounds {
    thunder: Handle<AudioSource>,
}

/// Anello di suono continuo (pioggia o vento).
#[derive(Component)]
pub struct WeatherLoop {
    rain: bool,
}

/// Avvio: sintetizza e avvia i due anelli (volume 0) e prepara il tuono.
pub fn setup_weather_audio(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let rain = sources.add(AudioSource {
        bytes: synth_rain().into(),
    });
    let wind = sources.add(AudioSource {
        bytes: synth_wind().into(),
    });
    let thunder = sources.add(AudioSource {
        bytes: synth_thunder().into(),
    });
    commands.insert_resource(WeatherSounds { thunder });
    for (handle, is_rain) in [(rain, true), (wind, false)] {
        commands.spawn((
            WeatherLoop { rain: is_rain },
            AudioPlayer::new(handle),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
        ));
    }
}

/// Sistema: volume di pioggia e vento secondo il meteo (più piano al coperto) e tuoni ritardati.
pub fn update_weather_audio(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<AudioSettings>,
    sounds: Option<Res<WeatherSounds>>,
    mut state: ResMut<WeatherState>,
    mut loops: Query<(&WeatherLoop, &mut AudioSink)>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
) {
    let master = settings.ambient_volume * 1.2;
    let inside = 1.0 - 0.6 * state.sheltered;
    for (l, mut sink) in &mut loops {
        let level = if l.rain {
            state.rain.max(state.snow * 0.0)
        } else {
            ((state.wind - 1.0) / 8.0).clamp(0.0, 1.0) * 0.8 + 0.05 * state.cloud
        };
        let target = Volume::Linear(master * level * inside);
        if sink.volume() != target {
            sink.set_volume(target);
        }
    }
    let dt = time.delta_secs();
    let mut fire = 0usize;
    state.thunder.retain_mut(|d| {
        *d -= dt;
        if *d <= 0.0 {
            fire += 1;
            false
        } else {
            true
        }
    });
    if let Some(sounds) = sounds {
        let cam = cameras
            .single()
            .map(|c| c.translation())
            .unwrap_or_default();
        for k in 0..fire {
            // Il fulmine cade da una parte del cielo, lontano: il tuono arriva da lì.
            let angle = rnd(time.elapsed_secs() + k as f32, 7) * std::f32::consts::TAU;
            commands.spawn((
                AudioPlayer::new(sounds.thunder.clone()),
                crate::audio::spatial_once(master * 1.3 * (1.0 - 0.4 * state.sheltered)),
                crate::audio::around(cam, angle, 150.0, 80.0),
            ));
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<WeatherState>()
        .init_resource::<SnowCover>()
        .add_systems(Startup, (spawn_precip, setup_weather_audio))
        .add_systems(
            Update,
            (
                update_weather,
                update_fog.after(update_weather),
                update_precip.after(update_weather),
                update_weather_audio.after(update_weather),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(month: u8, day: u8, year: u32) -> GameDate {
        GameDate { year, month, day }
    }

    #[test]
    fn weather_is_deterministic_per_date() {
        let d = date(10, 3, 2);
        assert_eq!(day_weather(&d), day_weather(&d));
    }

    #[test]
    fn precip_near_the_camera_is_hidden() {
        let cam = Vec3::new(0.0, 5.0, 0.0);
        assert!(too_close(Vec3::new(0.3, 5.2, -0.5), cam));
        assert!(!too_close(Vec3::new(0.0, 5.0, -3.0), cam));
        assert!(!too_close(Vec3::new(8.0, 1.0, 3.0), cam));
    }

    #[test]
    fn snow_only_in_cold_months_and_never_storms_in_winter() {
        for year in 0..40 {
            for (m, days) in [(6u8, 30u8), (7, 31), (8, 31)] {
                for d in 1..=days {
                    let w = day_weather(&date(m, d, year));
                    assert!(w.kind != Kind::Snow, "neve d'estate {m}/{d}/{year}");
                }
            }
            for (m, days) in [(12u8, 31u8), (1, 31), (2, 28)] {
                for d in 1..=days {
                    assert!(day_weather(&date(m, d, year)).kind != Kind::Storm);
                }
            }
        }
    }

    #[test]
    fn rain_turns_into_snow_when_freezing() {
        for year in 0..40 {
            for month in [12u8, 1, 2, 3] {
                for day in 1..=28 {
                    let d = date(month, day, year);
                    let w = day_weather(&d);
                    let mid =
                        temperature(d.day_of_year() as f32, w.start + w.len / 2.0, Kind::Clear);
                    if w.kind == Kind::Rain {
                        assert!(mid >= 1.5, "pioggia con {mid} °C il {day}/{month}");
                    }
                    if w.kind == Kind::Snow {
                        assert!(mid <= 3.0, "neve con {mid} °C il {day}/{month}");
                    }
                }
            }
        }
    }

    #[test]
    fn all_kinds_occur_over_a_year() {
        let mut seen = std::collections::HashSet::new();
        let mut d = GameDate::START;
        for _ in 0..365 * 4 {
            seen.insert(format!("{:?}", day_weather(&d).kind));
            d.next_day();
        }
        assert_eq!(seen.len(), 6, "{seen:?}");
    }

    #[test]
    fn temperature_follows_season_and_day() {
        let jan = temperature(20.0, 15.0, Kind::Clear);
        let jul = temperature(201.0, 15.0, Kind::Clear);
        assert!(jan < 8.0 && jul > 18.0, "{jan} {jul}");
        let night = temperature(201.0, 3.0, Kind::Clear);
        assert!(night < jul);
        assert!(temperature(201.0, 15.0, Kind::Snow) < jul);
    }

    #[test]
    fn windows_wrap_around_midnight() {
        let w = DayWeather {
            kind: Kind::Rain,
            start: 22.0,
            len: 5.0,
        };
        assert!(w.active(23.0) && w.active(2.0) && !w.active(12.0));
        assert!(
            !DayWeather {
                kind: Kind::Clear,
                start: 0.0,
                len: 0.0
            }
            .active(5.0)
        );
    }

    #[test]
    fn synthesized_sounds_are_valid_wavs() {
        for wav in [synth_rain(), synth_wind(), synth_thunder()] {
            assert_eq!(&wav[0..4], b"RIFF");
            assert!(wav.len() > 44 + RATE as usize);
        }
    }

    #[test]
    fn loops_are_seamless() {
        let buf: Vec<f32> = (0..1100).map(|i| (i as f32 * 0.1).sin()).collect();
        let out = make_loop(&buf, 1000);
        assert_eq!(out.len(), 1000);
        // L'ultimo campione e il primo sono contigui nel segnale originale.
        assert!((out[0] - buf[1000]).abs() < 0.2);
    }
}
