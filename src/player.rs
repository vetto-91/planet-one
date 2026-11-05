//! Player cinematico (Fase 2.3): character controller Avian + ground detection robusta.
//!
//! Perché questa versione?
//! Su pendenze ripide un singolo raycast centrale perde il contatto per qualche
//! frame durante la corsa: il salto veniva ignorato. Ora il ground-check usa
//! 5 raycast (centro + 4 offset) e il contatto fisico di `MoveAndSlide` viene
//! tracciato come "hit recente": il salto resta valido per 0.25 s (coyote)
//! dopo l'ultimo contatto, anche se il ray non colpisce più.
//!
//! Flusso (tutto in FixedUpdate):
//! - `update_grounded`: 5 raycast verso il basso, aggiorna GroundedState.
//! - `movement`: input orizzontale, gravità, salto con coyote/hit-recente.
//! - `run_move_and_slide`: sweep fisico + registrazione hit col terreno.
//! - `facing`: rotazione mesh.

use crate::combat::CombatState;
use crate::scene::{Ground, SceneNode};
use avian3d::prelude::*;
use bevy::prelude::*;

/// Marker del corpo del giocatore.
#[derive(Component)]
pub struct Player;

/// Marker del gruppo visivo (figlio): ruota verso la direzione di moto.
#[derive(Component)]
pub struct PlayerVisual;

/// Vita del player (usata da HUD e combattimento).
#[derive(Component)]
pub struct PlayerHealth {
    pub current: f32,
    pub max: f32,
}

impl Default for PlayerHealth {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
        }
    }
}

/// Buffer del tasto salto: il salto si attiva anche se Space viene premuto
/// pochi frame prima di essere grounded (utile in atterraggio/corsa su pendii).
#[derive(Component, Default)]
pub struct JumpBuffer {
    pub time_left: f32,
}

/// Stato del ground con coyote time e cronologia degli hit fisici.
///
/// - `grounded_now`: almeno un raycast ha colpito il terreno in questo step.
/// - `ground_dist`: distanza del hit più vicino (f32::INFINITY se nessuno).
/// - `coyote_left`: tempo residuo in cui il salto è permesso dopo l'ultimo
///   contatto (coyote time).
/// - `last_ground_hit`: tempo residuo in cui l'ultimo hit di MoveAndSlide
///   col terreno (normale verso l'alto) resta "recente" e abilita il salto.
#[derive(Component)]
pub struct GroundedState {
    pub grounded_now: bool,
    pub ground_dist: f32,
    pub coyote_left: f32,
    pub last_ground_hit: f32,
}

impl Default for GroundedState {
    fn default() -> Self {
        Self {
            grounded_now: false,
            ground_dist: f32::INFINITY,
            coyote_left: 0.0,
            last_ground_hit: 0.0,
        }
    }
}

/// Velocità di camminata (m/s).
const WALK_SPEED: f32 = 6.0;
/// Moltiplicatore di velocità in corsa (Shift).
const SPRINT_FACTOR: f32 = 1.7;
/// Impulso verticale del salto (m/s).
const JUMP_SPEED: f32 = 7.5;
/// Accelerazione di gravità (m/s²), manuale perché il corpo è kinematico.
const GRAVITY: f32 = 20.0;

// --- Ground detection ---
/// Distanza massima del raycast di rilevamento (m): permette di rilevare il
/// terreno anche quando il pg "galleggia" di poco su pendenze ripide.
const GROUND_DETECT_DISTANCE: f32 = 1.50;
/// Distanza entro cui il terreno viene "snapputo" (gravità azzerata): più
/// corta del detect, così in caduta libera a >0.95 m non si ferma in aria.
const GROUND_SNAP_DISTANCE: f32 = 0.95;
/// Normale minima (y) per considerare una superficie "suolo".
const MIN_GROUND_NORMAL_Y: f32 = 0.5;
/// Offset laterali dei 4 raycast aggiuntivi (m), entro il raggio capsula (0.35).
const RAY_OFFSETS: [Vec2; 4] = [
    Vec2::new(0.25, 0.0),
    Vec2::new(-0.25, 0.0),
    Vec2::new(0.0, 0.25),
    Vec2::new(0.0, -0.25),
];
/// Coyote time (s): il salto resta valido questo tempo dopo l'ultimo contatto.
const COYOTE_TIME: f32 = 0.25;
/// Durata (s) per cui un hit fisico di MoveAndSlide resta "recente".
const GROUND_HIT_RECENT: f32 = 0.20;
/// Soglia di velocità verticale sopra cui il salto è bloccato (anti double-jump).
const MAX_RISE_FOR_JUMP: f32 = 1.0;
/// Durata del buffer di salto (s): se premi Space entro questo tempo prima
/// di essere grounded, il salto parte appena tocchi terra.
const JUMP_BUFFER_TIME: f32 = 0.15;

/// Spawn del giocatore: corpo kinematico + mesh visivo figlio.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands
        .spawn((
            SceneNode,
            Player,
            RigidBody::Kinematic,
            Collider::capsule(0.35, 1.0),
            LockedAxes::ROTATION_LOCKED,
            LinearVelocity::ZERO,
            CustomPositionIntegration,
            GroundedState::default(),
            JumpBuffer::default(),
            PlayerHealth::default(),
            CombatState::default(),
            Visibility::default(),
            Transform::from_xyz(0.0, 1.0, 0.0),
        ))
        .with_children(|body| {
            body.spawn((PlayerVisual, Transform::default(), Visibility::default()))
                .with_children(|visual| {
                    visual.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.7, 1.3, 0.7))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.80, 0.35, 0.10),
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 0.0, 0.0),
                    ));
                    visual.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.3, 0.3, 0.3))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.95, 0.75, 0.25),
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 0.3, -0.4),
                    ));
                });
        });
}

/// Ground-check multi-ray: 5 raycast verso il basso (centro + 4 offset XZ).
///
/// `grounded_now` è true se almeno un ray colpisce un `Ground` con normale
/// verticale Breaking >= MIN_GROUND_NORMAL_Y entro GROUND_DETECT_DISTANCE.
/// `ground_dist` è la distanza del hit più vicino. Gira in FixedUpdate.
pub fn update_grounded(
    spatial_query: SpatialQuery,
    grounds: Query<Entity, With<Ground>>,
    mut players: Query<(Entity, &GlobalTransform, &mut GroundedState), With<Player>>,
    time: Res<Time>,
) {
    for (entity, transform, mut state) in &mut players {
        let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
        let center = transform.translation();

        // Raccoglie il hit più vicino tra centro e 4 offset laterali.
        let mut best: Option<RayHitData> = None;
        let mut origins = Vec::with_capacity(5);
        origins.push(center);
        for off in RAY_OFFSETS {
            origins.push(center + Vec3::new(off.x, 0.0, off.y));
        }

        for origin in origins {
            if let Some(hit) = spatial_query.cast_ray_predicate(
                origin,
                Dir3::NEG_Y,
                GROUND_DETECT_DISTANCE,
                true,
                &filter,
                &|hit_entity| grounds.contains(hit_entity),
            ) && hit.normal.y >= MIN_GROUND_NORMAL_Y
                && best.as_ref().is_none_or(|b| hit.distance < b.distance)
            {
                best = Some(hit);
            }
        }

        if let Some(hit) = best {
            state.grounded_now = true;
            state.ground_dist = hit.distance;
            state.coyote_left = COYOTE_TIME;
        } else {
            state.grounded_now = false;
            state.ground_dist = f32::INFINITY;
            state.coyote_left = (state.coyote_left - time.delta_secs()).max(0.0);
        }
    }
}

/// Calcola la velocità desiderata (input + gravità + salto) in FixedUpdate.
///
/// - Orizzontale: dall'input WASD relativo alla camera.
/// - Verticale: gravità; azzerata solo se il terreno è entro GROUND_SNAP_DISTANCE
///   (così in caduta libera non si ferma a mezz'aria).
/// - Salto: valido se grounded_now OPPURE coyote_left > 0 OPPURE un hit fisico
///   recente (last_ground_hit > 0), e non in forte ascesa (anti double-jump).
pub fn movement(
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut players: Query<(&mut LinearVelocity, &mut GroundedState, &mut JumpBuffer), With<Player>>,
    time: Res<Time>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };

    let mut input = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        input.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        input.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        input.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        input.x += 1.0;
    }
    let input = input.clamp_length_max(1.0);

    let sprinting = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let speed = WALK_SPEED * if sprinting { SPRINT_FACTOR } else { 1.0 };

    let cam_forward = camera.forward().normalize_or_zero();
    let forward = Vec3::new(cam_forward.x, 0.0, cam_forward.z).normalize_or_zero();
    let right = Vec3::new(cam_forward.z, 0.0, -cam_forward.x).normalize_or_zero();
    let forward = if forward.length_squared() < 0.5 {
        -Vec3::Z
    } else {
        forward
    };
    let right = if right.length_squared() < 0.5 {
        Vec3::X
    } else {
        right
    };

    let direction = (forward * input.y + right * input.x).normalize_or_zero();

    // Buffer del salto: se Space viene premuto, resta "prenotato" per
    // JUMP_BUFFER_TIME secondi e viene consumato appena si puo' saltare.
    let jump_pressed = keys.just_pressed(KeyCode::Space);

    for (mut velocity, mut state, mut buffer) in &mut players {
        velocity.x = direction.x * speed;
        velocity.z = direction.z * speed;

        // Snap al terreno solo se molto vicino (non in caduta libera).
        let snapped = state.grounded_now && state.ground_dist <= GROUND_SNAP_DISTANCE;
        if snapped && velocity.y <= 0.0 {
            velocity.y = 0.0;
        } else {
            velocity.y -= GRAVITY * time.delta_secs();
        }

        // Aggiorna il buffer di salto col delta del fixed step.
        if jump_pressed {
            buffer.time_left = JUMP_BUFFER_TIME;
        } else {
            buffer.time_left = (buffer.time_left - time.delta_secs()).max(0.0);
        }

        // Salto: buffer attivo + (coyote OR hit recente OR grounded) +
        // non in forte ascesa (anti double-jump).
        let can_jump = buffer.time_left > 0.0
            && (state.grounded_now || state.coyote_left > 0.0 || state.last_ground_hit > 0.0)
            && velocity.y <= MAX_RISE_FOR_JUMP;
        if can_jump {
            velocity.y = JUMP_SPEED;
            buffer.time_left = 0.0;
            state.coyote_left = 0.0;
            state.last_ground_hit = 0.0;
        }
    }
}

/// Esegue `MoveAndSlide` in FixedUpdate e registra gli hit col terreno.
///
/// Se durante lo sweep c'è un contatto con normale verso l'alto (>= 0.5),
/// `last_ground_hit` viene ricaricato a GROUND_HIT_RECENT: questo rende il
/// salto affidabile anche quando i raycast non vedono il terreno (pendenze).
pub fn run_move_and_slide(
    move_and_slide: MoveAndSlide,
    time: Res<Time>,
    mut players: Query<
        (
            Entity,
            &mut Transform,
            &mut LinearVelocity,
            &Collider,
            &mut GroundedState,
        ),
        With<Player>,
    >,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut velocity, collider, mut state) in &mut players {
        let mut hit_ground = false;
        let output = move_and_slide.move_and_slide(
            collider,
            transform.translation,
            transform.rotation,
            velocity.0,
            time.delta(),
            &MoveAndSlideConfig::default(),
            &SpatialQueryFilter::from_excluded_entities([entity]),
            |hit| {
                if hit.intersects() && hit.normal.y >= MIN_GROUND_NORMAL_Y {
                    hit_ground = true;
                }
                MoveAndSlideHitResponse::Accept
            },
        );

        transform.translation = output.position;
        velocity.0 = output.projected_velocity;

        if hit_ground {
            state.last_ground_hit = GROUND_HIT_RECENT;
        } else {
            state.last_ground_hit = (state.last_ground_hit - dt).max(0.0);
        }
    }
}

/// Ruota il mesh visivo verso la direzione di movimento (slerp in FixedUpdate).
pub fn facing(
    mut visuals: Query<&mut Transform, With<PlayerVisual>>,
    players: Query<&LinearVelocity, With<Player>>,
    time: Res<Time>,
) {
    let Ok(velocity) = players.single() else {
        return;
    };

    let horizontal = Vec2::new(velocity.x, velocity.z);
    if horizontal.length() < 0.1 {
        return;
    }

    let target_yaw = (-velocity.x).atan2(-velocity.z);
    let target = Quat::from_rotation_y(target_yaw);

    for mut transform in &mut visuals {
        let alpha = (10.0 * time.delta_secs()).clamp(0.0, 1.0);
        transform.rotation = transform.rotation.slerp(target, alpha);
    }
}
