//! Player cinematico (Fase 2.2): character controller Avian + coyote time.
//!
//! Perché il coyote time?
//! Su pendenze del terreno il raycast a volte perde il contatto per un frame
//! (il pg "galleggia" di pochi cm durante la corsa), e il salto veniva ignorato
//! anche se il giocatore era (=TerrainMesh/collisione). Con il coyote time il
//! salto resta valido per ~0.12 s dopo l'ultimo contatto reale.
//!
//! Flusso (tutto in FixedUpdate per delta stabile):
//! - `update_grounded`: raycast + gestione coyote/last-ground.
//! - `movement`: input Orizzontale, gravity, salto con coyote.
//! - `run_move_and_slide`: sweep fisico e scrittura Transform/LinearVelocity.

use crate::scene::{Ground, SceneNode};
use avian3d::prelude::*;
use bevy::prelude::*;

/// Marker del corpo del giocatore.
#[derive(Component)]
pub struct Player;

/// Marker del gruppo visivo (figlio): ruota verso la direzione di moto.
#[derive(Component)]
pub struct PlayerVisual;

/// Stato del ground-check con coyote time (gestito da [`update_grounded`]).
///
/// - `grounded_now`: il raycast colpisce il terreno in questo step.
/// - `coyote_left`: tempo residuo in cui il salto resta permesso
///   dopo l'ultimo contatto reale (coyote time).
#[derive(Component, Default)]
pub struct GroundedState {
    pub grounded_now: bool,
    pub coyote_left: f32,
}

/// Velocità di camminata (m/s).
const WALK_SPEED: f32 = 6.0;
/// Moltiplicatore di velocità in corsa (Shift).
const SPRINT_FACTOR: f32 = 1.7;
/// Impulso verticale del salto (m/s).
const JUMP_SPEED: f32 = 7.0;
/// Accelerazione di gravità (m/s²), manuale perché il corpo è kinematico.
const GRAVITY: f32 = 20.0;
/// Distanza base del ground-check (semi-altezza capsula 0.85) + pelle.
const GROUND_CHECK_DISTANCE: f32 = 0.85;
/// Margine extra per tollerare il "galleggiamento" su pendenze durante
/// camminata/corsa: il raycast arriva fino a 1.25 m sotto il centro.
const GROUND_TOLERANCE: f32 = 0.40;
/// Coyote time (s): dopo l'ultimo contatto, il salto resta valido per
/// questo tempo. Rende il salto affidabile in corsa sulle colline.
const COYOTE_TIME: f32 = 0.12;
/// Soglia sulla normale del contatto: solo superfici con normal.y > 0.6
/// contano come "suolo" (i muri non debeno dare grounded).
const MIN_GROUND_NORMAL_Y: f32 = 0.6;
/// Soglia di velocità verticale sopra la quale il salto è bloccato
/// (evita il doppio salto in ascesa).
const MAX_RISE_FOR_JUMP: f32 = 0.5;

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

/// Aggiorna lo stato di ground con coyote time. Gira in `FixedUpdate`.
///
/// Se il raycast colpisce un `Ground` con normale sufficientemente verticale:
///
/// - `grounded_now = true`, `coyote_left = COYOTE_TIME`.
///
/// Altrimenti `grounded_now = false` e `coyote_left` scala col tempo fino a 0.
pub fn update_grounded(
    spatial_query: SpatialQuery,
    grounds: Query<Entity, With<Ground>>,
    mut players: Query<(Entity, &GlobalTransform, &mut GroundedState), With<Player>>,
    time: Res<Time>,
) {
    let max_dist = GROUND_CHECK_DISTANCE + GROUND_TOLERANCE;
    for (entity, transform, mut state) in &mut players {
        let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
        let hit = spatial_query.cast_ray_predicate(
            transform.translation(),
            Dir3::NEG_Y,
            max_dist,
            true,
            &filter,
            &|hit_entity| grounds.contains(hit_entity),
        );

        let hit_ground = hit.filter(|h| h.normal.y >= MIN_GROUND_NORMAL_Y).is_some();

        if hit_ground {
            state.grounded_now = true;
            state.coyote_left = COYOTE_TIME;
        } else {
            state.grounded_now = false;
            state.coyote_left = (state.coyote_left - time.delta_secs()).max(0.0);
        }
    }
}

/// Calcola la velocità desiderata (input + gravità + salto con coyote) e la
/// scrive in `LinearVelocity`. Il movimento effettivo è in [`run_move_and_slide`].
/// Gira in `FixedUpdate` insieme a ground-check e fisica.
pub fn movement(
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut players: Query<(&mut LinearVelocity, &mut GroundedState), With<Player>>,
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

    let jump_pressed = keys.just_pressed(KeyCode::Space);

    for (mut velocity, mut state) in &mut players {
        velocity.x = direction.x * speed;
        velocity.z = direction.z * speed;

        // Verticale: gravità; a terra (grounded_now) azzera per restare appoggiati.
        if state.grounded_now && velocity.y <= 0.0 {
            velocity.y = 0.0;
        } else {
            velocity.y -= GRAVITY * time.delta_secs();
        }

        // Salto con coyote time: valido se grounded_now OPPURE entro coyote_left.
        // Anti double-jump: blocca se già in ascesa.
        if jump_pressed
            && (state.grounded_now || state.coyote_left > 0.0)
            && velocity.y <= MAX_RISE_FOR_JUMP
        {
            velocity.y = JUMP_SPEED;
            state.coyote_left = 0.0; // consuma il coyote: niente doppio salto
        }
    }
}

/// Esegue `MoveAndSlide` sul corpo kinematico in `FixedUpdate`.
pub fn run_move_and_slide(
    move_and_slide: MoveAndSlide,
    time: Res<Time>,
    mut players: Query<(Entity, &mut Transform, &mut LinearVelocity, &Collider), With<Player>>,
) {
    for (entity, mut transform, mut velocity, collider) in &mut players {
        let output = move_and_slide.move_and_slide(
            collider,
            transform.translation,
            transform.rotation,
            velocity.0,
            time.delta(),
            &MoveAndSlideConfig::default(),
            &SpatialQueryFilter::from_excluded_entities([entity]),
            |_hit| MoveAndSlideHitResponse::Accept,
        );

        transform.translation = output.position;
        velocity.0 = output.projected_velocity;
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
