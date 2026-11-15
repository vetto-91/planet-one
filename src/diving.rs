//! L'apnea (ADR 0055): il talento attivo del Nuoto, «Apnea dei Pescatori di Perle».
//!
//! In acqua profonda, con il talento sbloccato (Nuoto grado 30), tenendo premuto `Ctrl` ci si immerge
//! fino al fondale. Il fiato dura [`BREATH_BASE`] secondi più un poco per ogni rango del talento del
//! Nuoto; a galla torna in fretta. Finito il fiato si risale per forza, e sott'acqua si perde vita
//! finché non si torna su. Al largo della spiaggia del relitto c'è una cassa sommersa.

use crate::skills::Skill;
use bevy::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Secondi di fiato senza ranghi del talento del Nuoto.
pub const BREATH_BASE: f32 = 20.0;
/// Secondi in più per ogni rango del talento del Nuoto.
pub const BREATH_PER_RANK: f32 = 4.0;
/// Quanto in fretta torna il fiato a galla (secondi di fiato per secondo).
const REFILL: f32 = 6.0;
/// Vita persa ogni secondo sott'acqua senza fiato.
const DROWN_DAMAGE: f32 = 8.0;
/// Il centro del corpo resta così sopra il fondale.
pub const FLOOR_CLEAR: f32 = 0.95;
/// Da dove si cerca il fondale per la cassa sommersa: la spiaggia del relitto, verso il largo (sud).
const SUNKEN_FROM: Vec2 = Vec2::new(36.0, 92.0);
/// Fondale abbastanza profondo per la cassa.
const SUNKEN_FLOOR: f32 = -3.2;
/// Chiave del contenitore della cassa sommersa.
pub const SUNKEN_KEY: &str = "sommerso:relitto";

static BREATH: AtomicU32 = AtomicU32::new(0);
static UNDER: AtomicBool = AtomicBool::new(false);
static GASPING: AtomicBool = AtomicBool::new(false);

/// Fiato massimo con i ranghi del talento del Nuoto.
pub fn breath_max() -> f32 {
    BREATH_BASE + BREATH_PER_RANK * crate::skills::rank(Skill::Swimming)
}

/// Secondi di fiato rimasti (pieno all'avvio).
pub fn breath() -> f32 {
    let bits = BREATH.load(Ordering::Relaxed);
    if bits == 0 {
        breath_max()
    } else {
        f32::from_bits(bits)
    }
}

fn set_breath(v: f32) {
    // Lo zero vale «pieno»: il fiato esaurito si tiene appena sopra.
    BREATH.store(v.max(1e-4).to_bits(), Ordering::Relaxed);
}

/// Il talento attivo del Nuoto è sbloccato?
pub fn talent() -> bool {
    crate::skills::active_talent(Skill::Swimming)
}

/// Ci si può immergere adesso? (talento, e fiato non esaurito da poco)
pub fn can_dive() -> bool {
    talent() && !GASPING.load(Ordering::Relaxed)
}

/// Il movimento dice se la testa è sott'acqua.
pub fn set_under(under: bool) {
    UNDER.store(under, Ordering::Relaxed);
}

pub fn under() -> bool {
    UNDER.load(Ordering::Relaxed)
}

/// Un passo del fiato: sott'acqua cala, a galla torna. Rende il fiato nuovo e se si è appena esaurito.
pub fn step_breath(left: f32, max: f32, under: bool, dt: f32) -> (f32, bool) {
    if under {
        let next = (left - dt).max(0.0);
        (next, left > 0.0 && next <= 0.0)
    } else {
        ((left + REFILL * dt).min(max), false)
    }
}

/// Sistema: consuma e ridà il fiato; senza fiato si risale per forza e si perde vita.
pub fn breathe(
    time: Res<Time>,
    mut players: Query<&mut crate::player::PlayerHealth, With<crate::player::Player>>,
    mut session: ResMut<crate::save::Session>,
) {
    let dt = time.delta_secs();
    let max = breath_max();
    let under = under();
    let (left, ran_out) = step_breath(breath().min(max), max, under, dt);
    set_breath(left);
    if under && left > 0.0 {
        crate::skills::train(Skill::Swimming, dt * 0.4);
    }
    if ran_out {
        GASPING.store(true, Ordering::Relaxed);
        session.notify("Senza fiato: risali!");
        info!("[APNEA] fiato finito");
    }
    if GASPING.load(Ordering::Relaxed) && !under && left >= max * 0.5 {
        GASPING.store(false, Ordering::Relaxed);
    }
    if under
        && left <= 0.0
        && let Ok(mut hp) = players.single_mut()
    {
        hp.current = (hp.current - DROWN_DAMAGE * dt).max(1.0);
    }
}

/// Dove sta la cassa sommersa: dalla spiaggia del relitto verso il largo, il primo fondale profondo.
pub fn sunken_spot(height: impl Fn(f32, f32) -> f32) -> Option<Vec2> {
    (0..120)
        .map(|k| SUNKEN_FROM + Vec2::Y * (k as f32 * 0.5))
        .find(|p| height(p.x, p.y) <= SUNKEN_FLOOR)
        .map(|p| p + Vec2::Y * 1.5)
}

#[derive(Component)]
pub struct SunkenChest;

/// Sistema: sull'isola, la cassa sommersa al largo della spiaggia del relitto (si apre con l'apnea).
pub fn sunken_chest(
    mut commands: Commands,
    chests: Query<(), With<SunkenChest>>,
    field: Option<Res<crate::terrain::TerrainField>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if crate::region::current() != crate::region::Region::Isola || !chests.is_empty() {
        return;
    }
    let Some(field) = field else {
        return;
    };
    let Some(at) = sunken_spot(|x, z| field.height_at(x, z)) else {
        return;
    };
    let ground = field.height_at(at.x, at.y);
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.32, 0.26),
        perceptual_roughness: 0.95,
        ..default()
    });
    let brass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.5, 0.35),
        metallic: 0.6,
        perceptual_roughness: 0.5,
        ..default()
    });
    let root = commands
        .spawn((
            SunkenChest,
            crate::scene::SceneNode,
            Transform::from_xyz(at.x, ground, at.y).with_rotation(Quat::from_rotation_y(0.4)),
            Visibility::default(),
        ))
        .with_children(|c| {
            c.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.0, 0.55, 0.65))),
                MeshMaterial3d(wood),
                Transform::from_xyz(0.0, 0.22, 0.0).with_rotation(Quat::from_rotation_z(0.1)),
            ));
            for x in [-0.3f32, 0.3] {
                c.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.06, 0.58, 0.68))),
                    MeshMaterial3d(brass.clone()),
                    Transform::from_xyz(x, 0.22, 0.0).with_rotation(Quat::from_rotation_z(0.1)),
                ));
            }
        })
        .id();
    let e = commands
        .spawn(crate::containers::bundle(
            crate::containers::ContainerKind::Chest,
            Vec3::new(0.0, 0.4, 0.0),
            SUNKEN_KEY.to_string(),
            "sommerso",
        ))
        .id();
    commands.entity(root).add_child(e);
    info!(
        "[APNEA] cassa sommersa a {:.1} {:.1} (fondale {ground:.1})",
        at.x, at.y
    );
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (breathe, sunken_chest).run_if(in_state(crate::AppState::Playing)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breath_runs_out_under_water_and_comes_back_on_top() {
        let (left, out) = step_breath(1.0, 20.0, true, 0.6);
        assert!((left - 0.4).abs() < 1e-5 && !out);
        let (left, out) = step_breath(left, 20.0, true, 0.6);
        assert_eq!(left, 0.0);
        assert!(out, "il fiato si esaurisce una volta");
        assert!(!step_breath(0.0, 20.0, true, 0.6).1);
        let (left, _) = step_breath(0.0, 20.0, false, 1.0);
        assert!((left - REFILL).abs() < 1e-5);
        assert_eq!(step_breath(19.0, 20.0, false, 1.0).0, 20.0);
    }

    #[test]
    fn the_sunken_chest_lies_offshore_on_deep_water() {
        // Una spiaggia che scende di un metro ogni dieci verso sud.
        let slope = |_x: f32, z: f32| -(z - 92.0) / 10.0;
        let at = sunken_spot(slope).unwrap();
        assert!(slope(at.x, at.y) <= SUNKEN_FLOOR);
        assert!(sunken_spot(|_, _| 0.0).is_none());
    }
}
