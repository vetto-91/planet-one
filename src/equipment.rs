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

/// Sistema: in corsa le braccia portano la mano vicino alla testa e l'arma ci finirebbe dentro,
/// quindi viene riposta (invisibile) finché dura la corsa.
pub fn stow_weapon_while_running(
    animators: Query<&crate::animation::Animator, With<PlayerVisual>>,
    mut weapons: Query<&mut Visibility, With<HeldWeapon>>,
) {
    let running = animators.iter().any(|a| a.running());
    let wanted = if running {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut v in &mut weapons {
        if *v != wanted {
            *v = wanted;
        }
    }
}

/// Arma, scudo e zaino dell'ultimo aggiornamento.
type WornItems = (Option<ItemKind>, bool, Option<ItemKind>);

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
    let state = (equip.weapon, equip.shield, equip.backpack);
    let Ok(visual) = visuals.single() else {
        return;
    };
    // Dopo un reset della scena il player è nuovo e senza modelli: si ricostruiscono.
    let missing = held.is_empty() && (state.0.is_some() || state.1 || state.2.is_some());
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
            ItemKind::Bow => {
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
                base_color: Color::srgb(0.55, 0.75, 1.3),
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
            ))
            .id();
        commands.entity(visual).add_child(body);
    }
}
