//! Planet One — RPG 3D open source, cel-shaded.
//! Fase 1: movimento WASD relativo alla camera + orbita mouse + fisica (Avian).

mod camera;
mod player;

use avian3d::prelude::*;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Planet One".into(),
                resolution: (1280u32, 720u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        .insert_resource(ClearColor(Color::srgb(0.02, 0.03, 0.05)))
        .init_resource::<camera::OrbitCamera>()
        .add_systems(Startup, (setup, player::spawn))
        .add_systems(
            Update,
            (
                player::movement,
                player::facing,
                camera::orbit,
                camera::follow,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                player::movement,
                player::facing,
                camera::orbit,
                camera::follow,
            )
                .chain(),
        )
        .run();
}

/// Scena statica: terreno fisico+visuale, luci e camera.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Terreno: collider statico + mesh visuale.
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(40.0, 1.0, 40.0),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d {
            half_size: Vec2::splat(20.0),
            ..default()
        })),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.12, 0.10),
            ..default()
        })),
    ));

    // Luce ambientale soft.
    commands.spawn((AmbientLight {
        color: Color::srgb(0.6, 0.7, 0.9),
        brightness: 80.0,
        ..default()
    },));

    // Luce direzionale (sole) con ombre.
    commands.spawn((
        DirectionalLight {
            illuminance: 2500.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Camera: il transform iniziale viene poi guidato dal sistema di follow.
    // Nota: all'avvio Bevy 0.20 stampa un warning "render graph not configured"
    // riferito a entità Observer interne (upstream, non nostro): la nostra
    // camera ha il graph registrato dalle required components di Camera3d.
    commands.spawn((Camera3d::default(), Transform::from_xyz(-4.0, 3.5, 6.0)));
}
