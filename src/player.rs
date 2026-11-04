//! Player: spawn, movimento WASD+sprint relativo alla camera, salto e facing.

use crate::scene::{Ground, SceneNode};
use avian3d::prelude::*;
use bevy::prelude::*;

/// Marker del corpo fisico del giocatore.
#[derive(Component)]
pub struct Player;

/// Marker del gruppo visivo del giocatore (figlio del corpo fisico):
/// ruota per guardare nella direzione di movimento.
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
/// Soglia oltre la quale il player non è considerato a terra (m).
/// 0.85 = semi-altezza della capsula (raggio 0.35 + semi-lunghezza 0.5).
const GROUND_CHECK_DISTANCE: f32 = 0.85;
/// Pelle di tolleranza sotto i piedi (m): un po' di margine evita
/// "salto che non parte" su micro-irregolarità del contatto.
const GROUND_SKIN: f32 = 0.10;
/// Soglia di velocità verticale sopra la quale il salto è bloccato
/// (evita il doppio salto nei primi frame di ascesa).
const MAX_RISE_FOR_JUMP: f32 = 0.5;

/// Spawn del giocatore: capsule dinamica (corpo) + mesh visivo (figlio).
/// La radice è marcata `SceneNode` per partecipare al reset della scena.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands
        .spawn((
            SceneNode,
            Player,
            RigidBody::Dynamic,
            Collider::capsule(0.35, 1.0),
            LockedAxes::ROTATION_LOCKED,
            LinearVelocity::ZERO,
            // Visibility: serve per la propagazione dei figli mesh (evita B0004).
            Visibility::default(),
            Transform::from_xyz(0.0, 1.0, 0.0),
        ))
        .with_children(|body| {
            // Gruppo visivo: ruota verso la direzione di movimento.
            // Transform + Visibility servono per la propagazione dai figli mesh.
            body.spawn((PlayerVisual, Transform::default(), Visibility::default()))
                .with_children(|visual| {
                    // Corpo (placeholder per il personaggio).
                    visual.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.7, 1.3, 0.7))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.85, 0.45, 0.15),
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

/// Movimento WASD/frecce relativo all'orientamento della camera (piano XZ).
/// `Shift` attiva la corsa (moltiplicatore [`SPRINT_FACTOR`]).
/// La componente verticale della velocità è lasciata a gravità e salto.
pub fn movement(
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut players: Query<&mut LinearVelocity, With<Player>>,
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
    if input == Vec2::ZERO {
        // Nessun input: azzera la velocità orizzontale (stop secco).
        for mut velocity in &mut players {
            velocity.x = 0.0;
            velocity.z = 0.0;
        }
        return;
    }

    // Corsa con Shift (sinistro o destro).
    let sprinting = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let speed = WALK_SPEED * if sprinting { SPRINT_FACTOR } else { 1.0 };

    // Forward della camera proiettato sul piano XZ (forward = -Z locale).
    let cam_forward = camera.forward().normalize_or_zero();
    let forward = Vec3::new(cam_forward.x, 0.0, cam_forward.z).normalize_or_zero();
    let right = Vec3::new(cam_forward.z, 0.0, -cam_forward.x).normalize_or_zero();

    // Se la camera guarda quasi verticale, il forward degenera: usa -Z di sicurezza.
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

    for mut velocity in &mut players {
        velocity.x = direction.x * speed;
        velocity.z = direction.z * speed;
    }
}

/// Ground-check: raycast verso il basso dal centro della capsula.
///
/// Il player è considerato a terra (componente [`Grounded`] aggiunta) se il
/// raycast colpisce un collider marcato [`Ground`] entro la semi-altezza
/// della capsula più una piccola pelle di tolleranza. Il ray esclude il
/// collider del player stesso (senza il filtro colpirebbe la propria
/// capsula). Restringere al marker `Ground` evita che muri o ostacoli
/// futuri vengano scambiati per suolo calpestabile.
pub fn update_grounded(
    spatial_query: SpatialQuery,
    mut commands: Commands,
    grounds: Query<Entity, With<Ground>>,
    players: Query<(Entity, &GlobalTransform), With<Player>>,
) {
    for (entity, transform) in &players {
        // Il ray parte dal centro della capsula e guarda in basso.
        // Il predicato accetta solo collider marcati `Ground`.
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

/// Salto: impulso verticale su `just_pressed(Space)`, solo se a terra
/// e non già in ascesa (evita il double-jump).
pub fn jump(
    keys: Res<ButtonInput<KeyCode>>,
    mut players: Query<(&mut LinearVelocity, Option<&Grounded>), With<Player>>,
) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    for (mut velocity, grounded) in &mut players {
        let grounded = grounded.is_some();
        if grounded && velocity.y <= MAX_RISE_FOR_JUMP {
            velocity.y = JUMP_SPEED;
        }
    }
}

/// Ruota il mesh del giocatore verso la direzione di movimento (smooth).
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

    // Yaw che porta il -Z locale (il "naso") nella direzione di movimento.
    let target_yaw = (-velocity.x).atan2(-velocity.z);
    let target = Quat::from_rotation_y(target_yaw);

    for mut transform in &mut visuals {
        let alpha = (10.0 * time.delta_secs()).clamp(0.0, 1.0);
        transform.rotation = transform.rotation.slerp(target, alpha);
    }
}
