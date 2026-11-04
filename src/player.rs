//! Player: spawn, movimento WASD relativo alla camera e orientamento del mesh.

use avian3d::prelude::*;
use bevy::prelude::*;

/// Marker del corpo fisico del giocatore.
#[derive(Component)]
pub struct Player;

/// Marker del gruppo visivo del giocatore (figlio del corpo fisico):
/// ruota per guardare nella direzione di movimento.
#[derive(Component)]
pub struct PlayerVisual;

/// Velocità di camminata (m/s).
const WALK_SPEED: f32 = 6.0;

/// Spawn del giocatore: capsule dinamica (corpo) + mesh visivo (figlio).
pub fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn((
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

/// Movimento WASD relativo all'orientamento della camera (piano XZ).
/// La componente verticale della velocità è lasciata alla gravità.
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
        return;
    }

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
        velocity.x = direction.x * WALK_SPEED;
        velocity.z = direction.z * WALK_SPEED;
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
        return;
    }

    // Yaw che porta il -Z locale (il "naso") nella direzione di movimento.
    let target_yaw = (-velocity.x).atan2(-velocity.z);
    let target = Quat::from_rotation_y(target_yaw);

    for mut transform in &mut visuals {
        let alpha = (10.0 * time.delta_secs()).clamp(0.0, 1.0);
        transform.rotation = transform.rotation.slerp(target, alpha);
    }
}
