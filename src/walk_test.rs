//! Prova di camminata: un giocatore vero (capsula cinematica con `MoveAndSlide`, come in `player.rs`)
//! percorre i percorsi degli abitanti dentro gli edifici. Se resta incastrato nelle scale o con la
//! testa nei solai, il test lo dice e indica dove.

use crate::buildings::{Interior, model};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

const SPEED: f32 = 2.4;
const GRAVITY: f32 = 20.0;
const STEP: f32 = 1.0 / 60.0;

#[derive(Component)]
struct Walker {
    path: Vec<Vec3>,
    next: usize,
    vy: f32,
    ticks: u32,
    done: bool,
}

fn walk(
    ms: MoveAndSlide,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &Collider, &mut Walker)>,
) {
    for (e, mut t, c, mut w) in &mut q {
        if w.done {
            continue;
        }
        w.ticks += 1;
        let target = w.path[w.next] + Vec3::Y * 0.9;
        let to = target - t.translation;
        let flat = Vec3::new(to.x, 0.0, to.z);
        if flat.length() < 0.22 && to.y.abs() < 0.7 {
            w.next += 1;
            if w.next == w.path.len() {
                w.done = true;
            }
            continue;
        }
        let dir = flat.normalize_or_zero();
        w.vy -= GRAVITY * STEP;
        let vel = Vec3::new(dir.x * SPEED, w.vy, dir.z * SPEED);
        let out = ms.move_and_slide(
            c,
            t.translation,
            t.rotation,
            vel,
            time.delta(),
            &MoveAndSlideConfig::default(),
            &SpatialQueryFilter::from_excluded_entities([e]),
            |_| MoveAndSlideHitResponse::Accept,
        );
        t.translation = out.position;
        w.vy = if out.projected_velocity.y.abs() < 0.01 {
            -1.0
        } else {
            out.projected_velocity.y.min(0.0)
        };
    }
}

/// Percorre `path` dentro `int`; restituisce `Err(posizione)` se non arriva in tempo.
pub fn can_walk(int: &Interior, path: &[Vec3]) -> Result<(), String> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        PhysicsPlugins::default(),
    ))
    .init_asset::<Mesh>()
    .insert_resource(Time::<Fixed>::from_seconds(STEP as f64))
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs_f32(STEP),
    ))
    .add_systems(FixedUpdate, walk);
    for (c, rot, s) in &int.ramps {
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(s.x, s.y, s.z),
            Transform::from_translation(*c).with_rotation(*rot),
        ));
    }
    for (c, s) in int.ground.iter().chain(&int.walls).chain(&int.furniture) {
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(s.x, s.y, s.z),
            Transform::from_translation(*c),
        ));
    }
    let start = path[0] + Vec3::Y * 0.95;
    let id = app
        .world_mut()
        .spawn((
            RigidBody::Kinematic,
            Collider::capsule(0.35, 1.0),
            Transform::from_translation(start),
            Walker {
                path: path.to_vec(),
                next: 1,
                vy: 0.0,
                ticks: 0,
                done: false,
            },
        ))
        .id();
    app.finish();
    app.cleanup();
    for _ in 0..(60 * 60) {
        app.update();
        if app.world().get::<Walker>(id).is_some_and(|w| w.done) {
            return Ok(());
        }
    }
    let w = app.world().get::<Walker>(id).unwrap();
    let p = app.world().get::<Transform>(id).unwrap().translation;
    let near: Vec<String> = int
        .ground
        .iter()
        .chain(&int.walls)
        .chain(&int.furniture)
        .filter(|(c, sz)| {
            (c.x - p.x).abs() < sz.x / 2.0 + 0.5
                && (c.z - p.z).abs() < sz.z / 2.0 + 0.5
                && (c.y - p.y).abs() < sz.y / 2.0 + 1.3
        })
        .map(|(c, sz)| {
            format!(
                "[{:.2},{:.2},{:.2} | {:.2}x{:.2}x{:.2}]",
                c.x, c.y, c.z, sz.x, sz.y, sz.z
            )
        })
        .collect();
    Err(format!(
        "fermo a ({:.2},{:.2},{:.2}) verso il punto {} {:?}; vicino: {}",
        p.x,
        p.y - 0.85,
        p.z,
        w.next,
        w.path[w.next],
        near.join(" ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(kind: &str) -> Vec<String> {
        let int = model(kind, 7).interior;
        let mut bad = Vec::new();
        for (i, bed) in int.beds.iter().enumerate() {
            let mut path = int.route_to_bed(bed);
            // L'ultimo punto è il fianco del letto: gli abitanti lo raggiungono in linea retta,
            // la prova riguarda le scale.
            path.pop();
            if let Err(e) = can_walk(&int, &path) {
                bad.push(format!("{kind} letto {i}: {e}"));
            }
        }
        bad
    }

    #[test]
    fn every_bed_is_reachable_by_walking_not_only_on_paper() {
        let mut bad = Vec::new();
        for k in [
            "casa-piccola",
            "casa-media",
            "casa-grande",
            "locanda",
            "caserma",
            "fabbro",
            "forno",
            "arcere",
        ] {
            bad.extend(check(k));
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// Percorso lungo le rampe della chiocciola: dall'inizio della prima ai suoi punti d'arrivo.
    fn helix_path(int: &Interior, from_y: f32) -> Vec<Vec3> {
        let ends: Vec<(Vec3, Vec3)> = int
            .ramps
            .iter()
            .map(|(c, rot, s)| {
                let x = *rot * Vec3::X;
                let n = *rot * Vec3::Y;
                let mid = *c + n * (s.y / 2.0);
                (mid - x * (s.x / 2.0), mid + x * (s.x / 2.0))
            })
            .filter(|(a, _)| a.y >= from_y - 0.01)
            .collect();
        // Si parte dalla prima cella: l'ingresso dal pavimento è un solo gradino.
        let mut path: Vec<Vec3> = ends.iter().map(|e| e.1).collect();
        path.dedup_by(|a, b| a.distance(*b) < 0.05);
        path
    }

    #[test]
    fn the_spiral_towers_can_be_climbed_to_the_top() {
        let mut bad = Vec::new();
        for (name, int, from) in [
            ("faro", model("faro", 7).interior, 3.0),
            ("chiesa", model("chiesa", 7).interior, 3.0),
            (
                "torre in rovina",
                {
                    let mut b = crate::buildings::B::default();
                    let mut int = Interior::default();
                    crate::places::build("torre-rovina", &mut b, &mut int);
                    int
                },
                0.0,
            ),
        ] {
            let path = helix_path(&int, from);
            if path.len() < 8 {
                bad.push(format!("{name}: gradini non trovati ({})", path.len()));
                continue;
            }
            if let Err(e) = can_walk(&int, &path) {
                bad.push(format!("{name}: {e} (di {} gradini)", path.len()));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
