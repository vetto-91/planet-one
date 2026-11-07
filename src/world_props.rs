//! Oggetti veri del mondo (alberi, fiori, panchine, lampioni) creati dai
//! piazzamenti indicati in modalità debug (`debug/placements.txt`, vedi
//! `debug_place`). Sono modelli low-poly a parallelepipedi/coni: si rigenerano
//! da soli quando i piazzamenti cambiano (F4) o dopo il reset della scena.
//!
//! Gli alberi non vengono messi dove darebbero fastidio (spawn del giocatore,
//! casetta, pozione, zona del ladro): vengono saltati e il log `[PROPS]` lo dice.

use crate::daynight::GameClock;
use crate::debug_place::DebugPlace;
use crate::props::SnapToGround;
use crate::scene::SceneNode;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Tipi di piazzamento che hanno un modello vero (niente segnaposto F4).
pub fn has_model(kind: &str) -> bool {
    matches!(kind, "albero" | "fiori" | "panchina" | "lampione")
}

/// Marker degli oggetti creati da qui.
#[derive(Component)]
pub struct WorldProp;

/// Luce di un lampione.
#[derive(Component)]
pub struct LampLight;

/// Lanterna emissiva di un lampione (materiale condiviso).
#[derive(Component)]
pub struct LampBulb(Handle<StandardMaterial>);

/// Zone vietate agli alberi: (centro, raggio, motivo).
const TREE_KEEP_OUT: [(Vec2, f32, &str); 5] = [
    (Vec2::new(0.0, 0.0), 3.5, "spawn del giocatore"),
    (Vec2::new(6.5, 0.7), 5.5, "casetta del Viandante"),
    (Vec2::new(6.0, -5.0), 1.5, "pozione"),
    (Vec2::new(31.2, -34.9), 3.0, "ladro"),
    (Vec2::new(27.2, -34.9), 3.0, "percorso del ladro"),
];

fn tree_blocked(p: Vec2) -> Option<&'static str> {
    TREE_KEEP_OUT
        .iter()
        .find(|(c, r, _)| c.distance(p) < *r)
        .map(|(_, _, why)| *why)
}

/// Numero pseudo-casuale stabile in 0..1 dalle coordinate (stessa variazione a ogni avvio).
fn hash01(p: Vec2, salt: f32) -> f32 {
    ((p.x * 12.9898 + p.y * 78.233 + salt * 37.719).sin() * 43_758.545)
        .fract()
        .abs()
}

fn mat(materials: &mut Assets<StandardMaterial>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.95,
        ..default()
    })
}

/// Sistema: (ri)crea gli oggetti quando cambiano i piazzamenti o la scena è stata azzerata.
#[allow(clippy::too_many_arguments)]
pub fn sync_world_props(
    mut commands: Commands,
    dbg: Res<DebugPlace>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    props: Query<Entity, With<WorldProp>>,
    mut built: Local<(Option<u32>, usize)>,
) {
    let stale = built.0 != Some(dbg.revision());
    let wiped = props.is_empty() && built.1 > 0;
    if !stale && !wiped {
        return;
    }
    for e in &props {
        commands.entity(e).despawn();
    }
    let trunk = mat(&mut materials, Color::srgb(0.38, 0.26, 0.15));
    let wood = mat(&mut materials, Color::srgb(0.55, 0.38, 0.22));
    let iron = mat(&mut materials, Color::srgb(0.16, 0.17, 0.2));
    let stem = mat(&mut materials, Color::srgb(0.25, 0.55, 0.22));
    let leaves = [
        mat(&mut materials, Color::srgb(0.16, 0.5, 0.2)),
        mat(&mut materials, Color::srgb(0.22, 0.58, 0.24)),
        mat(&mut materials, Color::srgb(0.12, 0.42, 0.22)),
    ];
    let petals = [
        mat(&mut materials, Color::srgb(0.95, 0.45, 0.7)),
        mat(&mut materials, Color::srgb(0.98, 0.85, 0.25)),
        mat(&mut materials, Color::srgb(0.96, 0.96, 0.92)),
        mat(&mut materials, Color::srgb(0.9, 0.25, 0.25)),
        mat(&mut materials, Color::srgb(0.6, 0.4, 0.9)),
    ];
    let bulb_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.55),
        emissive: LinearRgba::rgb(0.2, 0.15, 0.05),
        ..default()
    });
    let trunk_mesh = meshes.add(Cylinder::new(0.2, 1.7));
    let cones = [
        meshes.add(Cone::new(1.35, 1.9)),
        meshes.add(Cone::new(1.05, 1.7)),
        meshes.add(Cone::new(0.72, 1.5)),
    ];
    let stem_mesh = meshes.add(Cylinder::new(0.015, 0.35));
    let head_mesh = meshes.add(Sphere::new(0.09));
    let seat_mesh = meshes.add(Cuboid::new(1.5, 0.08, 0.45));
    let back_mesh = meshes.add(Cuboid::new(1.5, 0.36, 0.06));
    let leg_mesh = meshes.add(Cuboid::new(0.08, 0.45, 0.08));
    let pole_mesh = meshes.add(Cylinder::new(0.06, 3.0));
    let base_mesh = meshes.add(Cylinder::new(0.15, 0.3));
    let lantern_mesh = meshes.add(Cuboid::new(0.28, 0.4, 0.28));
    let cap_mesh = meshes.add(Cone::new(0.26, 0.22));

    let (mut spawned, mut skipped) = (0usize, 0usize);
    for pl in &dbg.items {
        for &p in &pl.points {
            let root = (
                WorldProp,
                SceneNode,
                SnapToGround {
                    offset: 0.0,
                    half: Vec2::ZERO,
                },
                Transform::from_xyz(p.x, 0.0, p.y),
                Visibility::default(),
            );
            let h = hash01(p, 1.0);
            match pl.kind.as_str() {
                "albero" => {
                    if let Some(why) = tree_blocked(p) {
                        info!("[PROPS] albero in ({:.1},{:.1}) saltato: {why}", p.x, p.y);
                        skipped += 1;
                        continue;
                    }
                    let scale = 0.85 + hash01(p, 2.0) * 0.45;
                    let leaf = leaves[(h * 3.0) as usize % 3].clone();
                    commands
                        .spawn((root, RigidBody::Static))
                        .insert(
                            Transform::from_xyz(p.x, 0.0, p.y)
                                .with_rotation(Quat::from_rotation_y(h * std::f32::consts::TAU))
                                .with_scale(Vec3::splat(scale)),
                        )
                        .with_children(|c| {
                            c.spawn((
                                Mesh3d(trunk_mesh.clone()),
                                MeshMaterial3d(trunk.clone()),
                                Transform::from_xyz(0.0, 0.85, 0.0),
                            ));
                            for (i, (cone, y)) in cones.iter().zip([2.3, 3.2, 4.0]).enumerate() {
                                let _ = i;
                                c.spawn((
                                    Mesh3d(cone.clone()),
                                    MeshMaterial3d(leaf.clone()),
                                    Transform::from_xyz(0.0, y, 0.0),
                                ));
                            }
                            c.spawn((
                                Collider::cylinder(0.3, 2.0),
                                Transform::from_xyz(0.0, 1.0, 0.0),
                            ));
                        });
                }
                "fiori" => {
                    commands.spawn(root).with_children(|c| {
                        for k in 0..3 {
                            let a = k as f32 * 2.1 + h * 6.0;
                            let off = Vec3::new(a.cos() * 0.12, 0.0, a.sin() * 0.12);
                            let petal = petals[((h * 5.0) as usize + k) % 5].clone();
                            c.spawn((
                                Mesh3d(stem_mesh.clone()),
                                MeshMaterial3d(stem.clone()),
                                Transform::from_translation(off + Vec3::Y * 0.175),
                            ));
                            c.spawn((
                                Mesh3d(head_mesh.clone()),
                                MeshMaterial3d(petal),
                                Transform::from_translation(off + Vec3::Y * 0.37),
                            ));
                        }
                    });
                }
                "panchina" => {
                    commands
                        .spawn((root, RigidBody::Static))
                        .insert(Transform::from_xyz(p.x, 0.0, p.y).with_rotation(
                            Quat::from_rotation_y((h * 4.0).floor() * std::f32::consts::FRAC_PI_2),
                        ))
                        .with_children(|c| {
                            c.spawn((
                                Mesh3d(seat_mesh.clone()),
                                MeshMaterial3d(wood.clone()),
                                Transform::from_xyz(0.0, 0.47, 0.0),
                            ));
                            c.spawn((
                                Mesh3d(back_mesh.clone()),
                                MeshMaterial3d(wood.clone()),
                                Transform::from_xyz(0.0, 0.78, -0.2),
                            ));
                            for x in [-0.65, 0.65] {
                                for z in [-0.17, 0.17] {
                                    c.spawn((
                                        Mesh3d(leg_mesh.clone()),
                                        MeshMaterial3d(iron.clone()),
                                        Transform::from_xyz(x, 0.22, z),
                                    ));
                                }
                            }
                            c.spawn((
                                Collider::cuboid(1.5, 0.9, 0.5),
                                Transform::from_xyz(0.0, 0.45, 0.0),
                            ));
                        });
                }
                "lampione" => {
                    commands
                        .spawn((root, RigidBody::Static))
                        .with_children(|c| {
                            c.spawn((
                                Mesh3d(base_mesh.clone()),
                                MeshMaterial3d(iron.clone()),
                                Transform::from_xyz(0.0, 0.15, 0.0),
                            ));
                            c.spawn((
                                Mesh3d(pole_mesh.clone()),
                                MeshMaterial3d(iron.clone()),
                                Transform::from_xyz(0.0, 1.5, 0.0),
                            ));
                            c.spawn((
                                LampBulb(bulb_mat.clone()),
                                Mesh3d(lantern_mesh.clone()),
                                MeshMaterial3d(bulb_mat.clone()),
                                Transform::from_xyz(0.0, 3.15, 0.0),
                            ));
                            c.spawn((
                                Mesh3d(cap_mesh.clone()),
                                MeshMaterial3d(iron.clone()),
                                Transform::from_xyz(0.0, 3.46, 0.0),
                            ));
                            c.spawn((
                                LampLight,
                                PointLight {
                                    color: Color::srgb(1.0, 0.8, 0.5),
                                    intensity: 0.0,
                                    range: 11.0,
                                    shadow_maps_enabled: false,
                                    ..default()
                                },
                                Transform::from_xyz(0.0, 3.1, 0.0),
                            ));
                            c.spawn((
                                Collider::cylinder(0.12, 3.0),
                                Transform::from_xyz(0.0, 1.5, 0.0),
                            ));
                        });
                }
                _ => continue,
            }
            spawned += 1;
        }
    }
    info!("[PROPS] oggetti veri creati: {spawned} (alberi saltati: {skipped})");
    *built = (Some(dbg.revision()), spawned);
}

/// I lampioni si accendono al tramonto e si spengono all'alba (con dissolvenza).
pub fn update_street_lamps(
    time: Res<Time>,
    clock: Res<GameClock>,
    mut lights: Query<&mut PointLight, With<LampLight>>,
    bulbs: Query<&LampBulb>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let on = clock.hour >= 17.5 || clock.hour < 6.2;
    let k = (time.delta_secs() * 1.5).min(1.0);
    let mut level = 0.0;
    for mut l in &mut lights {
        let target = if on { 220_000.0 } else { 0.0 };
        l.intensity += (target - l.intensity) * k;
        level = l.intensity / 220_000.0;
    }
    if let Some(b) = bulbs.iter().next()
        && let Some(mut m) = materials.get_mut(&b.0)
    {
        m.emissive = LinearRgba::rgb(0.2 + 6.0 * level, 0.15 + 4.2 * level, 0.05 + 1.8 * level);
    }
}
