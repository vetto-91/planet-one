//! Player cinematico (Fase 2.1): character controller MoveAndSlide di Avian.
//!
//! Perché il passaggio da `RigidBody::Dynamic` a kinematico?
//! La capsula dinamica su heightfield a celle grandi (100 m / 256 = ~0.39 m)
//! tende a tunnelare o a rimbalzare sulle pendenze: il solver fisico non
//! garantisce il contatto quando la velocità verticale supera la dimensione
//! della cella. `MoveAndSlide` risolve il movimento con sweep continui
//! (shape cast), quindi scivola sulle colline senza sprofondare.
//!
//! Flusso:
//! - `movement` calcola la velocità desiderata (orizzontale + gravità + salto)
//!   e la scrive in `LinearVelocity`.
//! - `run_move_and_slide` (FixedUpdate) chiama `MoveAndSlide::move_and_slide`
//!   e scrive il risultato direttamente in `Transform` e `LinearVelocity`.
//! - `CustomPositionIntegration` impedisce ad Avian di muovere il corpo da solo.

use crate::scene::{Ground, SceneNode};
use avian3d::prelude::*;
use bevy::prelude::*;

/// Marker del corpo del giocatore.
#[derive(Component)]
pub struct Player;

/// Marker del gruppo visivo (figlio del corpo): ruota verso la direzione di moto.
#[derive(Component)]
pub struct PlayerVisual;

/// Marker: il player è a terra in questo frame (gestito da [`update_grounded`]).
#[derive(Component)]
pub struct Grounded;

/// Velocità di camminata (m/s).
const WALK_SPEED: f32 = 6.0;
/// Moltiplicatore di velocità in corsa (Shift).
const SPRINT_FACTOR: f32 = 1.7;
/// Impulso verticale del salto (m/s).
const JUMP_SPEED: f32 = 6.0;
/// Accelerazione di gravità (m/s²), applicata manualmente perché il corpo
/// è kinematico e Avian non applica la gravità da solo.
const GRAVITY: f32 = 20.0;
/// Soglia di distanza per il ground-check (m): semi-altezza capsula (0.85) + pelle.
const GROUND_CHECK_DISTANCE: f32 = 0.85;
const GROUND_SKIN: f32 = 0.10;
/// Soglia di velocità verticale sopra la quale il salto è bloccato
/// (evita il doppio salto nei primi frame di ascesa).
const MAX_RISE_FOR_JUMP: f32 = 0.5;

/// Spawn del giocatore: corpo kinematico + mesh visivo figlio.
///
/// - `RigidBody::Kinematic`: Avian non lo muove da solo, ma i suoi collider
///   interagiscono con il mondo (il terreno lo blocca).
/// - `CustomPositionIntegration`: disattiva l'integratore di posizione di Avian,
///   così solo `run_move_and_slide` sposta il corpo.
/// - `LockedAxes::ROTATION_LOCKED`: la capsula non ruota mai (evita capovolte).
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
            // Visibility: serve per la propagazione dei figli mesh (evita B0004).
            Visibility::default(),
            Transform::from_xyz(0.0, 1.0, 0.0),
        ))
        .with_children(|body| {
            body.spawn((PlayerVisual, Transform::default(), Visibility::default()))
                .with_children(|visual| {
                    // Corpo (placeholder per il personaggio).
                    visual.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.7, 1.3, 0.7))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.80, 0.35, 0.10),
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 0.0, 0.0),
                    ));
                    // "Naso" per rendere visibile l'orientamento.
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

/// Calcola la velocità desiderata del player (input + gravità + salto) e la
/// scrive in `LinearVelocity`. Il movimento effettivo avviene in
/// [`run_move_and_slide`].
///
/// La componente orizzontale è sovrascritta dall'input (WASD relativo alla
/// camera); la componente verticale accumula la gravità e viene azzerata
/// quando si è a terra, così il personaggio resta appoggiato al terreno.
pub fn movement(
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut players: Query<(&mut LinearVelocity, Option<&Grounded>), With<Player>>,
    time: Res<Time>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };

    // Direzione di input sul piano XZ.
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

    // Corsa con Shift.
    let sprinting = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let speed = WALK_SPEED * if sprinting { SPRINT_FACTOR } else { 1.0 };

    // Forward/right della camera proiettati sul piano XZ.
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

    for (mut velocity, grounded) in &mut players {
        // Orizzontale:速度 direttamente dall'input.
        velocity.x = direction.x * speed;
        velocity.z = direction.z * speed;

        // Verticale: gravità, azzerata a terra (grounded) per restare appoggiati.
        let grounded = grounded.is_some();
        if grounded && velocity.y <= 0.0 {
            velocity.y = 0.0;
        } else {
            velocity.y -= GRAVITY * time.delta_secs();
        }

        // Salto: impulso solo se a terra e non in ascesa (anti double-jump).
        if keys.just_pressed(KeyCode::Space) && grounded && velocity.y <= MAX_RISE_FOR_JUMP {
            velocity.y = JUMP_SPEED;
        }
    }
}

/// Esegue `MoveAndSlide` sul corpo kinematico: aggiorna `Transform` e
/// `LinearVelocity` con il risultato dello sweep. Gira in `FixedUpdate`
/// per avere un delta tempo stabile.
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

/// Ground-check: raycast verso il basso dal centro della capsula.
///
/// Il player è a terra se il ray colpisce un collider `Ground` entro la
/// semi-altezza della capsula più una pelle di tolleranza. Escludere il
/// player stesso evita che il ray colpisca la propria capsula.
pub fn update_grounded(
    spatial_query: SpatialQuery,
    mut commands: Commands,
    grounds: Query<Entity, With<Ground>>,
    players: Query<(Entity, &GlobalTransform), With<Player>>,
) {
    for (entity, transform) in &players {
        let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
        let hit = spatial_query.cast_ray_predicate(
            transform.translation(),
            Dir3::NEG_Y,
            GROUND_CHECK_DISTANCE + GROUND_SKIN,
            true,
            &filter,
            &|hit_entity| grounds.contains(hit_entity),
        );

        if hit.is_some() {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

/// Ruota il mesh visivo verso la direzione di movimento (smooth).
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
        return; // Fermi: mantieni l'orientamento attuale.
    }

    let target_yaw = (-velocity.x).atan2(-velocity.z);
    let target = Quat::from_rotation_y(target_yaw);

    for mut transform in &mut visuals {
        let alpha = (10.0 * time.delta_secs()).clamp(0.0, 1.0);
        transform.rotation = transform.rotation.slerp(target, alpha);
    }
}
