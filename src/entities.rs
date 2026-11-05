//! Entità (Fase 3): NPC con dialogo, trigger di interazione e nemico in patrol.
//!
//! Componenti:
//! - [`Interactable`]: marker su entità interagibili (NPC). Mostra il prompt
//!   "[E] Parla" quando il player è nel range.
//! - [`DialogState`]: resource con le righe del dialogo correntemente aperto.
//! - [`Npc`]: NPC stazionario con dialogo lineare.
//! - [`Enemy`] + [`PatrolState`]: nemico kinematico che patriglia tra waypoint,
//!   usando lo stesso `MoveAndSlide` del player (senza input utente).
//!
//! Interazione: tasto `E` quando si è nel range di un `Interactable`.
//! Dialogo: pannello in basso (stile RPG), `Enter`/`Space` per avanzare,
//! `Esc` per chiudere. Il movimento resta libero durante il dialogo.

use crate::player::Player;
use crate::scene::SceneNode;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Marker per entità interagibili (NPC, oggetti, ecc.).
#[derive(Component)]
pub struct Interactable {
    /// Testo del prompt mostrato quando il player è nel range.
    pub prompt: &'static str,
    /// Raggio di interazione (m).
    pub range: f32,
}

/// NPC: entité stazionaria con dialogo.
#[derive(Component)]
pub struct Npc {
    pub name: &'static str,
    /// Righe del dialogo, mostrate in sequenza.
    pub lines: &'static [&'static str],
}

/// Nemico: patriglia tra waypoint.
#[derive(Component)]
pub struct Enemy;

/// Stato del patrol del nemico.
#[derive(Component)]
pub struct PatrolState {
    /// Punti di patrol (posizioni assolute nel mondo).
    pub waypoints: Vec<Vec3>,
    /// Indice del waypoint corrente.
    pub current: usize,
    /// Velocità di patrol (m/s).
    pub speed: f32,
}

/// Resource: prompt dell'interagibile piu' vicino al player (per la UI).
/// Aggiornato da [`interact`]; `None` se nessuno nel range.
#[derive(Resource, Default)]
pub struct NearestInteract {
    pub prompt: Option<String>,
}

/// Resource: dialogo correntemente aperto (None = nessun dialogo).
#[derive(Resource, Default)]
pub struct DialogState {
    pub open: bool,
    pub speaker: String,
    pub lines: Vec<String>,
    pub index: usize,
}

/// Spawn dell'NPC a una posizione data, con dialogo.
pub fn spawn_npc(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
) {
    let mesh = meshes.add(Cuboid::new(0.7, 1.6, 0.7));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.35, 0.95),
        perceptual_roughness: 0.7,
        ..default()
    });

    commands.spawn((
        SceneNode,
        Npc {
            name: "Viandante",
            lines: &[
                "Salve, viaggiatore! Benvenuto su Planet One.",
                "Le colline qui intorno sono piene di sorprese... e di nemici.",
                "Prova a saltare con Space mentre corri: il coyote time ti aiuta!",
                "Se premi Esc trovi il menu. Buona fortuna!",
            ],
        },
        Interactable {
            prompt: "[E] Parla",
            range: 5.0,
        },
        RigidBody::Static,
        Collider::capsule(0.35, 1.0),
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(position),
    ));
}

/// Spawn del nemico con patrol tra due waypoint.
pub fn spawn_enemy(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    start: Vec3,
    end: Vec3,
) {
    let mesh = meshes.add(Cuboid::new(0.8, 1.2, 0.8));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.20, 0.20),
        perceptual_roughness: 0.6,
        ..default()
    });

    commands.spawn((
        SceneNode,
        Enemy,
        PatrolState {
            waypoints: vec![start, end],
            current: 0,
            speed: 2.5,
        },
        RigidBody::Kinematic,
        Collider::capsule(0.4, 0.8),
        LockedAxes::ROTATION_LOCKED,
        LinearVelocity::ZERO,
        CustomPositionIntegration,
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(start),
    ));
}

/// Sistema di interazione: trova l'`Interactable` più vicino al player entro
/// range e, se il tasto E è premuto, apre il dialogo.
pub fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    players: Query<&GlobalTransform, With<Player>>,
    interactables: Query<(&GlobalTransform, &Interactable, Option<&Npc>), Without<Player>>,
    mut dialog: ResMut<DialogState>,
    mut nearest: ResMut<NearestInteract>,
) {
    // Se un dialogo è già aperto non riaprire.
    if dialog.open {
        nearest.prompt = None;
        return;
    }

    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();

    // Trova l'interagibile più vicino entro range.
    let mut best: Option<(f32, &Interactable, Option<&Npc>)> = None;
    for (t, inter, npc) in &interactables {
        let dist = t.translation().distance(p);
        if dist <= inter.range && best.as_ref().is_none_or(|(d, _, _)| dist < *d) {
            best = Some((dist, inter, npc));
        }
    }

    // Aggiorna il prompt mostrato dalla UI per l'interagibile piu' vicino.
    nearest.prompt = best.as_ref().map(|(_, inter, _)| inter.prompt.to_string());
    // (log debug rimosso)

    if let Some((_, _, Some(npc))) = best
        && keys.just_pressed(KeyCode::KeyE)
    {
        dialog.open = true;
        dialog.speaker = npc.name.to_string();
        dialog.lines = npc.lines.iter().map(|s| s.to_string()).collect();
        dialog.index = 0;
        info!("dialogo aperto con {}", npc.name);
    }
}

/// Avanza il dialogo con Enter/Space; chiude con Esc.
pub fn advance_dialog(keys: Res<ButtonInput<KeyCode>>, mut dialog: ResMut<DialogState>) {
    if !dialog.open {
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        dialog.open = false;
        dialog.index = 0;
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        dialog.index += 1;
        if dialog.index >= dialog.lines.len() {
            dialog.open = false;
            dialog.index = 0;
        }
    }
}

/// Patrol del nemico: si muove verso il waypoint corrente con MoveAndSlide.
/// Gira in FixedUpdate come il player.
pub fn enemy_patrol(
    move_and_slide: MoveAndSlide,
    time: Res<Time>,
    mut enemies: Query<
        (
            Entity,
            &mut Transform,
            &mut LinearVelocity,
            &Collider,
            &mut PatrolState,
        ),
        With<Enemy>,
    >,
) {
    for (entity, mut transform, mut velocity, collider, mut patrol) in &mut enemies {
        let target = patrol.waypoints[patrol.current];
        let to_target = target - transform.translation;
        let horizontal = Vec3::new(to_target.x, 0.0, to_target.z);

        // Se siamo vicini al waypoint, passa al successivo.
        if horizontal.length() < 0.5 {
            patrol.current = (patrol.current + 1) % patrol.waypoints.len();
            velocity.0 = Vec3::ZERO;
            continue;
        }

        // Velocità verso il waypoint + gravità per restare sul terreno.
        let dir = horizontal.normalize();
        velocity.x = dir.x * patrol.speed;
        velocity.z = dir.z * patrol.speed;
        velocity.y -= 20.0 * time.delta_secs(); // stessa gravità del player

        let output = move_and_slide.move_and_slide(
            collider,
            transform.translation,
            transform.rotation,
            velocity.0,
            time.delta(),
            &MoveAndSlideConfig::default(),
            &SpatialQueryFilter::from_excluded_entities([entity]),
            |_hit| MoveAndSlideHitResponse::Accept,
        );

        transform.translation = output.position;
        velocity.0 = output.projected_velocity;
    }
}
