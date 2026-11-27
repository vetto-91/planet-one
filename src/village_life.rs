//! Paese vissuto: fumo dai comignoli e torce sulle mura accese di notte.
//!
//! Carretti, botti, casse, panni e reti sono piazzamenti normali (`buildings::model`,
//! `scripts/gen_paese.py`): si spostano con F4 e i contenitori funzionano come gli altri.

use crate::ambience::daylight;
use crate::audio::AudioSettings;
use crate::daynight::GameClock;
use crate::hut::SmokePuff;
use crate::player::Player;
use crate::scene::SceneNode;
use crate::village::VillageInfo;
use crate::weather::WeatherState;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

/// Fin dove (m) si vede il fumo dei camini.
const SMOKE_RANGE: f32 = 75.0;
const MAX_SMOKING: usize = 10;
const MAX_PUFFS: usize = 70;
/// Una torcia ogni tanti metri di camminamento.
const TORCH_SPACING: f32 = 15.0;
const TORCH_LUMENS: f32 = 90_000.0;

/// Bocca di un camino. `works`: bottega che tiene il fuoco acceso tutto il giorno.
#[derive(Component)]
pub struct TownChimney {
    pub works: bool,
    /// Falò all'aperto: fuma sempre.
    pub always: bool,
    /// Focolare sotto il camino: se c'è, il fumo segue il suo stato.
    pub hearth: Option<Entity>,
}

/// Focolare della casa del giocatore (`true`): lo accende e lo spegne solo lui, non l'orario.
#[derive(Component)]
pub struct OwnHearth(pub bool);

/// Luce del focolare di una casa: segue lo stato del focolare.
#[derive(Component)]
pub struct HearthLight {
    pub state: Entity,
    pub seed: f32,
}

/// L'orologio è saltato (caricamento di un salvataggio, teletrasporto nel tempo)? In quel caso
/// gli stati salvati di focolari e scuri valgono più dell'orario.
fn clock_jumped(hour: f32, last: &mut Option<f32>) -> bool {
    let jumped = last.is_some_and(|l| (hour - l).rem_euclid(24.0) > 0.5);
    *last = Some(hour);
    jumped
}

/// Sistema: di giorno e di sera i focolari si accendono da soli ai cambi d'ora; il giocatore può
/// spegnerli o accenderli fino al prossimo cambio.
fn hearth_schedule(
    clock: Res<GameClock>,
    weather: Res<WeatherState>,
    mut hearths: Query<(
        Ref<crate::hut::PropKind>,
        &mut crate::hut::PropState,
        Option<&OwnHearth>,
    )>,
    mut last: Local<Option<bool>>,
    mut last_hour: Local<Option<f32>>,
) {
    let want = fire_lit(false, clock.hour, weather.temp);
    let jumped = clock_jumped(clock.hour, &mut last_hour);
    let changed = *last != Some(want) && !jumped;
    *last = Some(want);
    for (k, mut st, own) in &mut hearths {
        let own = own.is_some_and(|o| o.0);
        if *k == crate::hut::PropKind::Hearth && (changed && !own || k.is_added()) && st.on != want
        {
            st.on = want;
        }
    }
}

/// Scuri di una casa con un abitante: li apre e chiude lui (`villagers`), non l'orario.
#[derive(Component)]
pub struct ResidentShutter;

/// Sistema: gli scuri delle case senza abitanti e li chiudono la sera (e col maltempo forte).
fn shutter_schedule(
    clock: Res<GameClock>,
    weather: Res<WeatherState>,
    mut windows: Query<(
        Ref<crate::hut::PropKind>,
        &crate::hut::PropId,
        &mut crate::hut::PropState,
        Has<ResidentShutter>,
    )>,
    mut last: Local<Option<bool>>,
    mut last_hour: Local<Option<f32>>,
) {
    let open = (7.0..19.5).contains(&clock.hour) && weather.cloud < 0.95;
    let jumped = clock_jumped(clock.hour, &mut last_hour);
    let changed = *last != Some(open) && !jumped;
    *last = Some(open);
    for (k, id, mut st, resident) in &mut windows {
        if *k == crate::hut::PropKind::Window
            && id.0.starts_with("scuri:")
            && (changed && !resident || k.is_added())
            && st.on != open
        {
            st.on = open;
        }
    }
}

/// Sistema: bagliore tremolante del focolare acceso.
fn hearth_glow(
    time: Res<Time>,
    states: Query<&crate::hut::PropState>,
    mut lights: Query<(&HearthLight, &mut PointLight)>,
) {
    let t = time.elapsed_secs();
    for (h, mut l) in &mut lights {
        let on = states.get(h.state).is_ok_and(|s| s.on);
        let target = if on {
            70_000.0 * (0.85 + 0.15 * (t * 9.0 + h.seed).sin() * (t * 3.7 + h.seed).cos())
        } else {
            0.0
        };
        l.intensity += (target - l.intensity) * (time.delta_secs() * 4.0).min(1.0);
    }
}

/// Pale del mulino.
#[derive(Component)]
pub struct MillSails;

/// Fuoco di un falò: fiamma e luce tremolano.
#[derive(Component)]
pub struct Campfire {
    seed: f32,
}

/// Sbuffo uscito da un camino del paese (non conta nel limite della stufa della casetta).
#[derive(Component)]
pub struct TownPuff;

#[derive(Component)]
struct WallTorch {
    seed: f32,
}

#[derive(Component)]
struct TorchFlame;

/// Il fuoco di una casa è acceso a quest'ora con questa temperatura?
pub fn fire_lit(works: bool, hour: f32, temp: f32) -> bool {
    if works {
        return (6.0..21.0).contains(&hour);
    }
    (6.0..9.5).contains(&hour) || (17.0..23.0).contains(&hour) || temp < 9.0
}

/// Punti a passo costante lungo il percorso chiuso `path`.
pub fn points_every(path: &[Vec3], spacing: f32) -> Vec<Vec3> {
    let mut out = Vec::new();
    let mut left = spacing / 2.0;
    for (i, a) in path.iter().enumerate() {
        let b = path[(i + 1) % path.len()];
        let len = a.xz().distance(b.xz());
        // Le torri sono salti di quota: lì non serve una torcia a mezz'aria.
        if len < 1e-3 || (a.y - b.y).abs() > 0.6 {
            continue;
        }
        let mut at = left;
        while at < len {
            out.push(a.lerp(b, at / len));
            at += spacing;
        }
        left = at - len;
    }
    out
}

/// Sistema: sbuffi dai camini accesi più vicini al giocatore, a turno.
#[allow(clippy::too_many_arguments)]
fn town_smoke(
    mut commands: Commands,
    time: Res<Time>,
    clock: Res<GameClock>,
    weather: Res<WeatherState>,
    player: Query<&Transform, With<Player>>,
    chimneys: Query<(&TownChimney, &GlobalTransform)>,
    hearth_states: Query<&crate::hut::PropState>,
    puffs: Query<(), With<TownPuff>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: Local<(f32, usize, usize)>,
    mut mesh: Local<Option<Handle<Mesh>>>,
) {
    state.0 -= time.delta_secs();
    if state.0 > 0.0 || puffs.iter().count() >= MAX_PUFFS {
        return;
    }
    state.0 = 0.11;
    let Ok(me) = player.single() else {
        return;
    };
    let me = me.translation;
    let mut lit: Vec<(f32, Vec3, bool)> = chimneys
        .iter()
        .filter(|(c, _)| match c.hearth {
            Some(h) => hearth_states.get(h).is_ok_and(|s| s.on),
            None => c.always || fire_lit(c.works, clock.hour, weather.temp),
        })
        .map(|(c, g)| {
            (
                g.translation().distance_squared(me),
                g.translation(),
                c.always,
            )
        })
        .filter(|(d, ..)| *d < SMOKE_RANGE * SMOKE_RANGE)
        .collect();
    if lit.is_empty() {
        return;
    }
    if state.2 != lit.len() {
        state.2 = lit.len();
        info!("[PAESE] camini accesi vicini: {}", lit.len());
    }
    lit.sort_by(|a, b| a.0.total_cmp(&b.0));
    lit.truncate(MAX_SMOKING);
    state.1 = (state.1 + 1) % lit.len();
    let (_, top, campfire) = lit[state.1];
    let t = time.elapsed_secs();
    let m = mesh
        .get_or_insert_with(|| meshes.add(Sphere::new(0.5)))
        .clone();
    let push = 0.14 + weather.wind * 0.07;
    commands.spawn((
        SceneNode,
        TownPuff,
        SmokePuff {
            age: 0.0,
            life: 6.0,
            size: if campfire { 1.2 } else { 3.5 },
            grey: if campfire { 0.55 } else { 0.4 },
            alpha: if campfire { 0.35 } else { 0.78 },
            drift: Vec2::new(
                push + (t * 12.9).sin() * 0.05,
                push * 0.4 + (t * 78.2).cos() * 0.05,
            ),
        },
        Mesh3d(m),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.42, 0.42, 0.45, 0.5),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_translation(top).with_scale(Vec3::splat(0.45)),
        NotShadowCaster,
    ));
}

/// Sistema: (ri)crea le torce quando cambia il percorso sulle mura.
fn sync_torches(
    mut commands: Commands,
    village: Res<VillageInfo>,
    existing: Query<Entity, With<WallTorch>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut built: Local<(usize, Option<Vec3>)>,
) {
    let stamp = (village.patrol.len(), village.patrol.first().copied());
    if village.patrol.is_empty() || (*built == stamp && !existing.is_empty()) {
        return;
    }
    *built = stamp;
    for e in &existing {
        commands.entity(e).despawn();
    }
    let post = meshes.add(Cuboid::new(0.07, 1.5, 0.07));
    let flame = meshes.add(Sphere::new(0.085).mesh().uv(8, 6));
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.17, 0.1),
        perceptual_roughness: 0.9,
        ..default()
    });
    let fire = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.6, 0.2),
        emissive: LinearRgba::rgb(6.0, 2.4, 0.5),
        ..default()
    });
    let spots = points_every(&village.patrol, TORCH_SPACING);
    info!("[PAESE] {} torce sulle mura", spots.len());
    for (i, at) in spots.into_iter().enumerate() {
        // Sul lato verso il paese, per non intralciare la ronda.
        let inward = (village.center - at.xz()).normalize_or_zero() * 0.8;
        let base = at + Vec3::new(inward.x, 0.0, inward.y);
        commands
            .spawn((
                SceneNode,
                WallTorch {
                    seed: i as f32 * 2.3,
                },
                Transform::from_translation(base),
                Visibility::default(),
            ))
            .with_children(|p| {
                p.spawn((
                    Mesh3d(post.clone()),
                    MeshMaterial3d(wood.clone()),
                    Transform::from_xyz(0.0, 0.75, 0.0),
                ));
                p.spawn((
                    TorchFlame,
                    Mesh3d(flame.clone()),
                    MeshMaterial3d(fire.clone()),
                    Transform::from_xyz(0.0, 1.62, 0.0),
                    NotShadowCaster,
                    Visibility::Hidden,
                ));
                p.spawn((
                    PointLight {
                        color: Color::srgb(1.0, 0.68, 0.36),
                        intensity: 0.0,
                        range: 13.0,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 1.75, 0.0),
                ));
            });
    }
}

/// Sistema: le torce si accendono al buio e la fiamma trema.
#[allow(clippy::type_complexity)]
fn light_torches(
    time: Res<Time>,
    clock: Res<GameClock>,
    settings: Res<AudioSettings>,
    torches: Query<(&WallTorch, &Children)>,
    mut flames: Query<(&mut Visibility, &mut Transform), With<TorchFlame>>,
    mut lights: Query<&mut PointLight>,
) {
    let dark = daylight(clock.hour, clock.sunrise, clock.sunset) < 0.45;
    let now = time.elapsed_secs();
    for (torch, children) in &torches {
        let flicker = 1.0 + 0.16 * (now * 11.0 + torch.seed).sin() * (now * 7.3 + torch.seed).cos();
        for child in children.iter() {
            if let Ok((mut vis, mut t)) = flames.get_mut(child) {
                vis.set_if_neq(if dark {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
                // Di giorno la fiamma è nascosta: niente scala da ripropagare (ADR 0056).
                if dark {
                    t.scale = Vec3::new(flicker, flicker * 1.6, flicker);
                }
            }
            if let Ok(mut light) = lights.get_mut(child) {
                let target = if dark && settings.gfx.interior_lights {
                    TORCH_LUMENS * flicker
                } else {
                    0.0
                };
                if light.intensity != target {
                    light.intensity = target;
                }
            }
        }
    }
}

/// Aggiunge a un luogo un falò acceso, con fumo e luce.
pub fn spawn_campfire(
    parent: &mut ChildSpawnerCommands,
    at: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Sottoterra (gallerie della grotta) il fumo non ha dove uscire: niente sbuffi.
    if at.y > -0.5 {
        parent.spawn((
            TownChimney {
                works: true,
                always: true,
                hearth: None,
            },
            Transform::from_translation(at + Vec3::Y * 0.6),
        ));
    }
    parent.spawn((
        Campfire { seed: at.x * 3.1 },
        Mesh3d(meshes.add(Sphere::new(0.2).mesh().uv(8, 6))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.55, 0.15),
            emissive: LinearRgba::rgb(9.0, 3.4, 0.6),
            ..default()
        })),
        PointLight {
            color: Color::srgb(1.0, 0.62, 0.3),
            intensity: 60_000.0,
            range: 11.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(at),
        NotShadowCaster,
    ));
}

/// Sistema: le fiamme dei falò tremano.
fn flicker_fires(time: Res<Time>, mut fires: Query<(&Campfire, &mut Transform, &mut PointLight)>) {
    let now = time.elapsed_secs();
    for (fire, mut t, mut light) in &mut fires {
        let k = 1.0 + 0.2 * (now * 9.0 + fire.seed).sin() * (now * 6.1 + fire.seed).cos();
        t.scale = Vec3::new(k, k * 1.7, k);
        light.intensity = 60_000.0 * k;
    }
}

/// Sistema: girano le pale del mulino, più in fretta col vento.
fn turn_sails(
    time: Res<Time>,
    weather: Res<WeatherState>,
    mut sails: Query<&mut Transform, With<MillSails>>,
) {
    let speed = 0.25 + weather.wind * 0.08;
    for mut t in &mut sails {
        t.rotate_local_z(-speed * time.delta_secs());
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            town_smoke,
            sync_torches,
            light_torches,
            flicker_fires,
            turn_sails,
            hearth_schedule,
            shutter_schedule,
            hearth_glow,
        ),
    )
    .add_systems(
        Update,
        burn_player.run_if(in_state(crate::AppState::Playing)),
    );
}

/// Danno di una scottatura (vita) e ogni quanti secondi si ripete restando nel fuoco (ADR 0064).
pub const BURN_DAMAGE: f32 = 4.0;
pub const BURN_EVERY: f32 = 0.8;
/// Raggio (m) delle fiamme di un falò e di un focolare, attorno al loro centro.
const CAMPFIRE_RADIUS: f32 = 0.45;
const HEARTH_RADIUS: f32 = 0.35;

/// I piedi del giocatore (centro del corpo `center`, capsula larga 0,35 m) sono nelle fiamme che
/// stanno in `fire` con raggio `radius`? Le fiamme salgono di 0,9 m dal loro centro.
pub fn in_flames(center: Vec3, fire: Vec3, radius: f32) -> bool {
    let feet = center.y - crate::player::FEET_BELOW_CENTER;
    let flat = Vec2::new(center.x - fire.x, center.z - fire.z).length();
    flat < radius + 0.2 && feet > fire.y - 0.6 && feet < fire.y + 0.9
}

/// Sistema: stare nelle fiamme di un falò o di un focolare acceso toglie vita a intervalli, con
/// uno sfrigolio e un avviso (segnalazione 2026-11-27_12-37-35, «il fuoco non fa danno»).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn burn_player(
    time: Res<Time>,
    cheats: Res<crate::shell::Cheats>,
    mut session: ResMut<crate::save::Session>,
    mut player: Query<
        (&GlobalTransform, &mut crate::player::PlayerHealth),
        With<crate::player::Player>,
    >,
    fires: Query<&GlobalTransform, With<Campfire>>,
    hearths: Query<(&GlobalTransform, &crate::world_props::HearthFlame)>,
    states: Query<&crate::hut::PropState>,
    mut wait: Local<f32>,
) {
    let Ok((body, mut hp)) = player.single_mut() else {
        return;
    };
    let p = body.translation();
    let burning = fires
        .iter()
        .any(|f| in_flames(p, f.translation(), CAMPFIRE_RADIUS))
        || hearths.iter().any(|(f, h)| {
            states.get(h.state).is_ok_and(|s| s.on) && in_flames(p, f.translation(), HEARTH_RADIUS)
        });
    if !burning {
        // Uscito dal fuoco: la prossima volta brucia subito.
        *wait = 0.0;
        return;
    }
    *wait -= time.delta_secs();
    if *wait > 0.0 {
        return;
    }
    *wait = BURN_EVERY;
    if cheats.god || hp.current <= 0.0 {
        return;
    }
    hp.current = (hp.current - BURN_DAMAGE).max(0.0);
    crate::sfx::play(crate::sfx::Sfx::Burn);
    session.notify("Ti stai bruciando!");
    info!("[FUOCO] scottatura, vita {:.0}", hp.current);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Il falò della grotta delle stele (segnalazione 2026-11-27_12-37-35): in piedi nel fuoco
    /// si brucia, accanto no, e nemmeno saltandoci sopra in alto.
    #[test]
    fn standing_in_a_fire_burns_but_beside_it_does_not() {
        let fire = Vec3::new(10.0, 1.0, 5.0);
        let stand = |dx: f32, dy: f32| Vec3::new(10.0 + dx, 1.0 + 0.85 + dy, 5.0);
        assert!(in_flames(stand(0.0, 0.0), fire, CAMPFIRE_RADIUS));
        assert!(in_flames(stand(0.6, 0.0), fire, CAMPFIRE_RADIUS));
        assert!(!in_flames(stand(1.0, 0.0), fire, CAMPFIRE_RADIUS));
        assert!(!in_flames(stand(0.0, 1.5), fire, CAMPFIRE_RADIUS));
        // Il focolare è più piccolo.
        assert!(!in_flames(stand(0.6, 0.0), fire, HEARTH_RADIUS));
    }

    #[test]
    fn clock_jumps_are_told_from_normal_time() {
        let mut last = None;
        assert!(!clock_jumped(8.0, &mut last));
        assert!(!clock_jumped(8.01, &mut last));
        assert!(clock_jumped(19.0, &mut last));
        assert!(clock_jumped(3.0, &mut last));
        assert!(clock_jumped(23.99, &mut last));
        assert!(
            !clock_jumped(0.01, &mut last),
            "la mezzanotte non è un salto"
        );
    }

    #[test]
    fn houses_light_the_fire_morning_and_evening_or_when_cold() {
        assert!(fire_lit(false, 7.0, 20.0));
        assert!(fire_lit(false, 20.0, 20.0));
        assert!(!fire_lit(false, 13.0, 20.0));
        assert!(fire_lit(false, 13.0, 4.0));
        assert!(!fire_lit(false, 3.0, 20.0));
    }

    #[test]
    fn workshops_burn_all_day() {
        assert!(fire_lit(true, 13.0, 25.0));
        assert!(!fire_lit(true, 23.0, 25.0));
    }

    #[test]
    fn torches_are_evenly_spaced_and_skip_jumps() {
        let square = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(30.0, 1.0, 0.0),
            Vec3::new(30.0, 1.0, 30.0),
            Vec3::new(0.0, 1.0, 30.0),
        ];
        let pts = points_every(&square, 15.0);
        assert_eq!(pts.len(), 8);
        for w in pts.windows(2) {
            assert!(w[0].distance(w[1]) <= 15.0 + 1e-3);
        }
        let jump = [Vec3::ZERO, Vec3::new(20.0, 3.0, 0.0)];
        assert!(points_every(&jump, 5.0).is_empty());
    }
}
