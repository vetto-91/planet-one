//! La Prova dell'Arco (ADR 0043): tre anelli di ferro appesi lungo il ruscello, oltre il ponte.
//!
//! Dal ponte si scocca verso valle. Una freccia che passa dentro l'anello giusto lo fa suonare;
//! servono tre anelli in fila — il vicino, il medio, il lontano — con tre frecce consecutive.
//! Una freccia che non centra l'anello che tocca fa ricominciare da capo. Gli anelli sono
//! sfalsati ai due lati dell'acqua, così ogni freccia ne può attraversare uno solo.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::region::Region;
use crate::skills::{self, Skill};

/// Dove ci si mette a tirare (centro del ponte) e verso dove corre l'acqua.
pub const STAND: Vec2 = Vec2::new(81.0, -8.0);
const DOWNSTREAM: Vec2 = Vec2::new(0.94, 0.342);
/// Distanza dal ponte di ogni anello (m), e altezza del centro sul terreno.
pub const DISTANCES: [f32; 3] = [9.0, 14.0, 19.0];
const HEIGHT: f32 = 2.3;
/// Raggio utile dell'anello: quanto può sgarrare la freccia dal centro.
pub const RADIUS: f32 = 0.55;

/// Scarto laterale di ogni anello (m): sfalsati, così una freccia ne attraversa uno solo.
pub const OFFSETS: [f32; 3] = [-2.6, 2.6, 0.0];

pub fn ring_xz(i: usize) -> Vec2 {
    let along = DOWNSTREAM.normalize();
    let side = Vec2::new(-along.y, along.x);
    STAND + along * DISTANCES[i] + side * OFFSETS[i]
}

/// La freccia, andando da `from` a `to`, passa dentro l'anello di centro `centre` che guarda
/// lungo `normal`?
pub fn through(from: Vec3, to: Vec3, centre: Vec3, normal: Vec3) -> bool {
    let (a, b) = ((from - centre).dot(normal), (to - centre).dot(normal));
    if a == b || a.signum() == b.signum() {
        return false;
    }
    let hit = from.lerp(to, a / (a - b));
    hit.distance(centre) <= RADIUS
}

/// Direzione di tiro che centra un bersaglio compensando la caduta della freccia.
pub fn aim(from: Vec3, target: Vec3, speed: f32, drop: f32) -> Vec3 {
    let t = from.distance(target) / speed;
    (target + Vec3::Y * 0.5 * drop * t * t - from).normalize_or_zero()
}

/// Stato della prova.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trial {
    /// Anelli centrati di fila (0..=3).
    pub hits: u8,
    /// La prova è stata superata almeno una volta.
    pub passed: bool,
    /// Tiro chiesto dalla shell: una freccia mirata all'anello (per le prove automatiche).
    pub request: Option<usize>,
}

impl Trial {
    /// Una freccia ha centrato l'anello `ring`: rende `true` se era quello giusto.
    pub fn ring(&mut self, ring: usize) -> bool {
        if ring != usize::from(self.hits) {
            return false;
        }
        self.hits += 1;
        if self.hits >= 3 {
            self.passed = true;
        }
        true
    }

    /// Una freccia è andata a vuoto: si ricomincia (a prova finita non c'è nulla da perdere).
    pub fn miss(&mut self) -> bool {
        let lost = self.hits > 0 && self.hits < 3;
        if self.hits < 3 {
            self.hits = 0;
        }
        lost
    }
}

/// Anello appeso (indice 0..3) e il suo centro nel mondo.
#[derive(Component)]
pub struct Ring(pub usize, pub Vec3);

/// Sistema: pali e anelli lungo il ruscello.
pub fn spawn_rings(
    mut commands: Commands,
    field: Option<Res<crate::terrain::TerrainField>>,
    pbr: Res<crate::pbr::PbrMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    rings: Query<(), With<Ring>>,
) {
    let Some(field) = field else {
        return;
    };
    if crate::region::current() != Region::Isola || !rings.is_empty() {
        return;
    }
    let iron = pbr.get(crate::pbr::Mat::Iron);
    let wood = pbr.get(crate::pbr::Mat::Planks);
    let n = Vec3::new(DOWNSTREAM.x, 0.0, DOWNSTREAM.y).normalize();
    let side = Vec3::Y.cross(n);
    for i in 0..3 {
        let p = ring_xz(i);
        let ground = field.height_at(p.x, p.y).max(crate::terrain::SEA_LEVEL);
        let centre = Vec3::new(p.x, ground + HEIGHT, p.y);
        // Palo a lato dell'acqua con un braccio da cui pende l'anello.
        let post = centre + side * 1.6;
        let post_h = HEIGHT + 1.1;
        commands.spawn((
            crate::scene::SceneNode,
            Mesh3d(meshes.add(crate::pbr::with_tangents(Cuboid::new(0.14, post_h, 0.14)))),
            MeshMaterial3d(wood.clone()),
            Transform::from_translation(Vec3::new(post.x, ground + post_h / 2.0 - 0.2, post.z)),
        ));
        commands.spawn((
            crate::scene::SceneNode,
            Mesh3d(meshes.add(crate::pbr::with_tangents(Cuboid::new(1.9, 0.1, 0.1)))),
            MeshMaterial3d(wood.clone()),
            Transform::from_translation(centre + side * 0.75 + Vec3::Y * (RADIUS + 0.25))
                .with_rotation(Quat::from_rotation_arc(Vec3::X, side)),
        ));
        commands.spawn((
            crate::scene::SceneNode,
            Ring(i, centre),
            Mesh3d(meshes.add(Torus::new(RADIUS, RADIUS + 0.09))),
            MeshMaterial3d(iron.clone()),
            Transform::from_translation(centre).with_rotation(Quat::from_rotation_arc(Vec3::Y, n)),
        ));
    }
}

/// Sistema: segue le frecce del giocatore, fa suonare gli anelli e tiene il conto della prova.
#[allow(clippy::too_many_arguments)]
pub fn judge(
    mut commands: Commands,
    mut trial: ResMut<Trial>,
    mut session: ResMut<crate::save::Session>,
    arrows: Query<(Entity, &Transform), With<crate::combat::Arrow>>,
    rings: Query<&Ring>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // Posizione al frame prima di ogni freccia in volo, e se ha già centrato un anello.
    mut seen: Local<HashMap<Entity, (Vec3, bool)>>,
) {
    let n = Vec3::new(DOWNSTREAM.x, 0.0, DOWNSTREAM.y).normalize();
    // Tiro mirato chiesto dalla shell.
    if let Some(i) = trial.request.take()
        && let Some(ring) = rings.iter().find(|r| r.0 == i)
        && let Ok(player) = players.single()
    {
        let from = player.translation() + Vec3::Y * 0.4;
        let dir = aim(
            from,
            ring.1,
            crate::combat::ARROW_SPEED,
            crate::combat::ARROW_DROP,
        );
        if let Ok(dir) = Dir3::new(dir) {
            crate::combat::loose(&mut commands, &mut meshes, &mut materials, from, dir, 1.0);
        }
    }
    // Solo vicino agli anelli: altrove le frecce non riguardano la prova.
    if rings.is_empty() {
        seen.clear();
        return;
    }
    let mut alive = Vec::new();
    for (e, tf) in &arrows {
        alive.push(e);
        let now = tf.translation;
        let Some((before, scored)) = seen.get(&e).copied() else {
            seen.insert(e, (now, false));
            continue;
        };
        let mut scored = scored;
        for ring in &rings {
            if !scored && through(before, now, ring.1, n) {
                scored = true;
                crate::sfx::play(crate::sfx::Sfx::Ring);
                if trial.ring(ring.0) {
                    skills::train(Skill::Archery, 8.0);
                    if trial.hits >= 3 {
                        skills::train(Skill::Archery, 30.0);
                        session.notify("Tre anelli in fila: la prova è superata!");
                        info!("[ARCO] prova superata");
                    } else {
                        session.notify(format!("Anello {} di 3", trial.hits));
                        info!("[ARCO] anello {} centrato", trial.hits);
                    }
                } else if trial.miss() {
                    session.notify("Anello sbagliato: si ricomincia dal primo");
                }
            }
        }
        seen.insert(e, (now, scored));
    }
    // Le frecce sparite senza centrare nulla, se passavano dagli anelli, sono tiri a vuoto.
    let gone: Vec<Entity> = seen
        .keys()
        .filter(|e| !alive.contains(e))
        .copied()
        .collect();
    for e in gone {
        if let Some((last, scored)) = seen.remove(&e)
            && !scored
            && last.xz().distance(STAND) < DISTANCES[2] + 12.0
            && trial.miss()
        {
            session.notify("A vuoto: si ricomincia dal primo anello");
            info!("[ARCO] freccia a vuoto");
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Trial>()
        .add_systems(Update, spawn_rings)
        .add_systems(
            Update,
            judge
                .after(crate::combat::update_arrows)
                .run_if(in_state(crate::AppState::Playing)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_rings_in_a_row_pass_the_trial() {
        let mut t = Trial::default();
        assert!(t.ring(0));
        assert!(!t.ring(2), "il lontano prima del medio non vale");
        assert!(t.miss());
        assert_eq!(t.hits, 0);
        assert!(!t.miss(), "a zero non c'è nulla da perdere");
        assert!(t.ring(0) && t.ring(1));
        assert!(!t.passed);
        assert!(t.ring(2));
        assert!(t.passed);
        assert!(!t.miss());
        assert_eq!(t.hits, 3);
    }

    #[test]
    fn an_arrow_counts_only_inside_the_ring() {
        let centre = Vec3::new(0.0, 2.0, 10.0);
        let n = Vec3::Z;
        let shot = |x: f32, y: f32| through(Vec3::new(x, y, 9.5), Vec3::new(x, y, 10.5), centre, n);
        assert!(shot(0.0, 2.0));
        assert!(shot(0.3, 2.3));
        assert!(!shot(0.6, 2.0), "fuori dal ferro");
        assert!(!through(
            Vec3::new(0.0, 2.0, 8.0),
            Vec3::new(0.0, 2.0, 9.0),
            centre,
            n
        ));
    }

    #[test]
    fn the_aim_is_raised_for_the_drop() {
        let from = Vec3::new(0.0, 1.0, 0.0);
        let target = Vec3::new(0.0, 1.0, 19.0);
        let dir = aim(from, target, 38.0, 6.0);
        assert!(dir.y > 0.0 && dir.y < 0.1);
        assert!(dir.z > 0.99);
    }

    #[test]
    fn rings_get_farther_from_the_bridge() {
        assert!(ring_xz(0).distance(STAND) < ring_xz(1).distance(STAND));
        assert!(ring_xz(1).distance(STAND) < ring_xz(2).distance(STAND));
        assert!((ring_xz(2).distance(STAND) - 19.0).abs() < 0.01);
        // Una freccia tirata dritta a un anello passa lontano dal ferro degli altri due.
        for target in 0..3 {
            for other in (0..3).filter(|o| *o != target) {
                let to = ring_xz(target) - STAND;
                let t = DISTANCES[other] / DISTANCES[target];
                let at = STAND + to * t;
                assert!(
                    at.distance(ring_xz(other)) > RADIUS * 2.0,
                    "{target} passa dentro {other}"
                );
            }
        }
    }
}
