//! Planet One — Fase 0: bootstrap.
//! Finestra, camera in terza persona statica, cubo e piano terra.
//! Il vero cel-shading (toon shader + outline) arriva in Fase 2.

use bevy::core_pipeline::schedule::Core3d;
use bevy::prelude::*;
use bevy::render::camera::CameraRenderGraph;

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
        .insert_resource(ClearColor(Color::srgb(0.02, 0.03, 0.05)))
        .add_systems(Startup, setup)
        .run();
}

/// Setup iniziale della scena: cubo, piano, luce e camera.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Cubo protagonista (placeholder per il personaggio).
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.45, 0.15),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Piano terra.
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

    // Camera in terza persona (statica in Fase 0).
    // CameraRenderGraph va inserito prima di Camera3d per evitare un warning
    // spurio dell'hook d'inserimento in Bevy 0.20.
    commands.spawn((
        CameraRenderGraph::new(Core3d),
        Camera3d::default(),
        Transform::from_xyz(-4.0, 3.5, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
