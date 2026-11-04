//! Camera in terza persona: orbita col mouse (yaw/pitch) + follow smooth del player.

use crate::player::Player;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

/// Parametri di orbita della camera (resource).
#[derive(Resource)]
pub struct OrbitCamera {
    /// Angolo orizzontale (rad), attorno al player.
    pub yaw: f32,
    /// Angolo verticale (rad): 0 = orizzonte, >0 = vista dall'alto. Clampato.
    pub pitch: f32,
    /// Distanza dal player (m).
    pub distance: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            pitch: 0.45,
            distance: 7.0,
        }
    }
}

/// Sensibilità del mouse (rad per pixel).
const SENSITIVITY: f32 = 0.005;
/// Limiti dell'angolo verticale.
const PITCH_MIN: f32 = 0.05;
const PITCH_MAX: f32 = 1.35;
/// Smorzamento del follow (1/s): più alto = camera più reattiva.
const FOLLOW_SMOOTHNESS: f32 = 8.0;
/// Offset verticale del punto osservato (petto del personaggio).
const LOOK_AT_HEIGHT: f32 = 1.0;

/// Aggiorna yaw/pitch della camera con il movimento relativo del mouse.
/// Nota: funziona mentre il cursore è sulla finestra (cursor lock in un backlog futuro).
pub fn orbit(mouse: Res<AccumulatedMouseMotion>, mut state: ResMut<OrbitCamera>) {
    state.yaw -= mouse.delta.x * SENSITIVITY;
    state.pitch = (state.pitch + mouse.delta.y * SENSITIVITY).clamp(PITCH_MIN, PITCH_MAX);
}

/// Posiziona la camera in orbita attorno al player, con smoothing.
pub fn follow(
    state: Res<OrbitCamera>,
    players: Query<&GlobalTransform, With<Player>>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
    time: Res<Time>,
) {
    let (Ok(player), Ok(mut camera)) = (players.single(), cameras.single_mut()) else {
        return;
    };

    // Offset orbitale attorno al punto osservato.
    let focus = player.translation() + Vec3::Y * LOOK_AT_HEIGHT;
    let offset = Vec3::new(
        state.pitch.cos() * state.yaw.sin(),
        state.pitch.sin(),
        state.pitch.cos() * state.yaw.cos(),
    ) * state.distance;

    let target = focus + offset;
    let alpha = (FOLLOW_SMOOTHNESS * time.delta_secs()).clamp(0.0, 1.0);
    camera.translation = camera.translation.lerp(target, alpha);
    camera.look_at(focus, Vec3::Y);
}
