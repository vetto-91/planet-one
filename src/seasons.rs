//! Stagioni visibili (ADR 0012): tinta di erba e sabbia, chiome degli alberi, fiori, neve a
//! terra e sui tetti, foglie che cadono. Tutto deriva da `SeasonLook`, calcolata dal giorno
//! dell'anno con fotogrammi chiave interpolati (nessuno scatto da una stagione all'altra).

use crate::daynight::GameClock;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

/// Aspetto del mondo in un dato giorno dell'anno.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// Moltiplicatore del colore dell'erba.
    pub grass: [f32; 3],
    /// Chiome tinte d'autunno (0..1).
    pub autumn: f32,
    /// Neve sulle chiome (0..1).
    pub cap: f32,
    /// Fiori sbocciati (0..1).
    pub flowers: f32,
    /// Neve a terra e sui tetti (0..1).
    pub snow: f32,
    /// Foglie in caduta (0..1).
    pub leaf_fall: f32,
    /// Tinta della luce del sole.
    pub sun: [f32; 3],
    /// Moltiplicatore della luce ambiente.
    pub ambient: f32,
}

#[allow(clippy::too_many_arguments)]
const fn k(
    doy: f32,
    grass: [f32; 3],
    autumn: f32,
    cap: f32,
    flowers: f32,
    snow: f32,
    leaf_fall: f32,
    sun: [f32; 3],
    ambient: f32,
) -> (f32, Look) {
    (
        doy,
        Look {
            grass,
            autumn,
            cap,
            flowers,
            snow,
            leaf_fall,
            sun,
            ambient,
        },
    )
}

/// Fotogrammi chiave (giorno dell'anno, 1 gennaio = 1). L'ultimo si raccorda col primo.
const KEYS: [(f32, Look); 12] = [
    k(
        1.0,
        [0.8, 0.85, 0.92],
        0.0,
        1.0,
        0.0,
        1.0,
        0.0,
        [0.92, 0.96, 1.05],
        0.92,
    ),
    k(
        50.0,
        [0.8, 0.85, 0.92],
        0.0,
        0.9,
        0.0,
        0.9,
        0.0,
        [0.94, 0.97, 1.03],
        0.94,
    ),
    k(
        76.0,
        [0.9, 1.0, 0.86],
        0.0,
        0.1,
        0.0,
        0.0,
        0.0,
        [0.97, 1.0, 1.0],
        0.98,
    ),
    k(
        105.0,
        [0.88, 1.1, 0.84],
        0.0,
        0.0,
        0.8,
        0.0,
        0.0,
        [1.0, 1.0, 0.98],
        1.0,
    ),
    k(
        135.0,
        [0.9, 1.1, 0.85],
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        [1.02, 1.0, 0.96],
        1.02,
    ),
    k(
        172.0,
        [1.0, 1.02, 0.82],
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        [1.05, 1.0, 0.93],
        1.05,
    ),
    k(
        230.0,
        [1.05, 0.95, 0.72],
        0.1,
        0.0,
        0.6,
        0.0,
        0.0,
        [1.05, 0.99, 0.9],
        1.03,
    ),
    k(
        266.0,
        [1.1, 0.9, 0.66],
        0.4,
        0.0,
        0.2,
        0.0,
        0.5,
        [1.05, 0.97, 0.9],
        1.0,
    ),
    k(
        300.0,
        [1.05, 0.8, 0.56],
        1.0,
        0.0,
        0.0,
        0.0,
        1.0,
        [1.04, 0.95, 0.88],
        0.96,
    ),
    k(
        330.0,
        [0.9, 0.8, 0.66],
        0.5,
        0.2,
        0.0,
        0.0,
        0.5,
        [0.98, 0.96, 0.96],
        0.94,
    ),
    k(
        346.0,
        [0.84, 0.84, 0.8],
        0.0,
        0.8,
        0.0,
        0.6,
        0.1,
        [0.93, 0.96, 1.03],
        0.92,
    ),
    k(
        358.0,
        [0.8, 0.85, 0.92],
        0.0,
        1.0,
        0.0,
        1.0,
        0.0,
        [0.92, 0.96, 1.05],
        0.92,
    ),
];

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Aspetto al giorno dell'anno `doy` (1..=366, anche frazionario).
pub fn look(doy: f32) -> Look {
    let doy = ((doy - 1.0).rem_euclid(365.0)) + 1.0;
    // Coppia di fotogrammi che racchiude `doy` (con raccordo ciclico).
    let n = KEYS.len();
    let i = (0..n).rev().find(|&i| KEYS[i].0 <= doy).unwrap_or(n - 1);
    let (d0, a) = KEYS[i];
    let (mut d1, b) = KEYS[(i + 1) % n];
    if d1 <= d0 {
        d1 += 365.0;
    }
    let d = if doy < d0 { doy + 365.0 } else { doy };
    let t = ((d - d0) / (d1 - d0)).clamp(0.0, 1.0);
    let t = t * t * (3.0 - 2.0 * t);
    let f = |x: f32, y: f32| x + (y - x) * t;
    Look {
        grass: lerp3(a.grass, b.grass, t),
        autumn: f(a.autumn, b.autumn),
        cap: f(a.cap, b.cap),
        flowers: f(a.flowers, b.flowers),
        snow: f(a.snow, b.snow),
        leaf_fall: f(a.leaf_fall, b.leaf_fall),
        sun: lerp3(a.sun, b.sun, t),
        ambient: f(a.ambient, b.ambient),
    }
}

/// Aspetto corrente (aggiornato ogni quarto di giorno di gioco).
#[derive(Resource)]
pub struct SeasonLook(pub Look);

impl Default for SeasonLook {
    fn default() -> Self {
        Self(look(80.0))
    }
}

/// Materiale del terreno (erba) o della sabbia, tinto dalla stagione.
#[derive(Component)]
pub struct SeasonGround {
    /// La sabbia segue la stagione solo in parte.
    pub sand: bool,
}

/// Chioma di un albero (`0..3` = variante di colore).
#[derive(Component)]
pub struct SeasonLeaf(pub usize);

/// Fiore: sboccia quando `flowers` supera la sua soglia.
#[derive(Component)]
pub struct SeasonFlower {
    pub threshold: f32,
}

/// Manto di neve sui tetti.
#[derive(Component)]
pub struct SeasonSnowRoof;

/// Strato di neve sul terreno (alfa = quantità di neve).
#[derive(Component)]
pub struct SeasonSnowGround;

/// Foglia in caduta intorno alla camera.
#[derive(Component)]
pub struct FallingLeaf {
    seed: f32,
}

const LEAF_GREEN: [[f32; 3]; 3] = [[0.16, 0.5, 0.2], [0.22, 0.58, 0.24], [0.12, 0.42, 0.22]];
const LEAF_AUTUMN: [[f32; 3]; 3] = [[0.85, 0.4, 0.1], [0.8, 0.62, 0.12], [0.65, 0.2, 0.1]];
const LEAF_SNOW: [f32; 3] = [0.9, 0.94, 0.98];
const POOL: usize = 48;

/// Colore (RGB) della chioma `i` nel `look` dato.
pub fn leaf_color(l: &Look, i: usize) -> [f32; 3] {
    let i = i % 3;
    let c = lerp3(LEAF_GREEN[i], LEAF_AUTUMN[i], l.autumn);
    lerp3(c, LEAF_SNOW, l.cap)
}

/// Sistema: ricalcola il `look` quando cambia il quarto di giorno.
pub fn update_look(clock: Res<GameClock>, mut season: ResMut<SeasonLook>, mut last: Local<i64>) {
    let bucket = (clock.day_fraction() * 4.0) as i64 + clock.date.year as i64 * 365 * 4;
    if bucket == *last {
        return;
    }
    *last = bucket;
    let new = look(clock.day_fraction());
    if new != season.0 {
        season.0 = new;
    }
}

/// Sistema: applica il `look` a materiali, fiori e neve (solo quando cambia o compaiono nuovi oggetti).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn apply_season(
    season: Res<SeasonLook>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    ground: Query<(&SeasonGround, &MeshMaterial3d<StandardMaterial>)>,
    leaves: Query<(&SeasonLeaf, &MeshMaterial3d<StandardMaterial>)>,
    mut flowers: Query<(&SeasonFlower, &mut Transform)>,
    mut roofs: Query<&mut Visibility, (With<SeasonSnowRoof>, Without<SeasonSnowGround>)>,
    mut snow_ground: Query<
        (&mut Visibility, &MeshMaterial3d<StandardMaterial>),
        (With<SeasonSnowGround>, Without<SeasonSnowRoof>),
    >,
    fresh: Query<
        (),
        Or<(
            Added<SeasonGround>,
            Added<SeasonLeaf>,
            Added<SeasonFlower>,
            Added<SeasonSnowRoof>,
            Added<SeasonSnowGround>,
        )>,
    >,
) {
    if !season.is_changed() && fresh.is_empty() {
        return;
    }
    let l = season.0;
    for (g, m) in &ground {
        if let Some(mut mat) = materials.get_mut(&m.0) {
            let mix = if g.sand { 0.35 } else { 1.0 };
            let c = lerp3([1.0; 3], l.grass, mix);
            mat.base_color = Color::srgb(c[0], c[1], c[2]);
        }
    }
    for (leaf, m) in &leaves {
        if let Some(mut mat) = materials.get_mut(&m.0) {
            let c = leaf_color(&l, leaf.0);
            mat.base_color = Color::srgb(c[0], c[1], c[2]);
        }
    }
    for (f, mut tf) in &mut flowers {
        let s = ((l.flowers - f.threshold) * 4.0).clamp(0.0, 1.0);
        tf.scale = Vec3::splat(s.max(0.0001));
    }
    for mut v in &mut roofs {
        *v = if l.snow > 0.3 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (mut v, m) in &mut snow_ground {
        *v = if l.snow > 0.02 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(mut mat) = materials.get_mut(&m.0) {
            mat.base_color = Color::srgba(0.93, 0.96, 1.0, (l.snow * 0.97).min(1.0));
        }
    }
}

/// Avvio: crea il gruppo di foglie in caduta (nascoste finché non è autunno).
pub fn spawn_leaves(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(0.14, 0.02, 0.1));
    let mats: Vec<_> = LEAF_AUTUMN
        .iter()
        .map(|c| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(c[0], c[1], c[2]),
                unlit: true,
                ..default()
            })
        })
        .collect();
    for i in 0..POOL {
        commands.spawn((
            FallingLeaf {
                seed: i as f32 * 1.618,
            },
            NotShadowCaster,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mats[i % 3].clone()),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}

/// Sistema: muove le foglie in un riquadro attorno alla camera; attive in proporzione a `leaf_fall`.
pub fn update_leaves(
    time: Res<Time>,
    season: Res<SeasonLook>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut leaves: Query<(&FallingLeaf, &mut Transform, &mut Visibility), Without<Camera3d>>,
) {
    let Ok(cam) = cameras.single() else {
        return;
    };
    let cam = cam.translation();
    let active = (season.0.leaf_fall * POOL as f32) as usize;
    let t = time.elapsed_secs();
    for (i, (leaf, mut tf, mut vis)) in leaves.iter_mut().enumerate() {
        if i >= active {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;
        let s = leaf.seed;
        // Caduta lenta (0,9 m/s) in un ciclo di 10 m, con oscillazione laterale.
        let fall = (t * 0.9 + s * 7.3).rem_euclid(10.0);
        let x = (s * 12.9898).sin() * 12.0 + (t * 0.7 + s).sin() * 1.2;
        let z = (s * 78.233).sin() * 12.0 + (t * 0.5 + s * 2.0).cos() * 1.2;
        tf.translation = cam + Vec3::new(x, 6.0 - fall, z);
        tf.rotation = Quat::from_euler(EulerRot::XYZ, t * 1.3 + s, t * 0.8, s);
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<SeasonLook>()
        .add_systems(Startup, spawn_leaves)
        .add_systems(
            Update,
            (
                update_look,
                apply_season.after(update_look),
                update_leaves.after(update_look),
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daynight::GameDate;

    fn at(m: u8, d: u8) -> Look {
        look(
            GameDate {
                year: 0,
                month: m,
                day: d,
            }
            .day_of_year() as f32,
        )
    }

    #[test]
    fn look_is_continuous_across_the_new_year() {
        let a = look(365.0);
        let b = look(366.0 - 365.0 + 0.0);
        assert!((a.snow - b.snow).abs() < 0.1);
        let c = look(1.0);
        let d = look(0.5 + 365.0);
        assert!((c.snow - d.snow).abs() < 0.05);
    }

    #[test]
    fn snow_only_in_winter() {
        assert!(at(1, 15).snow > 0.8);
        assert!(at(12, 28).snow > 0.8);
        assert!(at(4, 15).snow < 0.01);
        assert!(at(7, 15).snow < 0.01);
        assert!(at(10, 15).snow < 0.01);
    }

    #[test]
    fn flowers_bloom_in_spring_and_summer() {
        assert!(at(5, 20).flowers > 0.9);
        assert!(at(7, 15).flowers > 0.7);
        assert!(at(11, 15).flowers < 0.01);
        assert!(at(1, 15).flowers < 0.01);
    }

    #[test]
    fn autumn_has_colored_leaves_and_falling_leaves() {
        let l = at(10, 27);
        assert!(l.autumn > 0.9 && l.leaf_fall > 0.9);
        assert!(at(6, 15).autumn < 0.01 && at(6, 15).leaf_fall < 0.01);
    }

    #[test]
    fn leaves_turn_from_green_to_orange_to_white() {
        let summer = leaf_color(&at(7, 1), 0);
        let autumn = leaf_color(&at(10, 27), 0);
        let winter = leaf_color(&at(1, 15), 0);
        assert!(summer[1] > summer[0]);
        assert!(autumn[0] > autumn[1]);
        assert!(winter.iter().all(|c| *c > 0.8));
    }

    #[test]
    fn every_day_has_a_valid_look() {
        for doy in 1..=365 {
            let l = look(doy as f32);
            assert!((0.0..=1.0).contains(&l.snow) && (0.0..=1.0).contains(&l.flowers));
            assert!(l.grass.iter().all(|c| (0.4..=1.2).contains(c)));
        }
    }
}
