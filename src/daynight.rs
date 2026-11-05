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

/// Ora di gioco (0..24). Sopravvive al reset della scena.
#[derive(Resource)]
pub struct GameClock {
    pub hour: f32,
}

impl Default for GameClock {
    fn default() -> Self {
        // Ora iniziale: 10:00, o `PLANET_ONE_HOUR` (solo per provare il ciclo).
        let hour = std::env::var("PLANET_ONE_HOUR")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(10.0);
        Self { hour }
    }
}

impl GameClock {
    /// "HH:MM".
    pub fn label(&self) -> String {
        let h = self.hour.rem_euclid(24.0);
        format!("{:02}:{:02}", h as u32, ((h.fract()) * 60.0) as u32)
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

/// Direzione (verso il sole) all'ora data; leggera inclinazione verso sud (+Z).
pub fn sun_dir(hour: f32) -> Vec3 {
    let a = (hour - 6.0) / 24.0 * std::f32::consts::TAU;
    Vec3::new(a.cos(), a.sin(), 0.3).normalize()
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Sistema: avanza l'orologio in tempo reale (solo in gioco).
pub fn tick_clock(time: Res<Time>, mut clock: ResMut<GameClock>) {
    clock.hour = (clock.hour + time.delta_secs() * 24.0 / DAY_SECONDS).rem_euclid(24.0);
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
    mut skyboxes: Query<&mut Skybox>,
    mut ambient: ResMut<GlobalAmbientLight>,
) {
    let dir = sun_dir(clock.hour);
    let elev = dir.y;
    let day = smoothstep(-0.1, 0.25, elev);
    let night_moon = smoothstep(-0.1, 0.25, -elev);
    let warm = 1.0 - smoothstep(0.0, 0.45, elev);

    if let Ok((mut t, mut light)) = sun.single_mut() {
        *t = Transform::from_translation(dir * 20.0).looking_at(Vec3::ZERO, Vec3::Y);
        light.illuminance = 4000.0 * day;
        let c = Vec3::new(1.0, 0.95, 0.85).lerp(Vec3::new(1.0, 0.55, 0.3), warm);
        light.color = Color::srgb(c.x, c.y, c.z);
    }
    if let Ok((mut t, mut light)) = moon.single_mut() {
        *t = Transform::from_translation(-dir * 20.0).looking_at(Vec3::ZERO, Vec3::Y);
        light.illuminance = 700.0 * night_moon;
    }

    let amb = Vec3::new(0.35, 0.42, 0.7).lerp(Vec3::new(0.9, 0.92, 0.98), day);
    ambient.color = Color::srgb(amb.x, amb.y, amb.z);
    ambient.brightness = 70.0 + (80.0 - 70.0) * day;

    let sky = smoothstep(-0.15, 0.3, elev);
    for mut s in &mut skyboxes {
        s.brightness = 80.0 + (1000.0 - 80.0) * sky;
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
