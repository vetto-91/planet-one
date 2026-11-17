//! Armi e scudo impugnati: modelli semplici agganciati alle ossa delle mani del giocatore.

use crate::inventory::{Equipment, ItemKind};
use crate::player::PlayerVisual;
use bevy::prelude::*;

/// Marker dei pezzi impugnati (arma e scudo).
#[derive(Component)]
pub struct HeldItem;

/// Radice dell'arma impugnata (segue l'osso della mano).
#[derive(Component)]
pub struct HeldWeapon;

/// Rotazione dell'arma a riposo nel riferimento dell'osso della mano: la lama esce dal lato del pollice.
const REST_TILT: f32 = std::f32::consts::FRAC_PI_2;
/// Posizione dell'impugnatura nel palmo (riferimento dell'osso `hand_r`).
const GRIP: Vec3 = Vec3::new(-0.02, 0.09, 0.0);

/// Posizione e rotazione dell'arma sulla schiena (riferimento dell'osso `spine_03`).
const BACK_POS: Vec3 = Vec3::new(0.0, 0.05, 0.16);
const BACK_TILT: Vec3 = Vec3::new(0.0, 0.0, 2.2);

/// Sistema: in corsa le braccia portano la mano vicino alla testa e l'arma ci finirebbe dentro,
/// quindi si porta sulla schiena finché dura la corsa e torna nella mano appena ci si ferma.
#[allow(clippy::type_complexity)]
pub fn stow_weapon_while_running(
    mut commands: Commands,
    equip: Res<Equipment>,
    animators: Query<(&crate::animation::Animator, &crate::figure::HandBones), With<PlayerVisual>>,
    mut weapons: Query<
        (Entity, &mut Transform, &ChildOf),
        (With<HeldWeapon>, Without<crate::figure::AttachToHand>),
    >,
) {
    let Ok((animator, bones)) = animators.single() else {
        return;
    };
    let (parent, pos, rot) = match (animator.running(), bones.back) {
        // Con la bisaccia l'arma va sopra di essa, non dentro.
        (true, Some(back)) => (
            back,
            BACK_POS + Vec3::Z * if equip.backpack.is_some() { 0.14 } else { 0.0 },
            Quat::from_euler(EulerRot::XYZ, BACK_TILT.x, BACK_TILT.y, BACK_TILT.z),
        ),
        _ => (bones.right, GRIP, Quat::from_rotation_x(REST_TILT)),
    };
    for (e, mut tf, child_of) in &mut weapons {
        if child_of.parent() != parent {
            commands.entity(e).try_insert(ChildOf(parent));
            tf.translation = pos;
            tf.rotation = rot;
        }
    }
}

/// La copia dell'arma vista in prima persona, figlia della camera (ADR 0059).
#[derive(Component)]
pub struct ViewModel(pub ItemKind);

/// Dove sta l'arma in prima persona, nel riferimento della camera: l'arco nella sinistra, dritto e
/// un poco inclinato; le lame e l'ascia nella destra, con la punta in avanti.
pub fn view_pose(kind: ItemKind) -> Transform {
    // A metà grandezza: vista da così vicino l'arma vera riempirebbe lo schermo.
    if crate::inventory::weapon_stats(Some(kind)).ranged {
        Transform::from_xyz(-0.24, -0.16, -0.62)
            .with_rotation(
                Quat::from_rotation_y(1.15)
                    * Quat::from_rotation_z(0.22)
                    * Quat::from_rotation_x(-0.08),
            )
            .with_scale(Vec3::splat(0.34))
    } else {
        Transform::from_xyz(0.3, -0.34, -0.55)
            .with_rotation(
                Quat::from_rotation_y(-0.25)
                    * Quat::from_rotation_x(-0.9)
                    * Quat::from_rotation_z(-0.15),
            )
            .with_scale(Vec3::splat(0.55))
    }
}

/// Un piccolo ondeggiare: il respiro da fermi e il passo quando ci si muove.
pub fn view_sway(t: f32, speed: f32) -> Vec3 {
    let walk = (speed / 5.0).clamp(0.0, 1.0);
    let breath = (t * 1.3).sin() * 0.004;
    let step = t * (6.0 + 4.0 * walk);
    Vec3::new(
        step.sin() * 0.012 * walk,
        breath + (2.0 * step).sin().abs() * -0.014 * walk,
        0.0,
    )
}

/// Sistema: in prima persona l'arma della mano si nasconde (passerebbe davanti agli occhi
/// fermandosi dopo una corsa) e se ne mostra una copia ferma davanti alla camera, che ondeggia
/// appena; in terza persona torna quella vera.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn first_person_weapon(
    mut commands: Commands,
    time: Res<Time>,
    orbit: Res<crate::camera::OrbitCamera>,
    equip: Res<Equipment>,
    cameras: Query<Entity, With<Camera3d>>,
    players: Query<&avian3d::prelude::LinearVelocity, With<crate::player::Player>>,
    mut held: Query<(&mut Visibility, Option<&Children>), With<HeldWeapon>>,
    parts: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>, &Transform), Without<ViewModel>>,
    mut views: Query<(Entity, &ViewModel, &mut Transform), Without<HeldWeapon>>,
    mut settle: Local<(Option<ItemKind>, u8)>,
) {
    let first = orbit.mode == crate::camera::CameraMode::FirstPerson;
    for (mut vis, _) in &mut held {
        let want = if first {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        vis.set_if_neq(want);
    }
    let wanted = equip.weapon.filter(|_| first);
    // Arma cambiata o prima persona finita: la copia se ne va.
    for (e, view, _) in &views {
        if Some(view.0) != wanted {
            commands.entity(e).despawn();
        }
    }
    let Some(kind) = wanted else {
        *settle = (None, 0);
        return;
    };
    // Cambiata l'arma, il modello vero si ricrea al frame dopo: si aspetta prima di copiarlo.
    if settle.0 != Some(kind) {
        *settle = (Some(kind), 3);
    }
    if settle.1 > 0 {
        settle.1 -= 1;
        return;
    }
    let speed = players
        .single()
        .map_or(0.0, |v| Vec2::new(v.x, v.z).length());
    let sway = view_sway(time.elapsed_secs(), speed);
    let mut present = false;
    for (_, view, mut tf) in &mut views {
        if view.0 == kind {
            present = true;
            let pose = view_pose(kind);
            tf.translation = pose.translation + sway;
        }
    }
    if present {
        return;
    }
    // La copia nasce dai pezzi dell'arma vera (stessi modelli e materiali).
    let Ok(camera) = cameras.single() else {
        return;
    };
    let Some(children) = held.iter().find_map(|(_, c)| c) else {
        return; // l'arma vera non è ancora pronta: al prossimo frame
    };
    let root = commands
        .spawn((
            ViewModel(kind),
            view_pose(kind),
            Visibility::default(),
            ChildOf(camera),
        ))
        .id();
    for child in children.iter() {
        if let Ok((mesh, mat, tf)) = parts.get(child) {
            commands.spawn((
                mesh.clone(),
                mat.clone(),
                *tf,
                bevy::light::NotShadowCaster,
                ChildOf(root),
            ));
        }
    }
}

/// Arma, scudo e zaino dell'ultimo aggiornamento.
type WornItems = (Option<ItemKind>, (bool, bool), Option<ItemKind>);

/// Sistema: ricrea i modelli impugnati quando cambia l'equipaggiamento.
#[allow(clippy::too_many_arguments)]
pub fn update_held_items(
    mut commands: Commands,
    equip: Res<Equipment>,
    visuals: Query<Entity, With<PlayerVisual>>,
    held: Query<Entity, With<HeldItem>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    pbr: Res<crate::pbr::PbrMaterials>,
    mut last: Local<Option<WornItems>>,
) {
    let state = (equip.weapon, (equip.shield, equip.rim), equip.backpack);
    let Ok(visual) = visuals.single() else {
        return;
    };
    // Dopo un reset della scena il player è nuovo e senza modelli: si ricostruiscono.
    let missing = held.is_empty() && (state.0.is_some() || equip.shield || state.2.is_some());
    if *last == Some(state) && !missing {
        return;
    }
    *last = Some(state);
    for e in &held {
        commands.entity(e).despawn();
    }
    let mut mat = |c: Color, metal: f32| {
        materials.add(StandardMaterial {
            base_color: c,
            metallic: metal,
            perceptual_roughness: 0.55,
            ..default()
        })
    };
    let steel = mat(Color::srgb(0.78, 0.8, 0.86), 0.9);
    let wood = pbr.get(crate::pbr::Mat::Planks);
    let gold = mat(Color::srgb(0.85, 0.7, 0.25), 0.4);
    if let Some(w) = equip.weapon {
        let mut blade = |len: f32, w: f32| meshes.add(Cuboid::new(w, len, 0.025));
        let root = commands
            .spawn((
                HeldItem,
                HeldWeapon,
                crate::figure::AttachToHand { right: true },
                Transform::from_translation(GRIP).with_rotation(Quat::from_rotation_x(REST_TILT)),
                Visibility::default(),
            ))
            .id();
        commands.entity(visual).add_child(root);
        let add = |c: &mut Commands,
                   mesh: Handle<Mesh>,
                   m: &Handle<StandardMaterial>,
                   at: Vec3,
                   rot: Quat| {
            let child = c
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(m.clone()),
                    Transform::from_translation(at).with_rotation(rot),
                ))
                .id();
            c.entity(root).add_child(child);
        };
        match w {
            ItemKind::ShortSword | ItemKind::LongSword => {
                let len = if w == ItemKind::LongSword { 1.05 } else { 0.7 };
                add(
                    &mut commands,
                    blade(len, 0.08),
                    &steel,
                    Vec3::new(0.0, 0.18 + len / 2.0, 0.0),
                    Quat::IDENTITY,
                );
                add(
                    &mut commands,
                    meshes.add(Cuboid::new(0.32, 0.05, 0.06)),
                    &gold,
                    Vec3::new(0.0, 0.15, 0.0),
                    Quat::IDENTITY,
                );
                add(
                    &mut commands,
                    meshes.add(Cuboid::new(0.05, 0.22, 0.05)),
                    &wood,
                    Vec3::new(0.0, 0.03, 0.0),
                    Quat::IDENTITY,
                );
            }
            ItemKind::Axe => {
                add(
                    &mut commands,
                    meshes.add(Cuboid::new(0.05, 0.95, 0.05)),
                    &wood,
                    Vec3::new(0.0, 0.35, 0.0),
                    Quat::IDENTITY,
                );
                add(
                    &mut commands,
                    meshes.add(Cuboid::new(0.34, 0.3, 0.05)),
                    &steel,
                    Vec3::new(0.15, 0.72, 0.0),
                    Quat::IDENTITY,
                );
            }
            ItemKind::Bow | ItemKind::MasterBow => {
                add(
                    &mut commands,
                    meshes.add(Cuboid::new(0.05, 0.7, 0.05)),
                    &wood,
                    Vec3::new(0.0, 0.3, 0.0),
                    Quat::IDENTITY,
                );
                for s in [-1.0f32, 1.0] {
                    add(
                        &mut commands,
                        meshes.add(Cuboid::new(0.05, 0.5, 0.05)),
                        &wood,
                        Vec3::new(0.0, 0.3 + s * 0.52, -s.abs() * 0.0 + 0.1),
                        Quat::from_rotation_x(s * 0.45),
                    );
                }
                add(
                    &mut commands,
                    meshes.add(Cuboid::new(0.012, 1.5, 0.012)),
                    &steel,
                    Vec3::new(0.0, 0.3, 0.22),
                    Quat::IDENTITY,
                );
            }
            _ => {}
        }
    }
    if equip.shield {
        // Scudo in legno dipinto: le stesse assi con una tinta blu.
        let shield_mat = {
            let base = pbr.get(crate::pbr::Mat::Planks);
            let tinted = materials.get(&base).cloned();
            materials.add(StandardMaterial {
                // Lo Scudo dell'Orlo è di bronzo: la stessa tavola, tinta calda.
                base_color: if equip.rim {
                    Color::srgb(1.3, 0.85, 0.45)
                } else {
                    Color::srgb(0.55, 0.75, 1.3)
                },
                ..tinted.unwrap_or_default()
            })
        };
        let shield = commands
            .spawn((
                HeldItem,
                Mesh3d(meshes.add(Cuboid::new(0.07, 0.75, 0.55))),
                MeshMaterial3d(shield_mat),
                crate::figure::AttachToHand { right: false },
                // Sul dorso della mano sinistra (osso `hand_l`).
                Transform::from_xyz(-0.08, 0.05, 0.0),
                Visibility::default(),
            ))
            .id();
        let boss = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(0.05, 0.18, 0.18))),
                MeshMaterial3d(gold.clone()),
                Transform::from_xyz(-0.06, 0.0, 0.0),
            ))
            .id();
        commands.entity(shield).add_child(boss);
        commands.entity(visual).add_child(shield);
    }
    if let Some(pack) = equip.backpack {
        // Sulla schiena (+Z del modello): più capiente, più grande.
        let k = pack.backpack_kg().unwrap_or(5.0) / 30.0;
        let size = Vec3::new(0.26 + 0.14 * k, 0.26 + 0.34 * k, 0.12 + 0.14 * k);
        let cloth = pbr.get(crate::pbr::Mat::Cloth);
        let body = commands
            .spawn((
                HeldItem,
                Mesh3d(meshes.add(crate::pbr::with_tangents(Cuboid::new(
                    size.x, size.y, size.z,
                )))),
                MeshMaterial3d(cloth),
                Transform::from_xyz(0.0, 0.32, 0.15 + size.z / 2.0),
                Visibility::default(),
                // Segue il busto in ogni animazione, invece di restare ferma sul corpo (ADR 0059).
                crate::figure::AttachToBack,
            ))
            .id();
        commands.entity(visual).add_child(body);
    }
}

/// La luce della Lanterna di Vetro di Tiro, portata al fianco (QS-02).
#[derive(Component)]
pub struct CarriedLantern;

/// Potenza e portata della lanterna: il doppio di una lanterna da guardia, di notte e in grotta.
const LANTERN_LUMENS: f32 = 160_000.0;
const LANTERN_RANGE: f32 = 24.0;

/// Sistema: la lanterna accesa (dalla barra o dall'inventario) segue il giocatore, all'altezza
/// della cintura; spenta, o se non la si ha più, la luce sparisce.
pub fn carried_lantern(
    mut commands: Commands,
    equip: Res<Equipment>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    mut lights: Query<(Entity, &mut Transform), With<CarriedLantern>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let at = player.translation() + player.right() * 0.35 + Vec3::Y * 0.1;
    if !equip.lantern {
        for (e, _) in &lights {
            commands.entity(e).despawn();
        }
        return;
    }
    if let Some((_, mut tf)) = lights.iter_mut().next() {
        tf.translation = at;
        return;
    }
    commands.spawn((
        crate::scene::SceneNode,
        CarriedLantern,
        Mesh3d(meshes.add(Cylinder::new(0.07, 0.18))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.85, 0.5),
            emissive: LinearRgba::rgb(14.0, 8.0, 2.5),
            ..default()
        })),
        PointLight {
            color: Color::srgb(1.0, 0.82, 0.55),
            intensity: LANTERN_LUMENS,
            range: LANTERN_RANGE,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(at),
        bevy::light::NotShadowCaster,
    ));
    info!("[INVENTORY] lanterna di Tiro accesa");
}
