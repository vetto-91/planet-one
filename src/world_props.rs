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
    matches!(
        kind,
        "albero" | "fiori" | "panchina" | "lampione" | "muro" | "cartello" | "campo" | "orto"
    ) || crate::buildings::is_building(kind)
}

/// Marker degli oggetti creati da qui.
#[derive(Component)]
pub struct WorldProp;

/// Luce di un lampione.
#[derive(Component)]
pub struct LampLight;

/// Testa girevole del faro e luci che si accendono di sera.
#[derive(Component)]
pub struct LighthouseHead;
#[derive(Component)]
pub struct LighthouseBeam;

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

/// Zone vietate agli alberi come (centro, raggio) per il debug F4 (ingombro albero già dedotto).
pub fn tree_keep_out() -> Vec<(Vec2, f32)> {
    TREE_KEEP_OUT
        .iter()
        .map(|(c, r, _)| (*c, (*r - 1.2).max(0.0)))
        .collect()
}

fn tree_blocked(p: Vec2) -> Option<&'static str> {
    TREE_KEEP_OUT
        .iter()
        .find(|(c, r, _)| c.distance(p) < *r)
        .map(|(_, _, why)| *why)
}

/// Numero pseudo-casuale stabile in 0..1 dalle coordinate (stessa variazione a ogni avvio).
pub fn hash01(p: Vec2, salt: f32) -> f32 {
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
    asset_server: Res<AssetServer>,
    pbr: Res<crate::pbr::PbrMaterials>,
    village: Res<crate::village::VillageInfo>,
    field: Option<Res<crate::terrain::TerrainField>>,
    props: Query<Entity, With<WorldProp>>,
    mut built: Local<(Option<u32>, usize)>,
) {
    // Prima che il terreno esista non si può appoggiare nulla: si aspetta.
    let Some(field) = field else {
        return;
    };
    let stale = built.0 != Some(dbg.revision());
    let wiped = props.is_empty() && built.1 > 0;
    if !stale && !wiped {
        return;
    }
    for e in &props {
        commands.entity(e).despawn();
    }
    let wood = mat(&mut materials, Color::srgb(0.55, 0.38, 0.22));
    let iron = mat(&mut materials, Color::srgb(0.16, 0.17, 0.2));
    let stem = mat(&mut materials, Color::srgb(0.25, 0.55, 0.22));
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
    let firs = crate::trees::FirAssets::new(&mut meshes, &mut materials, &asset_server);
    let stem_mesh = meshes.add(Cylinder::new(0.015, 0.41));
    let head_mesh = meshes.add(Sphere::new(0.09));
    let seat_mesh = meshes.add(Cuboid::new(1.5, 0.08, 0.45));
    let back_mesh = meshes.add(Cuboid::new(1.5, 0.36, 0.06));
    let leg_mesh = meshes.add(Cuboid::new(0.08, 0.7, 0.08));
    let pole_mesh = meshes.add(Cylinder::new(0.06, 3.4));
    let base_mesh = meshes.add(Cylinder::new(0.15, 0.7));
    let lantern_mesh = meshes.add(Cuboid::new(0.28, 0.4, 0.28));
    let cap_mesh = meshes.add(Cone::new(0.26, 0.22));

    let building_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.95,
        ..default()
    });
    let snow_mat = crate::seasons::snow_material(&mut materials);
    let (mut spawned, mut skipped) = (0usize, 0usize);
    for pl in &dbg.items {
        if matches!(pl.kind.as_str(), "campo" | "orto") {
            // Campo coltivato: una sola mesh per tutto il piazzamento.
            let pts: Vec<Vec2> = pl
                .points
                .iter()
                .copied()
                .filter(|p| !crate::terrain::is_water(p.x, p.y))
                .collect();
            let mesh =
                crate::buildings::crop_field_mesh(&pl.kind, &pts, &|p| field.height_at(p.x, p.y));
            commands.spawn((
                WorldProp,
                SceneNode,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(building_mat.clone()),
                Transform::default(),
                Visibility::default(),
            ));
            spawned += pts.len();
            continue;
        }
        for (j, &p) in pl.points.iter().enumerate() {
            // Molo e barche stanno apposta in mare.
            let at_sea = matches!(pl.kind.as_str(), "molo" | "barca");
            if !at_sea && crate::terrain::is_water(p.x, p.y) {
                info!(
                    "[PROPS] oggetto in ({:.1},{:.1}) saltato: in mare",
                    p.x, p.y
                );
                skipped += 1;
                continue;
            }
            let yaw = pl.yaw(j);
            if crate::buildings::is_building(&pl.kind) {
                let seed =
                    (p.x * 100.0) as i32 as u32 ^ ((p.y * 100.0) as i32 as u32).rotate_left(11);
                let model = crate::buildings::model(&pl.kind, seed);
                // Molo e barche stanno sul mare (quota fissa), il resto si appoggia al terreno.
                let fixed_y = match pl.kind.as_str() {
                    "molo" => Some(crate::terrain::SEA_LEVEL),
                    "barca" => Some(crate::terrain::SEA_LEVEL + 0.05),
                    _ => None,
                };
                let kind = pl.kind.clone();
                let mut e = commands.spawn((
                    WorldProp,
                    SceneNode,
                    Transform::from_xyz(p.x, fixed_y.unwrap_or(0.0), p.y)
                        .with_rotation(Quat::from_rotation_y(yaw)),
                    Visibility::default(),
                    RigidBody::Static,
                ));
                if fixed_y.is_none() {
                    e.insert(SnapToGround {
                        offset: -0.05,
                        half: model.half,
                    });
                }
                e.with_children(|c| {
                    if model.snow.count_vertices() > 0 {
                        c.spawn((
                            crate::seasons::SeasonSnowRoof,
                            Mesh3d(meshes.add(model.snow)),
                            MeshMaterial3d(snow_mat.clone()),
                            bevy::light::NotShadowCaster,
                            Visibility::Hidden,
                        ));
                    }
                    for (m, mesh) in model.parts {
                        c.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(pbr.get(m))));
                    }
                    for (i, (center, size)) in model.colliders.into_iter().enumerate() {
                        let mut col = c.spawn((
                            Collider::cuboid(size.x, size.y, size.z),
                            Transform::from_translation(center),
                            crate::camera::BlocksCamera,
                        ));
                        // Il piano del molo è "terreno" per il controllo di appoggio del giocatore.
                        if kind == "molo" && i == 0 {
                            col.insert(crate::scene::Ground);
                        }
                    }
                    if kind == "faro" {
                        // Testa del faro: ruota; dentro c'è un faro orientabile e un bagliore fisso.
                        c.spawn((
                            LighthouseHead,
                            Transform::from_xyz(0.0, 14.55, 0.0),
                            Visibility::default(),
                        ))
                        .with_children(|h| {
                            h.spawn((
                                LighthouseBeam,
                                SpotLight {
                                    color: Color::srgb(1.0, 0.92, 0.7),
                                    intensity: 0.0,
                                    range: 160.0,
                                    outer_angle: 0.18,
                                    inner_angle: 0.05,
                                    shadow_maps_enabled: false,
                                    ..default()
                                },
                                Transform::from_xyz(0.0, 0.0, -0.4)
                                    .looking_to(Vec3::new(0.0, -0.06, -1.0), Vec3::Y),
                            ));
                        });
                        c.spawn((
                            LighthouseBeam,
                            PointLight {
                                color: Color::srgb(1.0, 0.88, 0.6),
                                intensity: 0.0,
                                range: 14.0,
                                shadow_maps_enabled: false,
                                ..default()
                            },
                            Transform::from_xyz(0.0, 14.7, 0.0),
                        ));
                    }
                });
                spawned += 1;
                continue;
            }
            if pl.kind == "muro" {
                continue; // il muro è generato da `village` dai vertici
            }
            let half = match pl.kind.as_str() {
                "panchina" => Vec2::new(0.75, 0.25),
                "albero" => Vec2::splat(0.3),
                "lampione" => Vec2::splat(0.15),
                _ => Vec2::ZERO,
            };
            let root = (
                WorldProp,
                SceneNode,
                SnapToGround {
                    offset: -0.05,
                    half,
                },
                Transform::from_xyz(p.x, 0.0, p.y).with_rotation(Quat::from_rotation_y(yaw)),
                Visibility::default(),
            );
            let h = hash01(p, 1.0);
            match pl.kind.as_str() {
                "cartello" => {
                    // Palo con tabella: la chiave della texture è nella nota del piazzamento.
                    let key = if pl.note.is_empty() {
                        "fontevivo"
                    } else {
                        pl.note.as_str()
                    };
                    let sign_mat = materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        base_color_texture: Some(
                            asset_server.load(format!("textures/cartelli/{key}.png")),
                        ),
                        perceptual_roughness: 0.9,
                        ..default()
                    });
                    commands
                        .spawn((root, RigidBody::Static))
                        .with_children(|c| {
                            c.spawn((
                                // Il palo sta dietro la tabella: non copre la scritta.
                                Mesh3d(meshes.add(Cuboid::new(0.12, 2.9, 0.12))),
                                MeshMaterial3d(wood.clone()),
                                Transform::from_xyz(0.0, 1.25, -0.1),
                            ));
                            c.spawn((
                                Mesh3d(meshes.add(Cuboid::new(1.74, 0.68, 0.07))),
                                MeshMaterial3d(wood.clone()),
                                Transform::from_xyz(0.0, 2.05, 0.0),
                            ));
                            c.spawn((
                                Mesh3d(meshes.add(Rectangle::new(1.66, 0.62))),
                                MeshMaterial3d(sign_mat),
                                Transform::from_xyz(0.0, 2.05, 0.04),
                            ));
                            c.spawn((
                                Collider::cuboid(0.3, 2.8, 0.3),
                                Transform::from_xyz(0.0, 1.25, -0.05),
                            ));
                        });
                }
                "albero" => {
                    if crate::village::in_polygon(&village.poly, p) {
                        info!(
                            "[PROPS] albero in ({:.1},{:.1}) saltato: dentro le mura",
                            p.x, p.y
                        );
                        skipped += 1;
                        continue;
                    }
                    if let Some(why) = tree_blocked(p) {
                        info!("[PROPS] albero in ({:.1},{:.1}) saltato: {why}", p.x, p.y);
                        skipped += 1;
                        continue;
                    }
                    let scale = 0.85 + hash01(p, 2.0) * 0.45;
                    let variant = (h * 3.0) as usize % 3;
                    commands
                        .spawn((root, RigidBody::Static))
                        .insert(
                            Transform::from_xyz(p.x, 0.0, p.y)
                                .with_rotation(Quat::from_rotation_y(yaw))
                                .with_scale(Vec3::splat(scale)),
                        )
                        .with_children(|c| {
                            c.spawn((
                                Mesh3d(firs.trunk.clone()),
                                MeshMaterial3d(firs.bark.clone()),
                            ));
                            c.spawn((
                                crate::seasons::SeasonLeaf(variant),
                                Mesh3d(firs.foliage[variant].clone()),
                                MeshMaterial3d(firs.needles[variant].clone()),
                            ));
                            // Neve sui rami: stessa chioma, appena più in alto, solo d'inverno.
                            c.spawn((
                                crate::seasons::SeasonSnowRoof,
                                Mesh3d(firs.foliage[variant].clone()),
                                MeshMaterial3d(firs.snow.clone()),
                                Transform::from_xyz(0.0, 0.04, 0.0),
                                Visibility::Hidden,
                                bevy::light::NotShadowCaster,
                            ));
                            c.spawn((
                                Collider::cylinder(0.3, 2.4),
                                Transform::from_xyz(0.0, 0.8, 0.0),
                            ));
                        });
                }
                "fiori" => {
                    commands
                        .spawn(root)
                        .insert(crate::seasons::SeasonFlower {
                            threshold: hash01(p, 5.0) * 0.7,
                        })
                        .with_children(|c| {
                            for k in 0..3 {
                                let a = k as f32 * 2.1 + h * 6.0;
                                let off = Vec3::new(a.cos() * 0.12, 0.0, a.sin() * 0.12);
                                let petal = petals[((h * 5.0) as usize + k) % 5].clone();
                                c.spawn((
                                    Mesh3d(stem_mesh.clone()),
                                    MeshMaterial3d(stem.clone()),
                                    Transform::from_translation(off + Vec3::Y * 0.145),
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
                                        Transform::from_xyz(x, 0.1, z),
                                    ));
                                }
                            }
                            c.spawn((
                                Collider::cuboid(1.5, 1.15, 0.5),
                                Transform::from_xyz(0.0, 0.325, 0.0),
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
                                Transform::from_xyz(0.0, -0.05, 0.0),
                            ));
                            c.spawn((
                                Mesh3d(pole_mesh.clone()),
                                MeshMaterial3d(iron.clone()),
                                Transform::from_xyz(0.0, 1.3, 0.0),
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
                                    shadow_depth_bias: 0.012,
                                    shadow_normal_bias: 0.25,
                                    ..default()
                                },
                                Transform::from_xyz(0.0, 3.1, 0.0),
                            ));
                            c.spawn((
                                Collider::cylinder(0.12, 3.4),
                                Transform::from_xyz(0.0, 1.3, 0.0),
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

/// Potenza dei lampioni (lm), calibrata sulla resa HDR.
const LAMP_LUMENS: f32 = 50_000.0;

/// I lampioni si accendono al tramonto e si spengono all'alba (con dissolvenza).
pub fn update_street_lamps(
    time: Res<Time>,
    clock: Res<GameClock>,
    weather: Res<crate::weather::WeatherState>,
    mut lights: Query<(&GlobalTransform, &mut PointLight), With<LampLight>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    bulbs: Query<&LampBulb>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Accesi anche con il cielo molto scuro (temporale).
    let on = clock.lamps_on() || weather.cloud > 0.95;
    let k = (time.delta_secs() * 1.5).min(1.0);
    let mut level = 0.0;
    // Le ombre (luce che non passa le pareti) solo per i 3 lampioni più vicini alla camera, entro 30 m.
    let cam = cameras
        .single()
        .map(|c| c.translation())
        .unwrap_or_default();
    let mut dists: Vec<f32> = lights
        .iter()
        .map(|(t, _)| t.translation().distance(cam))
        .collect();
    dists.sort_by(|a, b| a.total_cmp(b));
    let limit = dists.get(2).copied().unwrap_or(f32::MAX).min(30.0);
    for (t, mut l) in &mut lights {
        let shadows = t.translation().distance(cam) <= limit;
        if l.shadow_maps_enabled != shadows {
            l.shadow_maps_enabled = shadows;
        }
        let target = if on { LAMP_LUMENS } else { 0.0 };
        l.intensity += (target - l.intensity) * k;
        level = l.intensity / LAMP_LUMENS;
    }
    if let Some(b) = bulbs.iter().next()
        && let Some(mut m) = materials.get_mut(&b.0)
    {
        m.emissive = LinearRgba::rgb(0.2 + 6.0 * level, 0.15 + 4.2 * level, 0.05 + 1.8 * level);
    }
}

/// Sistema: il faro si accende al tramonto e il fascio gira (un giro ogni ~8 s).
pub fn update_lighthouse(
    time: Res<Time>,
    clock: Res<GameClock>,
    mut heads: Query<&mut Transform, With<LighthouseHead>>,
    mut spots: Query<&mut SpotLight, With<LighthouseBeam>>,
    mut points: Query<&mut PointLight, With<LighthouseBeam>>,
) {
    for mut tf in &mut heads {
        tf.rotate_y(time.delta_secs() * 0.8);
    }
    let dark = clock.lighthouse_dark();
    let k = (time.delta_secs() * 1.5).min(1.0);
    for mut l in &mut spots {
        let target = if dark { 40_000_000.0 } else { 0.0 };
        l.intensity += (target - l.intensity) * k;
    }
    for mut l in &mut points {
        let target = if dark { 200_000.0 } else { 0.0 };
        l.intensity += (target - l.intensity) * k;
    }
}
