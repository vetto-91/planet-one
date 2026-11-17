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

/// Camera libera della modalità piazzamenti (F4 + F6, ADR 0061): vola staccata dal giocatore
/// per avvicinarsi agli oggetti. I tasti del volo non arrivano al gioco (il giocatore resta fermo).
#[derive(Resource, Default)]
pub struct FlyCamera {
    pub active: bool,
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    /// Tasti del volo tenuti, dagli eventi della tastiera (in `ButtonInput` vengono azzerati).
    held: Vec<KeyCode>,
    /// Tasti premuti in `ButtonInput` in questo fotogramma (anche dal pilota automatico).
    pressed: Vec<KeyCode>,
}

/// Tasti del volo: avanti, indietro, sinistra, destra, su, giù, veloce.
const FLY_KEYS: [KeyCode; 11] = [
    KeyCode::KeyW,
    KeyCode::KeyS,
    KeyCode::KeyA,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::Space,
    KeyCode::KeyQ,
    KeyCode::KeyC,
    KeyCode::ShiftLeft,
    KeyCode::ShiftRight,
    KeyCode::ArrowUp,
];
/// Velocità della camera libera (m/s); con Maiusc quattro volte tanto.
const FLY_SPEED: f32 = 8.0;

impl FlyCamera {
    fn down(&self, k: KeyCode) -> bool {
        self.held.contains(&k) || self.pressed.contains(&k)
    }

    /// Guarda verso `target` (stessa convenzione della prima persona).
    pub fn look_at(&mut self, target: Vec3) {
        let d = (target - self.pos).normalize_or_zero();
        if d != Vec3::ZERO {
            self.yaw = (-d.x).atan2(-d.z);
            self.pitch = (-d.y).asin();
        }
    }

    fn rotation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(-self.pitch)
    }
}

/// Sistema (PreUpdate, dopo l'input): con la camera libera accesa legge i tasti del volo e li
/// toglie al gioco, così il giocatore non cammina, non salta e non lancia incantesimi.
pub fn fly_keys(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut fly: ResMut<FlyCamera>,
    debug: Res<crate::debug_place::DebugPlace>,
) {
    let on = debug.on && debug.fly;
    for ev in events.read() {
        if !FLY_KEYS.contains(&ev.key_code) {
            continue;
        }
        fly.held.retain(|k| *k != ev.key_code);
        if ev.state.is_pressed() {
            fly.held.push(ev.key_code);
        }
    }
    if !on {
        fly.held.clear();
        fly.pressed.clear();
        return;
    }
    fly.pressed = FLY_KEYS.into_iter().filter(|k| keys.pressed(*k)).collect();
    for k in FLY_KEYS {
        keys.reset(k);
    }
    keys.reset(KeyCode::ArrowDown);
    keys.reset(KeyCode::ArrowLeft);
    keys.reset(KeyCode::ArrowRight);
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
/// Offset degli occhi rispetto al centro della capsula del player (prima persona): gli occhi del
/// modello stanno a 1,70 m da terra, il centro della capsula a 0,85 m.
const FIRST_PERSON_EYE: f32 = 0.85;
/// Parti del modello del giocatore che in prima persona si nascondono: la testa starebbe davanti
/// alla camera (e le animazioni la spostano). Il resto del corpo resta visibile guardando in basso.
const HEAD_PARTS: [&str; 4] = ["part_SuperHero", "part_Hair", "part_Eyes", "part_Eyebrows"];

/// Una parte della testa del giocatore (vedi [`HEAD_PARTS`]).
#[derive(Component)]
pub struct PlayerHead;

/// Sistema: riconosce le parti della testa nel modello del giocatore appena caricato.
pub fn tag_player_head(
    mut commands: Commands,
    visuals: Query<Entity, With<crate::player::PlayerVisual>>,
    heads: Query<(), With<PlayerHead>>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    if !heads.is_empty() {
        return;
    }
    for visual in &visuals {
        for e in children.iter_descendants(visual) {
            if names
                .get(e)
                .is_ok_and(|n| HEAD_PARTS.iter().any(|p| n.as_str().starts_with(p)))
            {
                commands.entity(e).insert(PlayerHead);
            }
        }
    }
}

/// Nasconde/mostra il mesh visivo del player: in prima persona non si deve vedere.
#[allow(clippy::type_complexity)]
pub fn player_visibility(
    // Nascondi sia il corpo fisico (Player) sia il gruppo mesh (PlayerVisual):
    // il corpo ha Visibility perche' e' parent dei mesh figli.
    mut bodies: Query<
        &mut Visibility,
        (
            Or<(With<Player>, With<crate::player::PlayerVisual>)>,
            Without<PlayerHead>,
        ),
    >,
    mut heads: Query<&mut Visibility, With<PlayerHead>>,
    state: Res<OrbitCamera>,
    cheats: Res<crate::shell::Cheats>,
) {
    // In prima persona il corpo resta: guardando in basso ci si vede, con ciò che si impugna.
    let target = if cheats.invisible {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    for mut v in &mut bodies {
        v.set_if_neq(target);
    }
    // In prima persona la testa sparisce: la camera sta dove sono gli occhi.
    let head = if state.mode == CameraMode::FirstPerson {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut v in &mut heads {
        v.set_if_neq(head);
    }
}

/// Aggiorna yaw/pitch con il movimento relativo del mouse (sempre attivo in
/// entrambe le modalità, così la mira resta sincronizzata).
pub fn orbit(
    mut cursor_events: MessageReader<CursorMoved>,
    mut motion_events: MessageReader<bevy::input::mouse::MouseMotion>,
    cursors: Query<&bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
    mut state: ResMut<OrbitCamera>,
    debug: Res<crate::debug_place::DebugPlace>,
    keys: Res<ButtonInput<KeyCode>>,
    mut fly: ResMut<FlyCamera>,
) {
    let cursor: Vec2 = cursor_events
        .read()
        .map(|ev| ev.delta.unwrap_or_default())
        .sum();
    let motion: Vec2 = motion_events.read().map(|ev| ev.delta).sum();
    let captured = cursors
        .single()
        .is_ok_and(|c| c.grab_mode != bevy::window::CursorGrabMode::None);
    let delta = pick_delta(captured, cursor, motion);
    // Con F4 la visuale resta normale; solo con Ctrl (cursore libero sul pannello) non ruota.
    if crate::debug_place::cursor_free(debug.on, &keys) {
        return;
    }
    // Camera libera: il mouse gira lei, la visuale del giocatore resta com'era.
    if fly.active {
        fly.yaw -= delta.x * state.sensitivity;
        fly.pitch = (fly.pitch + delta.y * state.sensitivity).clamp(-1.54, 1.54);
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

/// Quale movimento del mouse usare.
///
/// - Cursore catturato (schermo intero): il movimento grezzo del dispositivo (`MouseMotion`). Con il
///   cursore bloccato, su Wayland (anche via XWayland) e su macOS la posizione del cursore non cambia
///   più e `CursorMoved` non porta nulla: la visuale restava ferma (difetto della v0.0.61 su un PC
///   con Wayland).
/// - Cursore libero (finestra), o se il dispositivo tace: lo spostamento del cursore (`CursorMoved`),
///   che su X11 arriva sempre e non fa girare la visuale quando il mouse è su un'altra finestra.
///
/// Mai tutti e due insieme, o la rotazione raddoppia.
pub fn pick_delta(captured: bool, cursor: Vec2, motion: Vec2) -> Vec2 {
    if captured && motion != Vec2::ZERO {
        motion
    } else {
        cursor
    }
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
    (mut fly, mut debug, field): (
        ResMut<FlyCamera>,
        ResMut<crate::debug_place::DebugPlace>,
        Option<Res<crate::terrain::TerrainField>>,
    ),
) {
    let (Ok(player), Ok(mut camera)) = (players.single(), cameras.single_mut()) else {
        return;
    };
    let flying = debug.on && debug.fly;
    if flying && !fly.active {
        // Si parte da dove si trova la camera, guardando dove guardava.
        fly.active = true;
        fly.pos = camera.translation;
        let ahead = camera.translation + *camera.forward();
        fly.look_at(ahead);
    } else if !flying && fly.active {
        fly.active = false;
    }
    if fly.active {
        // «Vai e seleziona»: davanti all'oggetto, un po' in alto, guardandolo.
        if let Some(p) = debug.goto.take() {
            let ground = field.as_ref().map_or(0.0, |f| f.height_at(p.x, p.y));
            let target = Vec3::new(p.x, ground + 0.8, p.y);
            let back = Quat::from_rotation_y(fly.yaw) * Vec3::Z;
            fly.pos =
                target + Vec3::new(back.x, 0.0, back.z).normalize_or(Vec3::Z) * 6.0 + Vec3::Y * 3.0;
            fly.look_at(target);
        }
        let rot = fly.rotation();
        let axis = |plus: &[KeyCode], minus: &[KeyCode]| {
            let p = plus.iter().any(|k| fly.down(*k)) as i32 as f32;
            let m = minus.iter().any(|k| fly.down(*k)) as i32 as f32;
            p - m
        };
        let ahead = axis(&[KeyCode::KeyW, KeyCode::ArrowUp], &[KeyCode::KeyS]);
        let side = axis(&[KeyCode::KeyD], &[KeyCode::KeyA]);
        let up = axis(
            &[KeyCode::KeyE, KeyCode::Space],
            &[KeyCode::KeyQ, KeyCode::KeyC],
        );
        let fast = fly.down(KeyCode::ShiftLeft) || fly.down(KeyCode::ShiftRight);
        let dir =
            (rot * Vec3::NEG_Z * ahead + rot * Vec3::X * side + Vec3::Y * up).clamp_length_max(1.0);
        let speed = FLY_SPEED * if fast { 4.0 } else { 1.0 };
        fly.pos += dir * speed * time.delta_secs();
        camera.translation = fly.pos;
        camera.rotation = rot;
        return;
    }

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
            // Occhi a 1,70 m da terra (il centro della capsula è a 0,85 m), appena davanti al
            // viso: la testa è nascosta (`player_visibility`), così non entra mai nell'inquadratura.
            let ahead = Quat::from_rotation_y(state.yaw) * Vec3::NEG_Z * 0.12;
            camera.translation = player.translation() + Vec3::Y * FIRST_PERSON_EYE + ahead;
            camera.rotation = rot;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_captured_cursor_turns_with_the_raw_mouse_motion() {
        let cursor = Vec2::new(15.0, 0.0);
        let motion = Vec2::new(14.0, 2.0);
        // Schermo intero: conta il movimento grezzo, anche quando il cursore bloccato non si muove.
        assert_eq!(pick_delta(true, Vec2::ZERO, motion), motion);
        assert_eq!(
            pick_delta(true, cursor, motion),
            motion,
            "mai i due sommati"
        );
        // Se il dispositivo tace (X11 senza eventi grezzi) resta il cursore.
        assert_eq!(pick_delta(true, cursor, Vec2::ZERO), cursor);
        // A finestra solo il cursore: il mouse su un'altra finestra non gira la visuale.
        assert_eq!(pick_delta(false, Vec2::ZERO, motion), Vec2::ZERO);
        assert_eq!(pick_delta(false, cursor, motion), cursor);
    }
}
