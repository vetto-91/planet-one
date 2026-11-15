//! La Sera delle Lanterne (ADR 0048): la sera prima della partenza gli abitanti lasciano le loro
//! faccende e si radunano in piazza attorno al pozzo, e le lanterne si accendono una a una.
//!
//! Vale mentre MP-05 è alla tappa della piazza o a quella dell'alba (dopo i doni), dalle 19:30 alle
//! 23:30. Le guardie restano al loro turno. I posti si cercano attorno al pozzo, lontano da
//! bancarelle, lampioni e muri (`free_spot`), e si danno in ordine di nome.

use crate::quests::missions::Missions;
use crate::villagers::Villager;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Inizio e fine della festa (ore).
pub const START: f32 = 19.5;
pub const END: f32 = 23.5;
/// Lanterne attorno al pozzo, e a che distanza dal centro della piazza.
const LANTERNS: usize = 8;
const LANTERN_RING: f32 = 2.5;
/// Anelli dei posti degli abitanti, e distanza minima fra due di loro.
const RINGS: [f32; 5] = [3.6, 4.6, 5.6, 6.6, 7.6];
const GAP: f32 = 1.15;
/// Secondi fra l'accensione di una lanterna e la successiva.
const LIGHT_EVERY: f32 = 0.8;

/// La festa è in corso?
pub fn active(missions: &Missions, hour: f32) -> bool {
    matches!(missions.step_of("mp05"), Some(1 | 2)) && (START..END).contains(&hour)
}

/// Posti attorno a `center`, anello dopo anello, a passo di circa `GAP` metri: ognuno libero
/// (`free`) e lontano dagli altri. Lo sguardo è verso il centro.
pub fn ring_spots(center: Vec2, wanted: usize, free: impl Fn(Vec2) -> bool) -> Vec<(Vec2, Vec2)> {
    let mut taken: Vec<(Vec2, Vec2)> = Vec::new();
    for (k, r) in RINGS.into_iter().enumerate() {
        let n = ((std::f32::consts::TAU * r) / GAP).floor() as usize;
        // Anelli sfalsati: chi sta dietro guarda fra due teste.
        let offset = k as f32 * 0.5;
        for i in 0..n {
            if taken.len() >= wanted {
                return taken;
            }
            let a = (i as f32 + offset) / n as f32 * std::f32::consts::TAU;
            let p = center + Vec2::from_angle(a) * r;
            if free(p) && taken.iter().all(|(q, _)| q.distance(p) >= GAP) {
                taken.push((p, (center - p).normalize_or(Vec2::Y)));
            }
        }
    }
    taken
}

/// Una lanterna della festa: si accende alla sua volta.
#[derive(Component)]
pub struct FestLantern {
    order: usize,
    lit: bool,
}

/// Stato della festa.
#[derive(Resource, Default)]
pub struct LanternEvening {
    pub on: bool,
    /// Secondi da quando è cominciata (per accendere le lanterne una a una).
    elapsed: f32,
    /// Quanti abitanti hanno un posto in piazza.
    pub gathered: usize,
    /// Chi è stato chiamato alla festa (da rimandare a casa alla fine).
    guests: Vec<Entity>,
}

/// Chi si muove (abitanti e giocatore): non conta come ostacolo per un posto libero.
pub type Movers = Or<(With<crate::entities::Npc>, With<crate::player::Player>)>;

/// Il punto è libero? Niente di solido (bancarelle, lampioni, muri) in un cilindro alto quanto una
/// persona; il suolo e chi si muove non contano. Serve anche a `side_scenes`.
pub fn spot_is_free(
    spatial: &SpatialQuery,
    field: &crate::terrain::TerrainField,
    grounds: &Query<(), With<crate::scene::Ground>>,
    movers: &Query<(), Movers>,
    p: Vec2,
    radius: f32,
) -> bool {
    let y = field.height_at(p.x, p.y);
    spatial
        .shape_intersections(
            &Collider::cylinder(radius, 1.2),
            Vec3::new(p.x, y + 1.0, p.y),
            Quat::IDENTITY,
            &SpatialQueryFilter::default(),
        )
        .into_iter()
        .all(|e| grounds.contains(e) || movers.contains(e))
}

/// Materiali delle lanterne: carta spenta e carta accesa.
struct LanternLook {
    pole: Handle<StandardMaterial>,
    dark: Handle<StandardMaterial>,
    lit: Handle<StandardMaterial>,
    pole_mesh: Handle<Mesh>,
    arm_mesh: Handle<Mesh>,
    paper_mesh: Handle<Mesh>,
}

/// Sistema: comincia e finisce la festa, dà i posti agli abitanti e accende le lanterne.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn lantern_evening(
    mut commands: Commands,
    (time, clock, missions): (Res<Time>, Res<crate::daynight::GameClock>, Res<Missions>),
    info: Res<crate::village::VillageInfo>,
    field: Option<Res<crate::terrain::TerrainField>>,
    spatial: SpatialQuery,
    mut state: ResMut<LanternEvening>,
    mut villagers: Query<(Entity, &mut Villager)>,
    grounds: Query<(), With<crate::scene::Ground>>,
    movers: Query<(), Movers>,
    mut lanterns: Query<(Entity, &mut FestLantern, &Children)>,
    mut lights: Query<&mut PointLight>,
    mut papers: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut look: Local<Option<LanternLook>>,
) {
    let Some(field) = field else {
        return;
    };
    let want =
        crate::region::current() == crate::region::Region::Isola && active(&missions, clock.hour);
    if want && !state.on {
        state.on = true;
        state.elapsed = 0.0;
        let plaza = info.spec.plaza;
        // Libero: niente di solido (bancarelle, lampioni, muri) in un cilindro alto quanto una
        // persona; il suolo e chi si muove non contano.
        let free =
            |p: Vec2, radius: f32| spot_is_free(&spatial, &field, &grounds, &movers, p, radius);
        let look = look.get_or_insert_with(|| LanternLook {
            pole: materials.add(StandardMaterial {
                base_color: Color::srgb(0.32, 0.22, 0.14),
                perceptual_roughness: 0.85,
                ..default()
            }),
            dark: materials.add(StandardMaterial {
                base_color: Color::srgb(0.75, 0.55, 0.32),
                perceptual_roughness: 0.9,
                ..default()
            }),
            lit: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.75, 0.4),
                emissive: LinearRgba::rgb(9.0, 4.6, 1.4),
                ..default()
            }),
            pole_mesh: meshes.add(Cylinder::new(0.05, 3.0)),
            arm_mesh: meshes.add(Cuboid::new(0.04, 0.04, 0.5)),
            paper_mesh: meshes.add(Cylinder::new(0.18, 0.38)),
        });
        let mut placed = 0;
        for i in 0..LANTERNS {
            let a = i as f32 / LANTERNS as f32 * std::f32::consts::TAU + 0.2;
            let p = plaza + Vec2::from_angle(a) * LANTERN_RING;
            if !free(p, 0.25) {
                continue;
            }
            let y = field.height_at(p.x, p.y);
            // Il braccio porge la lanterna verso l'esterno, sopra chi sta attorno.
            let out = (p - plaza).normalize_or(Vec2::Y);
            commands
                .spawn((
                    crate::scene::SceneNode,
                    FestLantern {
                        order: placed,
                        lit: false,
                    },
                    Transform::from_xyz(p.x, y, p.y)
                        .with_rotation(Quat::from_rotation_y(out.x.atan2(out.y))),
                    Visibility::default(),
                ))
                .with_children(|c| {
                    c.spawn((
                        Mesh3d(look.pole_mesh.clone()),
                        MeshMaterial3d(look.pole.clone()),
                        Transform::from_xyz(0.0, 1.5, 0.0),
                    ));
                    c.spawn((
                        Mesh3d(look.arm_mesh.clone()),
                        MeshMaterial3d(look.pole.clone()),
                        Transform::from_xyz(0.0, 2.95, 0.22),
                    ));
                    c.spawn((
                        Mesh3d(look.paper_mesh.clone()),
                        MeshMaterial3d(look.dark.clone()),
                        Transform::from_xyz(0.0, 2.7, 0.42),
                    ));
                    c.spawn((
                        PointLight {
                            color: Color::srgb(1.0, 0.7, 0.4),
                            intensity: 0.0,
                            range: 8.0,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_xyz(0.0, 2.5, 0.42),
                    ));
                });
            placed += 1;
        }
        // Posti degli abitanti, in ordine di nome; le lanterne non sono ancora nella fisica.
        let lantern_spots: Vec<Vec2> = (0..LANTERNS)
            .map(|i| {
                let a = i as f32 / LANTERNS as f32 * std::f32::consts::TAU + 0.2;
                plaza + Vec2::from_angle(a) * LANTERN_RING
            })
            .collect();
        let mut guests: Vec<(Entity, String)> = villagers
            .iter()
            .filter(|(_, v)| v.joins_festival() && v.called().is_none())
            .map(|(e, v)| (e, v.name().to_string()))
            .collect();
        guests.sort_by(|a, b| a.1.cmp(&b.1));
        let spots = ring_spots(plaza, guests.len(), |p| {
            free(p, 0.4) && lantern_spots.iter().all(|l| l.distance(p) > 0.9)
        });
        state.guests.clear();
        for ((e, _), spot) in guests.iter().zip(spots.iter()) {
            if let Ok((_, mut v)) = villagers.get_mut(*e) {
                v.call_to(Some(*spot));
                state.guests.push(*e);
            }
        }
        state.gathered = spots.len().min(guests.len());
        info!(
            "[LANTERNE] comincia la Sera delle Lanterne: {} abitanti in piazza, {placed} lanterne",
            state.gathered
        );
    } else if !want && state.on {
        state.on = false;
        state.gathered = 0;
        // Solo chi era stato chiamato alla festa: gli altri posti della storia restano.
        for e in std::mem::take(&mut state.guests) {
            if let Ok((_, mut v)) = villagers.get_mut(e) {
                v.call_to(None);
            }
        }
        for (e, ..) in &lanterns {
            commands.entity(e).despawn();
        }
        info!("[LANTERNE] la festa è finita: tutti a casa");
    }
    if !state.on {
        return;
    }
    state.elapsed += time.delta_secs();
    let Some(look) = look.as_ref() else {
        return;
    };
    for (_, mut lantern, children) in &mut lanterns {
        if lantern.lit || state.elapsed < (lantern.order as f32 + 1.0) * LIGHT_EVERY {
            continue;
        }
        lantern.lit = true;
        for c in children.iter() {
            if let Ok(mut l) = lights.get_mut(c) {
                l.intensity = 45_000.0;
            }
            if let Ok(mut m) = papers.get_mut(c)
                && m.0 == look.dark
            {
                m.0 = look.lit.clone();
            }
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<LanternEvening>().add_systems(
        Update,
        lantern_evening.run_if(in_state(crate::AppState::Playing)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_ring_the_square_apart_and_look_at_the_well() {
        let c = Vec2::new(-1.2, 10.0);
        // Una bancarella a est: nessuno ci finisce sopra.
        let stall = c + Vec2::new(4.6, 0.0);
        let spots = ring_spots(c, 30, |p| p.distance(stall) > 1.5);
        assert_eq!(spots.len(), 30);
        for (i, (p, look)) in spots.iter().enumerate() {
            assert!(p.distance(stall) > 1.5);
            assert!(p.distance(c) >= 3.5);
            assert!((c - *p).normalize().dot(*look) > 0.99);
            for (q, _) in &spots[i + 1..] {
                assert!(p.distance(*q) >= GAP - 1e-3);
            }
        }
        assert!(ring_spots(c, 5, |_| false).is_empty());
    }
}
