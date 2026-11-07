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
use bevy::prelude::*;
use bevy::window::CursorMoved;

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

/// Marker dei collider che la camera in terza persona non deve attraversare
/// (pareti e tetto della casetta).
#[derive(Component)]
pub struct BlocksCamera;

/// Limiti dell'angolo verticale.
const PITCH_MIN: f32 = 0.05;
const PITCH_MAX: f32 = 1.35;
/// Smorzamento del follow in terza persona (1/s).
const FOLLOW_SMOOTHNESS: f32 = 8.0;
/// Offset verticale del punto osservato in terza persona (petto del pg).
const LOOK_AT_HEIGHT: f32 = 1.0;
/// Offset degli occhi rispetto al centro della capsula del player (prima persona).
const FIRST_PERSON_EYE: f32 = 0.72;

/// Nasconde/mostra il mesh visivo del player: in prima persona non si deve vedere.
#[allow(clippy::type_complexity)]
pub fn player_visibility(
    state: Res<OrbitCamera>,
    // Nascondi sia il corpo fisico (Player) sia il gruppo mesh (PlayerVisual):
    // il corpo ha Visibility perche' e' parent dei mesh figli.
    mut bodies: Query<&mut Visibility, Or<(With<Player>, With<crate::player::PlayerVisual>)>>,
) {
    let target = if state.mode == CameraMode::FirstPerson {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    for mut v in &mut bodies {
        *v = target;
    }
}

/// Aggiorna yaw/pitch con il movimento relativo del mouse (sempre attivo in
/// entrambe le modalità, così la mira resta sincronizzata).
pub fn orbit(
    mut cursor_events: MessageReader<CursorMoved>,
    mut state: ResMut<OrbitCamera>,
    debug: Res<crate::debug_place::DebugPlace>,
) {
    // Usiamo CursorMoved (delta calcolato da Bevy dal WindowEvent): su X11 i
    // DeviceEvent MouseMotion possono non arrivare, CursorMoved arriva sempre.
    let mut delta = Vec2::ZERO;
    for ev in cursor_events.read() {
        delta += ev.delta.unwrap_or_default();
    }
    // Con il debug F4 il mouse sceglie i punti: si guarda solo tenendo premuto il tasto destro.
    if debug.on && !state.rmb_held {
        return;
    }
    state.yaw -= delta.x * state.sensitivity;
    // In prima persona permettiamo di guardare più in alto/giù (meno clamp alto).
    // Verticale: in 1a persona quasi 180° (±88°) — al massimo in alto (cielo)
    // o in basso (terreno) la visuale si ferma, così il pg non si ribalta.
    // In 3a persona range conservativo per non far passare la camera sotto il suolo.
    let (min, max) = if state.mode == CameraMode::FirstPerson {
        (-1.54, 1.54)
    } else {
        (PITCH_MIN, PITCH_MAX)
    };
    // Stesso segno di dy' in entrambe le viste: pitch alto = guardi in basso
    // (sia in 3a che in 1a), cosi' il passaggio 3a->1a non inverte la verticale.
    state.pitch = (state.pitch + delta.y * state.sensitivity).clamp(min, max);
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
            // Entrando in 1a persona, azzera il pitch: la mira parte orizzontale.
            // Cosi' non erediti l'altezza 3a che farebbe guardare cielo/terra a caso.
            state.pitch = 0.0;
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
    let new_mode = if state.rmb_held {
        CameraMode::FirstPerson
    } else {
        CameraMode::ThirdPerson
    };
    // Alla transizione 3a->1a azzera pitch: mira orizzontale, niente salto verticale.
    if new_mode == CameraMode::FirstPerson && state.mode == CameraMode::ThirdPerson {
        state.pitch = 0.0;
    }
    state.mode = new_mode;
}

/// Posiziona la camera: in terza persona orbita attorno al player con
/// smoothing; in prima persona va agli occhi del player e guarda nella
/// sua stessa direzione (crosshair sincronizzata).
pub fn follow(
    state: Res<OrbitCamera>,
    spatial: avian3d::prelude::SpatialQuery,
    blockers: Query<(), With<BlocksCamera>>,
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

            let mut target = focus + offset;
            // Non attraversare pareti e tetto: raycast dal fuoco verso la camera.
            let mut clipped = false;
            if let Ok(dir) = Dir3::new(offset)
                && let Some(hit) = spatial.cast_ray_predicate(
                    focus,
                    dir,
                    state.distance,
                    true,
                    &avian3d::prelude::SpatialQueryFilter::default(),
                    &|e| blockers.contains(e),
                )
            {
                target = focus + *dir * (hit.distance - 0.25).max(0.4);
                clipped = true;
            }
            let alpha = (FOLLOW_SMOOTHNESS * time.delta_secs()).clamp(0.0, 1.0);
            camera.translation = if clipped {
                target // contro un muro: subito vicino, senza passare attraverso
            } else {
                camera.translation.lerp(target, alpha)
            };
            camera.look_at(focus, Vec3::Y);
        }
        CameraMode::FirstPerson => {
            // VERA prima persona: la camera va DENTRO il player e ruota come
            // una FPS standard. yaw su Y (destra/sinistra), pitch su X (alto/basso).
            // In orbit() in 1a persona dy e' invertito, quindi mouse SU aumenta
            // pitch; rotation_x(+pitch) in Bevy (Y-up) porta il forward verso +Y = SU.
            // pitch alto = guardi in basso (come in 3a), quindi -pitch su X.
            // Mouse giu' -> pitch aumenta -> -pitch diminuisce -> guardi giu'.
            // La mira resta coerente con la crosshair della 3a persona.
            let rot = Quat::from_rotation_y(state.yaw) * Quat::from_rotation_x(-state.pitch);
            // Occhi a ~1,57 m da terra (il centro della capsula è a 0,85 m).
            camera.translation = player.translation() + Vec3::Y * FIRST_PERSON_EYE;
            camera.rotation = rot;
        }
    }
}
