//! Ciclo giorno/notte: orologio di gioco, sole e luna (luci + dischi in cielo).
//!
//! 1 ora di gioco = 60 s reali (giorno = 24 min). Il sole sorge a est (+X)
//! alle 6, culmina alle 12 e tramonta a ovest alle 18; la luna è opposta.
//! Il tasto T apre "Aspetta" (stato `AppState::Wait`) per far avanzare l'ora.

use bevy::light::{NotShadowCaster, Skybox};
use bevy::prelude::*;

/// Secondi reali di un giorno di gioco (24 h).
pub const DAY_SECONDS: f32 = 24.0 * 60.0;
/// Distanza (m) dei dischi di sole/luna dalla camera.
const DISC_DISTANCE: f32 = 400.0;

/// Giorni dei mesi (anno di 365 giorni, nessun bisestile).
const MONTH_DAYS: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
/// Nomi abbreviati dei mesi.
const MONTH_NAMES: [&str; 12] = [
    "gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic",
];
/// Latitudine del mondo (gradi): Europa centrale.
const LATITUDE_DEG: f32 = 48.0;
/// Inclinazione dell'asse terrestre (gradi).
const AXIAL_TILT_DEG: f32 = 23.44;

/// Stagione astronomica (Europa centrale).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn name(self) -> &'static str {
        match self {
            Season::Spring => "Primavera",
            Season::Summer => "Estate",
            Season::Autumn => "Autunno",
            Season::Winter => "Inverno",
        }
    }
}

/// Data di gioco: anno (parte da 0), mese 1..=12, giorno 1..=31.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GameDate {
    pub year: u32,
    pub month: u8,
    pub day: u8,
}

impl GameDate {
    /// Data di partenza: 21 marzo, anno 0 (equinozio di primavera).
    pub const START: GameDate = GameDate {
        year: 0,
        month: 3,
        day: 21,
    };

    /// Passa al giorno successivo.
    pub fn next_day(&mut self) {
        if self.day < MONTH_DAYS[self.month as usize - 1] {
            self.day += 1;
        } else if self.month < 12 {
            self.month += 1;
            self.day = 1;
        } else {
            self.year += 1;
            self.month = 1;
            self.day = 1;
        }
    }

    /// Giorno dell'anno, 1..=365 (1 gennaio = 1).
    pub fn day_of_year(&self) -> u32 {
        MONTH_DAYS[..self.month as usize - 1]
            .iter()
            .map(|d| *d as u32)
            .sum::<u32>()
            + self.day as u32
    }

    /// Stagione astronomica: primavera dal 21/3, estate dal 21/6, autunno dal 23/9, inverno dal 21/12.
    pub fn season(&self) -> Season {
        match (self.month, self.day) {
            (1 | 2, _) | (3, 1..=20) | (12, 21..) => Season::Winter,
            (3, _) | (4 | 5, _) | (6, ..=20) => Season::Spring,
            (6, _) | (7 | 8, _) | (9, ..=22) => Season::Summer,
            _ => Season::Autumn,
        }
    }

    /// "21 mar, anno 0".
    pub fn label(&self) -> String {
        format!(
            "{} {}, anno {}",
            self.day,
            MONTH_NAMES[self.month as usize - 1],
            self.year
        )
    }

    /// Legge `anno,mese,giorno` (per `PLANET_ONE_DATE`).
    pub fn parse(text: &str) -> Option<Self> {
        let v: Vec<u32> = text
            .split(',')
            .filter_map(|x| x.trim().parse().ok())
            .collect();
        let [year, month, day] = v[..] else {
            return None;
        };
        if !(1..=12).contains(&month) || day < 1 || day > MONTH_DAYS[month as usize - 1] as u32 {
            return None;
        }
        Some(Self {
            year,
            month: month as u8,
            day: day as u8,
        })
    }
}

/// Declinazione del sole (radianti) al giorno dell'anno (frazionario).
fn declination(day_of_year: f32) -> f32 {
    AXIAL_TILT_DEG.to_radians() * ((day_of_year - 80.0) / 365.0 * std::f32::consts::TAU).sin()
}

/// Ore di alba e tramonto (ora solare locale) al giorno dell'anno.
pub fn sunrise_sunset(day_of_year: f32) -> (f32, f32) {
    let phi = LATITUDE_DEG.to_radians();
    let cos_h0 = (-phi.tan() * declination(day_of_year).tan()).clamp(-1.0, 1.0);
    let half = cos_h0.acos().to_degrees() / 15.0;
    (12.0 - half, 12.0 + half)
}

/// Ora di gioco (0..24) e data. Sopravvive al reset della scena.
#[derive(Resource, Clone)]
pub struct GameClock {
    pub hour: f32,
    pub date: GameDate,
    /// Ora di alba e tramonto del giorno corrente (aggiornate a ogni cambio di giorno).
    pub sunrise: f32,
    pub sunset: f32,
}

impl Default for GameClock {
    fn default() -> Self {
        // Ora iniziale: 10:00, o `PLANET_ONE_HOUR`; data: `PLANET_ONE_DATE=anno,mese,giorno`
        // (solo per provare il ciclo).
        let hour = std::env::var("PLANET_ONE_HOUR")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(10.0);
        let date = std::env::var("PLANET_ONE_DATE")
            .ok()
            .and_then(|v| {
                let parsed = GameDate::parse(&v);
                if parsed.is_none() {
                    warn!("[DEVTOOLS] PLANET_ONE_DATE={v} non valida: serve anno,mese,giorno");
                }
                parsed
            })
            .unwrap_or(GameDate::START);
        Self::new(date, hour)
    }
}

impl GameClock {
    pub fn new(date: GameDate, hour: f32) -> Self {
        let (sunrise, sunset) = sunrise_sunset(date.day_of_year() as f32);
        Self {
            hour,
            date,
            sunrise,
            sunset,
        }
    }

    /// "HH:MM".
    pub fn label(&self) -> String {
        let h = self.hour.rem_euclid(24.0);
        format!("{:02}:{:02}", h as u32, ((h.fract()) * 60.0) as u32)
    }

    /// "21 mar, anno 0 · Primavera".
    pub fn date_label(&self) -> String {
        format!("{} · {}", self.date.label(), self.date.season().name())
    }

    /// Giorno dell'anno frazionario (per il sole).
    pub fn day_fraction(&self) -> f32 {
        self.date.day_of_year() as f32 + self.hour / 24.0
    }

    /// Imposta data e ora (da un salvataggio).
    pub fn set(&mut self, date: GameDate, hour: f32) {
        self.date = date;
        self.hour = hour;
        let (r, s) = sunrise_sunset(self.date.day_of_year() as f32);
        self.sunrise = r;
        self.sunset = s;
    }

    /// Fa avanzare l'orologio di `hours` ore di gioco, cambiando data e orari del sole.
    pub fn advance(&mut self, hours: f32) {
        self.hour += hours;
        let mut changed = false;
        while self.hour >= 24.0 {
            self.hour -= 24.0;
            self.date.next_day();
            changed = true;
        }
        if changed {
            let (r, s) = sunrise_sunset(self.date.day_of_year() as f32);
            self.sunrise = r;
            self.sunset = s;
        }
    }

    /// Giorno "attivo" del villaggio: dall'alba a 3 h dopo il tramonto (cancelli aperti, Viandante sveglio).
    pub fn awake(&self) -> bool {
        (self.sunrise..self.sunset + 3.0).contains(&self.hour)
    }

    /// Lampioni accesi: da mezz'ora prima del tramonto a poco dopo l'alba.
    pub fn lamps_on(&self) -> bool {
        self.hour >= self.sunset - 0.5 || self.hour < self.sunrise + 0.2
    }

    /// Buio per faro (da un'ora dopo l'alba... a prima del tramonto è giorno).
    pub fn lighthouse_dark(&self) -> bool {
        !(self.sunrise + 0.5..self.sunset).contains(&self.hour)
    }

    /// Lanterne delle guardie: dal tramonto a un'ora dopo l'alba... (fuori da alba..tramonto+1).
    pub fn guard_lanterns_on(&self) -> bool {
        !(self.sunrise..self.sunset + 1.0).contains(&self.hour)
    }
}

/// Stato della finestra "Aspetta": ore da far passare.
#[derive(Resource)]
pub struct WaitUi {
    pub hours: u32,
}

impl Default for WaitUi {
    fn default() -> Self {
        Self { hours: 1 }
    }
}

/// Luce direzionale del sole.
#[derive(Component)]
pub struct Sun;
/// Luce direzionale della luna.
#[derive(Component)]
pub struct Moon;
/// Disco visibile in cielo (sole o luna).
#[derive(Component)]
pub struct SkyDisc {
    pub is_sun: bool,
}

/// Direzione (verso il sole) al giorno dell'anno e all'ora date: est = +X, sud = +Z, su = +Y.
/// Equinozio: sorge a est alle 6, tramonta a ovest alle 18; d'estate sale di più, d'inverno meno.
pub fn sun_dir(day_of_year: f32, hour: f32) -> Vec3 {
    let phi = LATITUDE_DEG.to_radians();
    let delta = declination(day_of_year);
    let h = (hour - 12.0) / 24.0 * std::f32::consts::TAU;
    let up = phi.sin() * delta.sin() + phi.cos() * delta.cos() * h.cos();
    let east = -delta.cos() * h.sin();
    let south = delta.cos() * h.cos() * phi.sin() - delta.sin() * phi.cos();
    Vec3::new(east, up, south).normalize()
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Sistema: avanza l'orologio in tempo reale (solo in gioco).
pub fn tick_clock(
    time: Res<Time>,
    settings: Res<crate::audio::AudioSettings>,
    mut clock: ResMut<GameClock>,
) {
    let speed = settings.time_speed.max(1) as f32;
    clock.advance(time.delta_secs() * 24.0 / DAY_SECONDS * speed);
}

/// Crea i dischi di sole e luna (seguono la camera, non fanno parte della scena).
pub fn spawn_sky_discs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let sphere = meshes.add(Sphere::new(1.0));
    for (is_sun, radius, color) in [
        (true, 16.0, Color::srgb(1.0, 0.93, 0.6)),
        (false, 10.0, Color::srgb(0.62, 0.68, 0.82)),
    ] {
        commands.spawn((
            SkyDisc { is_sun },
            NotShadowCaster,
            Mesh3d(sphere.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                unlit: true,
                ..default()
            })),
            Transform::from_scale(Vec3::splat(radius)),
            Visibility::Hidden,
        ));
    }
}

/// Sistema: posiziona luci e dischi e regola luce ambiente e cielo in base all'ora.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_sky(
    clock: Res<GameClock>,
    mut sun: Query<(&mut Transform, &mut DirectionalLight), (With<Sun>, Without<Moon>)>,
    mut moon: Query<(&mut Transform, &mut DirectionalLight), (With<Moon>, Without<Sun>)>,
    mut discs: Query<
        (&SkyDisc, &mut Transform, &mut Visibility),
        (Without<Sun>, Without<Moon>, Without<Camera3d>),
    >,
    cameras: Query<
        &Transform,
        (
            With<Camera3d>,
            Without<Sun>,
            Without<Moon>,
            Without<SkyDisc>,
        ),
    >,
    mut skyboxes: Query<(&mut Skybox, Option<&mut bevy::light::EnvironmentMapLight>)>,
    mut ambient: ResMut<GlobalAmbientLight>,
    season: Res<crate::seasons::SeasonLook>,
    weather: Res<crate::weather::WeatherState>,
) {
    let dir = sun_dir(clock.day_fraction().floor(), clock.hour);
    let elev = dir.y;
    let day = smoothstep(-0.1, 0.25, elev);
    let night_moon = smoothstep(-0.1, 0.25, -elev);
    let warm = 1.0 - smoothstep(0.0, 0.45, elev);

    if let Ok((mut t, mut light)) = sun.single_mut() {
        *t = Transform::from_translation(dir * 20.0).looking_at(Vec3::ZERO, Vec3::Y);
        light.illuminance = 9000.0 * day * (1.0 - 0.78 * weather.cloud) + weather.flash * 9000.0;
        let c = Vec3::new(1.0, 0.95, 0.85).lerp(Vec3::new(1.0, 0.55, 0.3), warm)
            * Vec3::from(season.0.sun);
        let c = c.lerp(Vec3::new(0.8, 0.85, 0.9), 0.6 * weather.cloud);
        light.color = Color::srgb(c.x, c.y, c.z);
    }
    if let Ok((mut t, mut light)) = moon.single_mut() {
        *t = Transform::from_translation(-dir * 20.0).looking_at(Vec3::ZERO, Vec3::Y);
        light.illuminance = 160.0 * night_moon;
    }

    let amb = Vec3::new(0.35, 0.42, 0.7).lerp(Vec3::new(0.9, 0.92, 0.98), day);
    ambient.color = Color::srgb(amb.x, amb.y, amb.z);
    ambient.brightness =
        (8.0 + (40.0 - 8.0) * day) * season.0.ambient * (1.0 - 0.15 * weather.cloud)
            + weather.flash * 700.0;

    let sky = smoothstep(-0.15, 0.3, elev);
    for (mut s, env) in &mut skyboxes {
        let b = (80.0 + (1500.0 - 80.0) * sky) * (1.0 - 0.6 * weather.cloud.max(weather.fog))
            + weather.flash * 2200.0;
        s.brightness = b;
        if let Some(mut env) = env {
            // Bevy crea `EnvironmentMapLight` una sola volta dalla versione "generata": l'intensità si regola qui.
            // Di notte il cielo non deve illuminare quasi nulla.
            env.intensity = (6.0 + 1000.0 * sky) * (1.0 - 0.6 * weather.cloud.max(weather.fog))
                + weather.flash * 900.0;
        }
    }

    let cam = cameras.single().map(|t| t.translation).unwrap_or_default();
    for (disc, mut t, mut vis) in &mut discs {
        let d = if disc.is_sun { dir } else { -dir };
        t.translation = cam + d * DISC_DISTANCE;
        *vis = if d.y > -0.05 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_date_is_march_21_year_0() {
        let d = GameDate::START;
        assert_eq!(d.label(), "21 mar, anno 0");
        assert_eq!(d.day_of_year(), 80);
        assert_eq!(d.season(), Season::Spring);
    }

    #[test]
    fn dates_roll_over_months_and_years() {
        let mut d = GameDate {
            year: 0,
            month: 2,
            day: 28,
        };
        d.next_day();
        assert_eq!((d.month, d.day), (3, 1));
        let mut d = GameDate {
            year: 0,
            month: 12,
            day: 31,
        };
        d.next_day();
        assert_eq!((d.year, d.month, d.day), (1, 1, 1));
        // Un anno intero di giorni riporta alla stessa data.
        let mut d = GameDate::START;
        for _ in 0..365 {
            d.next_day();
        }
        assert_eq!(
            d,
            GameDate {
                year: 1,
                ..GameDate::START
            }
        );
    }

    #[test]
    fn seasons_follow_astronomical_dates() {
        let s = |month, day| {
            GameDate {
                year: 0,
                month,
                day,
            }
            .season()
        };
        assert_eq!(s(3, 20), Season::Winter);
        assert_eq!(s(3, 21), Season::Spring);
        assert_eq!(s(6, 20), Season::Spring);
        assert_eq!(s(6, 21), Season::Summer);
        assert_eq!(s(9, 22), Season::Summer);
        assert_eq!(s(9, 23), Season::Autumn);
        assert_eq!(s(12, 20), Season::Autumn);
        assert_eq!(s(12, 21), Season::Winter);
        assert_eq!(s(1, 1), Season::Winter);
    }

    #[test]
    fn sunrise_and_sunset_follow_the_season() {
        let doy = |m, d| {
            GameDate {
                year: 0,
                month: m,
                day: d,
            }
            .day_of_year() as f32
        };
        let (r, t) = sunrise_sunset(doy(3, 21));
        assert!((r - 6.0).abs() < 0.1 && (t - 18.0).abs() < 0.1);
        let (rs, ts) = sunrise_sunset(doy(6, 21));
        assert!(rs < 4.6 && ts > 19.4, "estate: {rs} {ts}");
        let (rw, tw) = sunrise_sunset(doy(12, 21));
        assert!(rw > 7.4 && tw < 16.6, "inverno: {rw} {tw}");
    }

    #[test]
    fn sun_is_higher_in_summer_than_in_winter() {
        let noon = |m, d| {
            sun_dir(
                GameDate {
                    year: 0,
                    month: m,
                    day: d,
                }
                .day_of_year() as f32,
                12.0,
            )
            .y
        };
        assert!(noon(6, 21) > noon(3, 21) && noon(3, 21) > noon(12, 21));
        // All'equinozio alle 6 il sole è all'orizzonte, a est.
        let d = sun_dir(80.0, 6.0);
        assert!(d.y.abs() < 0.05 && d.x > 0.9);
    }

    #[test]
    fn advancing_the_clock_changes_the_day() {
        let mut c = GameClock::new(GameDate::START, 23.0);
        c.advance(2.0);
        assert_eq!((c.date.day, c.date.month), (22, 3));
        assert!((c.hour - 1.0).abs() < 1e-4);
        c.advance(48.0);
        assert_eq!(c.date.day, 24);
    }
}
