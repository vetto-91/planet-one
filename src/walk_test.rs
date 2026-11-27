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

    /// Dalla porta su per tutte le rampe fino al sottotetto (segnalazione 37: nel forno la testa
    /// sbatteva nella fascia sotto la gronda e non si saliva).
    #[test]
    fn the_attic_can_be_reached_on_foot() {
        let mut bad = Vec::new();
        for k in [
            "forno",
            "casa-piccola",
            "casa-media",
            "casa-grande",
            "arcere",
            "fabbro",
            "locanda",
            "caserma",
        ] {
            // Seme 7, quello della casa del porticciolo (3,2; 90,5: segnalazione 2026-11-18_23-48-25,
            // la scala della soffitta non si saliva) e altri: ogni seme cambia la disposizione.
            let porticciolo = 320u32 ^ 9050u32.rotate_left(11);
            for seed in [7, porticciolo].into_iter().chain(1..20) {
                let int = model(k, seed).interior;
                // Dal piede dell'ultima rampa (le rampe di sotto le prova `every_bed_…`) su fino in
                // cima e fuori dal vano, nel sottotetto. Il primo gradino è un pianerottolo pieno:
                // la capsula della prova non sale gli scalini, si parte già sulla rampa.
                let (c, rot, size) = *int.ramps.last().unwrap();
                let (x, n) = (rot * Vec3::X, rot * Vec3::Y);
                let mid = c + n * (size.y / 2.0);
                let low = mid - x * (size.x / 2.0);
                let route = int.routes.last().unwrap();
                let path = vec![
                    low + Vec3::Y * 0.05,
                    route[route.len() - 2],
                    // Fuori dal vano, proprio dove si fermano gli abitanti: prima qui il muretto del
                    // sottotetto bloccava il giocatore (la prova arretrava di 30 cm e non lo vedeva).
                    route[route.len() - 1],
                ];
                if let Err(e) = can_walk(&int, &path) {
                    bad.push(format!("{k} seme {seed}: {e}"));
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// Faro: dalla porta su per la rampa al primo piano, fino all'ingresso della chiocciola
    /// (segnalazione 45: la chiocciola stava sopra il vano della rampa).
    #[test]
    fn the_lighthouse_ground_flight_leads_to_the_spiral() {
        let int = model("faro", 7).interior;
        let path = int.routes.last().unwrap().clone();
        if let Err(e) = can_walk(&int, &path) {
            panic!("faro: {e}");
        }
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

    #[test]
    fn the_cave_can_be_walked_from_the_entrance_to_the_back() {
        let mut b = crate::buildings::B::default();
        let mut int = Interior::default();
        crate::places::build("grotta", &mut b, &mut int);
        // Dall'ingresso lungo il corridoio, poi nella camera fino al fondo e al falò.
        let path = [
            Vec3::new(0.0, 0.5, 8.2),
            Vec3::new(0.0, 0.5, 5.0),
            Vec3::new(0.0, 0.5, 1.0),
            Vec3::new(0.0, 0.5, -2.0),
            Vec3::new(-2.5, 0.5, -6.5),
            Vec3::new(2.0, 0.5, -7.0),
        ];
        if let Err(e) = can_walk(&int, &path) {
            panic!("grotta: {e}");
        }
        // Le gallerie sotto la collina: discesa, laghetto, sala del tesoro, cunicolo e nicchia.
        for (name, path) in [
            (
                "discesa e sala del tesoro",
                vec![
                    Vec3::new(-2.5, 0.06, -6.0),
                    Vec3::new(-2.5, 0.06, -8.6),
                    Vec3::new(-2.5, -1.2, -12.6),
                    Vec3::new(-2.5, -1.2, -17.0),
                    Vec3::new(-2.5, -1.2, -20.0),
                    Vec3::new(-4.8, -1.2, -22.0),
                ],
            ),
            (
                "laghetto",
                vec![
                    Vec3::new(-2.5, -1.2, -12.6),
                    Vec3::new(-2.5, -1.2, -15.0),
                    Vec3::new(-6.0, -1.2, -15.0),
                    Vec3::new(-7.0, -1.2, -16.2),
                ],
            ),
            (
                "cunicolo e nicchia",
                vec![
                    Vec3::new(-2.5, -1.2, -20.0),
                    Vec3::new(-0.5, -1.2, -20.4),
                    Vec3::new(3.0, -1.2, -20.4),
                    Vec3::new(5.3, -1.2, -21.0),
                    Vec3::new(5.3, -1.2, -25.4),
                ],
            ),
        ] {
            if let Err(e) = can_walk(&int, &path) {
                panic!("grotta, {name}: {e}");
            }
        }
        // Da fuori: dal piede della rampa (a quota del terreno, 6,4 m dall'arco) fino alla camera.
        let outside = [
            Vec3::new(0.0, -2.1, 15.4),
            Vec3::new(0.0, -1.0, 12.0),
            Vec3::new(0.0, 0.1, 8.6),
            Vec3::new(0.0, 0.1, 5.0),
            Vec3::new(0.0, 0.1, -2.0),
        ];
        if let Err(e) = can_walk(&int, &outside) {
            panic!("grotta da fuori: {e}");
        }
    }
}

/// Nemico di prova: cammina verso +Z con `entities::walk_step`.
#[derive(Component)]
struct Climber;

fn climb(
    ms: MoveAndSlide,
    spatial: SpatialQuery,
    time: Res<Time>,
    bodies: Query<&RigidBody>,
    mut q: Query<(Entity, &mut Transform, &Collider, &mut LinearVelocity), With<Climber>>,
) {
    for (e, mut t, c, mut v) in &mut q {
        v.x = 0.0;
        v.z = 2.6;
        let footing = |e: Entity| bodies.get(e).ok().is_none_or(|b| b.is_static());
        let (p, nv) = crate::entities::walk_step(
            &ms,
            &spatial,
            e,
            c,
            t.translation,
            t.rotation,
            v.0,
            time.delta(),
            &footing,
        );
        t.translation = p;
        v.0 = nv;
    }
}

/// Fa salire a un ladro (capsula 0,4 × 0,8) una scalinata di `steps` gradini alti `rise`, profondi
/// 0,8 m; restituisce la quota dei piedi dopo 8 secondi.
pub fn enemy_climbs(rise: f32, steps: usize) -> f32 {
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
    .add_systems(FixedUpdate, climb);
    // Prato largo sotto, poi i gradini pieni fino a terra.
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(20.0, 1.0, 40.0),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));
    for i in 0..steps {
        let h = rise * (i + 1) as f32;
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(4.0, h, 0.8),
            Transform::from_xyz(0.0, h / 2.0, 2.0 + 0.8 * i as f32 + 0.4),
        ));
    }
    // Pianerottolo in cima.
    let top = rise * steps as f32;
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(4.0, top, 30.0),
        Transform::from_xyz(0.0, top / 2.0, 2.0 + 0.8 * steps as f32 + 15.0),
    ));
    let id = app
        .world_mut()
        .spawn((
            RigidBody::Kinematic,
            Collider::capsule(0.4, 0.8),
            LinearVelocity::ZERO,
            CustomPositionIntegration,
            Climber,
            Transform::from_xyz(0.0, 0.8, 0.0),
        ))
        .id();
    app.finish();
    app.cleanup();
    for _ in 0..(8 * 60) {
        app.update();
    }
    app.world().get::<Transform>(id).unwrap().translation.y - 0.8
}

#[cfg(test)]
mod enemy_tests {
    /// I banditi sui gradoni di pietra (segnalazione 2026-11-27_12-40-17): gradini da 40 cm si
    /// salgono, un muro da 1 m no.
    #[test]
    fn enemies_climb_steps_but_not_walls() {
        let top = super::enemy_climbs(0.4, 5);
        assert!((top - 2.0).abs() < 0.1, "piedi a {top:.2}");
        let wall = super::enemy_climbs(1.0, 1);
        assert!(wall < 0.1, "piedi a {wall:.2}");
    }
}
