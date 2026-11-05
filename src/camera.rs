//! Camera: orbita in terza persona con mouse (yaw/pitch) + vista in prima
//! persona su RMB tenuto o toggle con V (lock).
//!
//! - Terza persona: offset orbitale attorno al player, come prima.
//! - Prima persona: la camera va alla posizione degli occhi del player
//!   (altezza EYE_HEIGHT) e ruota con yaw/pitch, guardando esattamente
//!   nella stessa direzione del personaggio.
//!
//! Regole:
//! - RMB tenuto → prima persona temporanea (rilascio = torna in terza).
//! - V → toggle lock prima persona (RMB non interferisce se lock attivo).
//! - Se il lock è attivo, rilasciare RMB NON fa tornare in terza persona.

use crate::player::Player;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

/// Altezza degli occhi del player in prima persona (m).

/// Modalità della camera.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CameraMode {
    /// Terza persona (default): offset orbitale.
    #[default]
    ThirdPerson,
    /// Prima persona: dagli occhi del player.
    FirstPerson,
}

/// Stato della camera: parametri orbita + modalità corrente + lock V.
#[derive(Resource)]
pub struct OrbitCamera {
    /// Angolo orizzontale (rad).
    pub yaw: f32,
    /// Angolo verticale (rad): 0 = orizzonte, >0 = vista dall'alto. Clampato.
    pub pitch: f32,
    /// Distanza dal player in terza persona (m).
    pub distance: f32,
    /// Sensibilità del mouse (rad per pixel), regolabile dal menu Esc.
    pub sensitivity: f32,
    /// Modalità attiva (terza/prima persona).
    pub mode: CameraMode,
    /// Se true, V ha bloccato la prima persona (rimane finché non si ripreme V).
    pub first_person_lock: bool,
    /// Se true, il tasto destro è tenuto (richiesta temporanea di prima persona).
    pub rmb_held: bool,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            pitch: 0.45,
            distance: 7.0,
            sensitivity: 0.003,
            mode: CameraMode::ThirdPerson,
            first_person_lock: false,
            rmb_held: false,
        }
    }
}

/// Limiti dell'angolo verticale.
const PITCH_MIN: f32 = 0.05;
const PITCH_MAX: f32 = 1.35;
/// Smorzamento del follow in terza persona (1/s).
const FOLLOW_SMOOTHNESS: f32 = 8.0;
/// Offset verticale del punto osservato in terza persona (petto del pg).
const LOOK_AT_HEIGHT: f32 = 1.0;

/// Aggiorna yaw/pitch con il movimento relativo del mouse (sempre attivo in
/// entrambe le modalità, così la mira resta sincronizzata).
pub fn orbit(mouse: Res<AccumulatedMouseMotion>, mut state: ResMut<OrbitCamera>) {
    state.yaw -= mouse.delta.x * state.sensitivity;
    // In prima persona permettiamo di guardare più in alto/giù (meno clamp alto).
    let (min, max) = if state.mode == CameraMode::FirstPerson {
        (-1.45, 1.45)
    } else {
        (PITCH_MIN, PITCH_MAX)
    };
    state.pitch = (state.pitch + mouse.delta.y * state.sensitivity).clamp(min, max);
}

/// Legge gli eventi raw del mouse e aggiorna `rmb_held` nella resource.
/// Gira in PreUpdate (prima di Update) così mode_input vede lo stato fresco.
pub fn rmb_raw(
    mut events: MessageReader<bevy::input::mouse::MouseButtonInput>,
    mut state: ResMut<OrbitCamera>,
) {
    for ev in events.read() {
        if ev.button == MouseButton::Right {
            state.rmb_held = ev.state.is_pressed();
        }
    }
}

/// Gestisce il cambio modalità: V toggle lock, RMB tenuto = temporaneo.
/// Se il lock è attivo, RMB non cambia nulla.
pub fn mode_input(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<OrbitCamera>) {
    let rmb = state.rmb_held;
    // V: toggle del lock prima persona.
    if keys.just_pressed(KeyCode::KeyV) {
        state.first_person_lock = !state.first_person_lock;
        state.mode = if state.first_person_lock {
            CameraMode::FirstPerson
        } else {
            CameraMode::ThirdPerson
        };
        info!(
            "camera mode: {:?} (lock={})",
            state.mode, state.first_person_lock
        );
        return;
    }

    // Se il lock è attivo, RMB non fa nulla.
    if state.first_person_lock {
        state.rmb_held = rmb;
        return;
    }

    // RMB tenuto → prima persona temporanea; al rilascio → terza persona.
    state.rmb_held = rmb;
    state.mode = if state.rmb_held {
        CameraMode::FirstPerson
    } else {
        CameraMode::ThirdPerson
    };
}

/// Posiziona la camera: in terza persona orbita attorno al player con
/// smoothing; in prima persona va agli occhi del player e guarda nella
/// sua stessa direzione (crosshair sincronizzata).
pub fn follow(
    state: Res<OrbitCamera>,
    players: Query<&GlobalTransform, With<Player>>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
    time: Res<Time>,
) {
    let (Ok(player), Ok(mut camera)) = (players.single(), cameras.single_mut()) else {
        return;
    };

    match state.mode {
        CameraMode::ThirdPerson => {
            // Offset orbitale attorno al punto osservato (come prima).
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
        CameraMode::FirstPerson => {
            // VERA prima persona: la camera va esattamente DENTRO il player
            // (stessa posizione, stessa rotazione yaw/pitch): il pg non si vede.
            let rot = Quat::from_euler(EulerRot::ZYX, state.yaw, -state.pitch, 0.0);
            camera.translation = player.translation();
            camera.rotation = rot;
        }
    }
}
