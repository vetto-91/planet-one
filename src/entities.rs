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
//! Dialogo: pannello in basso (stile RPG), `E`/`Invio` per avanzare,
//! `Esc` per chiudere. Il movimento resta libero durante il dialogo.

use crate::combat::EnemyHealth;
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
    /// Scelte mostrate dopo l'ultima riga (vuoto = nessuna scelta).
    pub choices: Vec<String>,
    /// Scelta evidenziata (indice in `choices`).
    pub selected: usize,
    /// `true` quando si mostra la domanda con le scelte (dopo l'ultima riga).
    pub choosing: bool,
    /// Scelta confermata alla chiusura (`None` = nessuna scelta, es. Esc).
    pub result: Option<usize>,
    /// La E che chiude il dialogo non deve riaprirlo nello stesso frame.
    pub block_interact: bool,
    /// NPC con cui si sta parlando (per il congedo se ci si allontana).
    pub speaker_entity: Option<Entity>,
    /// Il dialogo è stato letto/confermato fino in fondo (non chiuso con Esc
    /// né per allontanamento): solo allora la quest avanza.
    pub finished: bool,
}

/// Distanza (m) oltre cui il dialogo si chiude da solo (range interazione: 5 m).
pub const LEAVE_DISTANCE: f32 = 8.0;

/// Chiude il dialogo registrando l'eventuale scelta confermata.
fn close_dialog(dialog: &mut DialogState, result: Option<usize>, finished: bool) {
    dialog.finished = finished;
    dialog.open = false;
    dialog.index = 0;
    dialog.choosing = false;
    dialog.result = result;
    dialog.block_interact = true;
}

/// Spawn dell'NPC (Viandante) come figlio di `parent` (la casetta), alla
/// posizione locale `local`. Il fronte (-Z) segue quello del genitore. Non è
/// `SceneNode`: il despawn ricorsivo del genitore lo elimina.
pub fn spawn_npc(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    parent: Entity,
    local: Vec3,
) {
    let mesh = meshes.add(crate::figure::figure_mesh(
        Vec3::new(0.7, 1.6, 0.7),
        &crate::figure::FigureUv::standard(),
    ));
    let mat = materials.add(crate::figure::skin_material(
        asset_server.load("textures/viandante.png"),
    ));

    let npc = commands
        .spawn((
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
            // Collider figlio: si somma al corpo statico della casetta.
            Collider::capsule(0.35, 1.0),
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(local),
        ))
        .id();
    commands.entity(parent).add_child(npc);
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
        EnemyHealth::default(),
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
    interactables: Query<(Entity, &GlobalTransform, &Interactable, Option<&Npc>), Without<Player>>,
    mut dialog: ResMut<DialogState>,
    mut nearest: ResMut<NearestInteract>,
    quest: Res<crate::quests::denaro_rubato::QuestState>,
    texts: Res<crate::quests::QuestTexts>,
) {
    // Se un dialogo è già aperto non riaprire.
    if dialog.open {
        nearest.prompt = None;
        return;
    }
    // La E che ha appena chiuso il dialogo non deve riaprirlo.
    if dialog.block_interact {
        dialog.block_interact = false;
        return;
    }

    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();

    // Trova l'interagibile più vicino entro range.
    let mut best: Option<(f32, Entity, &Interactable, Option<&Npc>)> = None;
    for (e, t, inter, npc) in &interactables {
        let dist = t.translation().distance(p);
        if dist <= inter.range && best.as_ref().is_none_or(|(d, _, _, _)| dist < *d) {
            best = Some((dist, e, inter, npc));
        }
    }

    // Aggiorna il prompt mostrato dalla UI per l'interagibile piu' vicino.
    nearest.prompt = best
        .as_ref()
        .map(|(_, _, inter, _)| inter.prompt.to_string());
    // (log debug rimosso)

    if let Some((_, entity, _, Some(npc))) = best
        && keys.just_pressed(KeyCode::KeyE)
    {
        dialog.open = true;
        dialog.speaker = npc.name.to_string();
        // Battute e scelte dalla quest (file dialoghi.txt), altrimenti default dell'NPC.
        let (lines, choices) = crate::quests::npc_dialog(&quest, &texts);
        dialog.lines = if lines.is_empty() {
            npc.lines.iter().map(|s| s.to_string()).collect()
        } else {
            lines
        };
        dialog.index = 0;
        dialog.choices = choices;
        dialog.selected = 0;
        dialog.choosing = false;
        dialog.result = None;
        dialog.finished = false;
        dialog.speaker_entity = Some(entity);
        info!("dialogo aperto con {}", npc.name);
    }
}

/// Avanza il dialogo con E o Invio; chiude con Esc.
///
/// Se il dialogo ha delle scelte, dopo l'ultima riga si entra in modalità
/// `choosing`: Su/Giù (o W/S) cambiano la scelta evidenziata, E/Invio la
/// confermano. Esc chiude senza scelta (equivale a rifiutare).
pub fn advance_dialog(keys: Res<ButtonInput<KeyCode>>, mut dialog: ResMut<DialogState>) {
    if !dialog.open {
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        close_dialog(&mut dialog, None, false);
        return;
    }

    let confirm = keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::Enter);

    if dialog.choosing {
        let n = dialog.choices.len();
        if n > 0 {
            if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
                dialog.selected = (dialog.selected + n - 1) % n;
            }
            if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
                dialog.selected = (dialog.selected + 1) % n;
            }
        }
        if confirm {
            let choice = dialog.selected;
            info!("[DIALOG] scelta confermata: {}", choice);
            close_dialog(&mut dialog, Some(choice), true);
        }
        return;
    }

    if confirm {
        dialog.index += 1;
        if dialog.index >= dialog.lines.len() {
            if dialog.choices.is_empty() {
                close_dialog(&mut dialog, None, true);
            } else {
                // Resta sull'ultima riga e mostra le scelte.
                dialog.index = dialog.lines.len().saturating_sub(1);
                dialog.choosing = true;
                dialog.selected = 0;
            }
        }
    }
}

/// Se ci si allontana oltre [`LEAVE_DISTANCE`] dall'NPC durante il dialogo, lo
/// chiude come uscendo (nessuna scelta, quest invariata) e l'NPC saluta.
pub fn dialog_walk_away(
    mut dialog: ResMut<DialogState>,
    mut barks: ResMut<crate::quests::Barks>,
    texts: Res<crate::quests::QuestTexts>,
    players: Query<&GlobalTransform, With<Player>>,
    npcs: Query<(&GlobalTransform, &Npc)>,
) {
    if !dialog.open {
        return;
    }
    let (Ok(player), Some(entity)) = (players.single(), dialog.speaker_entity) else {
        return;
    };
    let Ok((npc_t, npc)) = npcs.get(entity) else {
        return;
    };
    if player.translation().distance(npc_t.translation()) > LEAVE_DISTANCE {
        close_dialog(&mut dialog, None, false);
        dialog.block_interact = false;
        barks.say(entity, npc.name, texts.denaro_rubato.text("saluto", &[]));
        info!("[DIALOG] giocatore allontanato: dialogo chiuso");
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

        // Il fronte (-Z) guarda nella direzione di marcia.
        let target = Quat::from_rotation_y((-dir.x).atan2(-dir.z));
        let alpha = (10.0 * time.delta_secs()).clamp(0.0, 1.0);
        transform.rotation = transform.rotation.slerp(target, alpha);
    }
}
