//! La traversata fra l'isola e il continente (ADR 0054).
//!
//! Le due vie del Prologo arrivano tutte e due all'Approdo delle Brume (`Trama-Planet-One/10`, §1):
//! - **la via del mare**: la barca di Gaspare attracca al molo (`Region::arrival`);
//! - **la via delle profondità**: dalla Porta Sommersa, oltre i gradini che scendono nell'acqua, una via
//!   sotto il mare sbuca in un'antica **fonte di pietra a gradoni tra le radici di una quercia
//!   gigante**, a una trentina di metri dal porticciolo.
//!
//! La via sotto il mare si percorre nei due sensi: in fondo ai gradini sull'isola, e nella fonte sul
//! continente, `E` comincia la traversata (una scena raccontata). Si può prendere solo dopo la
//! partenza del Prologo (`continente`), e serve la Porta Sommersa aperta.

use crate::quests::missions::Missions;
use crate::region::Region;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// La fonte a gradoni sul continente (centro), a una trentina di metri dal porto.
pub const NORD_SPRING: Vec2 = Vec2::new(26.0, 444.0);
/// Dove si sbuca dalla fonte: sul bordo, verso il porto.
const NORD_SPRING_EXIT: Vec3 = Vec3::new(22.6, 4.0, 446.0);
/// Dove si sbuca sull'isola tornando dalla via sotto il mare: nella nicchia, davanti alla Porta
/// Sommersa (coordinate del mondo, grotta a 90,5; 18 ruotata di mezzo giro).
const ISLAND_DEEP_EXIT: Vec3 = Vec3::new(85.2, 1.8, 43.0);
/// Fin dove si tocca l'acqua della fonte.
const REACH: f32 = 3.2;
/// Fin dove si prende la via in fondo ai gradini oltre la Porta Sommersa.
const ISLAND_REACH: f32 = 1.8;

/// Il prossimo arrivo è dalla via delle profondità? (lo dice la scena, lo legge l'arrivo)
static DEEP: AtomicBool = AtomicBool::new(false);

/// La scena del viaggio sceglie la via: `true` = la via sotto il mare.
pub fn set_deep(deep: bool) {
    DEEP.store(deep, Ordering::Relaxed);
}

/// Dove si arriva in `to`: dalla via sotto il mare la fonte (o la nicchia sull'isola), altrimenti il molo.
/// Legge e dimentica la via scelta.
pub fn arrival(to: Region) -> (Vec3, f32) {
    let deep = DEEP.swap(false, Ordering::Relaxed);
    match (to, deep) {
        // Dalla fonte si guarda il porto, a sud-ovest.
        (Region::Nord, true) => (NORD_SPRING_EXIT, 2.3),
        // Nella nicchia si guarda verso il cunicolo, lontano dalla porta.
        (Region::Isola, true) => (ISLAND_DEEP_EXIT, 0.0),
        (region, false) => region.arrival(),
    }
}

/// Le righe della traversata di ritorno sull'isola dalla via sotto il mare.
pub fn return_lines() -> Vec<String> {
    [
        "Scendi i gradini della fonte. Sotto le radici della quercia l'acqua si apre su una via lastricata.",
        "Il filo di rame corre lungo la parete, e stavolta lo segui verso sud.",
        "Ore di buio e di passi. Poi, davanti, la lastra di pietra e rame abbassata nel pavimento.",
        "L'isola di Fontevivo.",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

/// La fonte a gradoni e la quercia (sul continente).
#[derive(Component)]
pub struct Spring;

/// Sistema: sul continente, costruisce la fonte di pietra a gradoni tra le radici della quercia gigante.
pub fn build_spring(
    mut commands: Commands,
    springs: Query<(), With<Spring>>,
    field: Option<Res<crate::terrain::TerrainField>>,
    pbr: Option<Res<crate::pbr::PbrMaterials>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if crate::region::current() != Region::Nord || !springs.is_empty() {
        return;
    }
    let (Some(field), Some(pbr)) = (field, pbr) else {
        return;
    };
    let ground = field.height_at(NORD_SPRING.x, NORD_SPRING.y);
    let stone = pbr.get(crate::pbr::Mat::Stone);
    let bark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.22, 0.15),
        perceptual_roughness: 0.95,
        ..default()
    });
    let leaves = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.32, 0.14),
        perceptual_roughness: 0.9,
        ..default()
    });
    let water = materials.add(StandardMaterial {
        base_color: Color::srgba(0.12, 0.3, 0.32, 0.85),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        ..default()
    });
    commands
        .spawn((
            Spring,
            crate::scene::SceneNode,
            Transform::from_xyz(NORD_SPRING.x, ground, NORD_SPRING.y),
            Visibility::default(),
            RigidBody::Static,
        ))
        .with_children(|c| {
            // Quattro gradoni quadrati che scendono verso l'acqua: ognuno è un anello di pietra.
            for k in 0..4 {
                let outer = 3.4 - 0.7 * k as f32;
                let inner = outer - 0.7;
                let top = 0.25 - 0.3 * k as f32;
                for (cx, cz, sx, sz) in [
                    (0.0, (outer + inner) / 2.0, outer * 2.0, 0.7),
                    (0.0, -(outer + inner) / 2.0, outer * 2.0, 0.7),
                    ((outer + inner) / 2.0, 0.0, 0.7, inner * 2.0),
                    (-(outer + inner) / 2.0, 0.0, 0.7, inner * 2.0),
                ] {
                    let h = top + 1.2;
                    c.spawn((
                        Mesh3d(meshes.add(Cuboid::new(sx, h, sz))),
                        MeshMaterial3d(stone.clone()),
                        Transform::from_xyz(cx, top - h / 2.0, cz),
                        Collider::cuboid(sx, h, sz),
                    ));
                }
            }
            // L'acqua ferma in fondo.
            c.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.4, 0.04, 1.4))),
                MeshMaterial3d(water),
                Transform::from_xyz(0.0, -0.75, 0.0),
            ));
            // La quercia gigante, dietro la fonte (a nord): tronco, radici che abbracciano i gradoni,
            // e una chioma larga.
            let trunk_at = Vec3::new(0.0, 0.0, -5.0);
            c.spawn((
                Mesh3d(meshes.add(Cylinder::new(1.1, 9.0))),
                MeshMaterial3d(bark.clone()),
                Transform::from_translation(trunk_at + Vec3::Y * 4.5),
                Collider::cylinder(1.1, 9.0),
            ));
            for k in 0..7 {
                let a = -0.6 + k as f32 * 0.55;
                let dir = Vec3::new(a.sin(), 0.0, a.cos());
                c.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.45, 0.4, 4.2))),
                    MeshMaterial3d(bark.clone()),
                    Transform::from_translation(trunk_at + dir * 2.4 + Vec3::Y * 0.15)
                        .with_rotation(Quat::from_rotation_y(a) * Quat::from_rotation_x(0.12)),
                ));
            }
            for (x, y, z, r) in [
                (0.0, 10.5, -5.0, 4.5),
                (3.2, 9.6, -3.5, 3.2),
                (-3.4, 9.8, -4.0, 3.4),
                (0.5, 9.2, -1.6, 3.0),
                (-1.0, 11.5, -7.0, 3.2),
            ] {
                c.spawn((
                    Mesh3d(meshes.add(Sphere::new(r))),
                    MeshMaterial3d(leaves.clone()),
                    Transform::from_xyz(x, y, z).with_scale(Vec3::new(1.0, 0.7, 1.0)),
                ));
            }
        });
    info!("[TRAVERSATA] la fonte a gradoni sotto la quercia");
}

/// Sistema: la via sotto il mare si prende con `E`, in fondo ai gradini oltre la Porta Sommersa
/// (sull'isola) o nell'acqua della fonte (sul continente). Solo dopo la partenza del Prologo.
#[allow(clippy::too_many_arguments)]
pub fn undersea_way(
    keys: Res<ButtonInput<KeyCode>>,
    missions: Res<Missions>,
    dialog: Res<crate::entities::DialogState>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    caves: Query<&GlobalTransform, With<crate::places::CaveMouth>>,
    mut cutscene: ResMut<crate::cutscene::Cutscene>,
    karma: Res<crate::karma::Karma>,
    mut nearest: ResMut<crate::entities::NearestInteract>,
) {
    if dialog.open || cutscene.active() {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let me = player.translation();
    let open = missions.unlocked("continente") && missions.unlocked("porta-sommersa");
    let (here, prompt) = match crate::region::current() {
        Region::Isola => {
            let Some(cave) = caves.iter().next() else {
                return;
            };
            let deep = cave.transform_point(crate::places::SUNKEN_DEEP);
            // Solo giù sui gradini, nell'acqua: non accanto al baule della nicchia.
            let near = me.xz().distance(deep.xz()) <= ISLAND_REACH && me.y < deep.y + 1.6;
            (near, "[E] Prendi la via sotto il mare, verso il continente")
        }
        Region::Nord => (
            me.xz().distance(NORD_SPRING) <= REACH,
            "[E] Scendi nella fonte: la via sotto il mare, verso l'isola",
        ),
    };
    if !here || !open {
        return;
    }
    nearest.prompt = Some(match nearest.prompt.take() {
        Some(p) => format!("{p} · {prompt}"),
        None => prompt.to_string(),
    });
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    match crate::region::current() {
        Region::Isola => cutscene.start_travel(Region::Nord, "profonda", &karma, false),
        Region::Nord => cutscene.start_journey(Region::Isola, return_lines(), true),
    }
    info!("[TRAVERSATA] la via sotto il mare");
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (build_spring, undersea_way.after(crate::entities::interact))
            .run_if(in_state(crate::AppState::Playing)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_way_chooses_where_you_arrive_once() {
        set_deep(true);
        assert_eq!(arrival(Region::Nord).0, NORD_SPRING_EXIT);
        // Letta una volta, la via si dimentica: il prossimo viaggio arriva al molo.
        assert_eq!(arrival(Region::Nord), Region::Nord.arrival());
        set_deep(true);
        assert_eq!(arrival(Region::Isola).0, ISLAND_DEEP_EXIT);
        // La fonte è a una trentina di metri dal porto.
        let port = Region::Nord.arrival().0.xz();
        let d = port.distance(NORD_SPRING);
        assert!((20.0..40.0).contains(&d), "{d}");
    }
}
