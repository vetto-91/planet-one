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
    pub prompt: String,
    /// Raggio di interazione (m): per gli NPC è la prossimità, per gli oggetti
    /// (`aim: Some`) la distanza massima dal giocatore.
    pub range: f32,
    /// `Some(raggio)`: l'oggetto si usa solo **guardandolo** col mirino (raggio in
    /// metri attorno al suo punto di interazione) e ha priorità sull'NPC finché
    /// è guardato. `None`: solo prossimità (NPC).
    pub aim: Option<f32>,
    /// Per gli oggetti montati a parete: direzione (locale al genitore) verso il lato da cui si
    /// usano. Serve il giocatore da quel lato e linea di vista libera verso il punto davanti.
    pub use_from: Option<Vec3>,
}

/// Stato dell'NPC rispetto al dialogo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NpcMode {
    /// Al suo posto: si può parlare.
    Ready,
    /// Impegnato nella routine (cammina, accende la stufa…): non interagibile.
    Busy,
    /// Dorme: "[E]" fa solo uno "Zzz…".
    Asleep,
}

/// NPC: entità stazionaria con dialogo.
#[derive(Component)]
pub struct Npc {
    pub name: &'static str,
    /// Stato: pronto a parlare, occupato (routine) o addormentato.
    pub mode: NpcMode,
    /// Righe del dialogo, mostrate in sequenza.
    pub lines: &'static [&'static str],
    /// Chiave dell'assortimento se è un venditore (vedi `shop.rs`).
    pub shop: Option<&'static str>,
}

/// Nemico: patriglia tra waypoint.
#[derive(Component)]
#[require(EnemyMind)]
pub struct Enemy;

/// Stato di allerta: se vede il giocatore smette di pattugliare e lo attacca.
#[derive(Component, Default)]
pub struct EnemyMind {
    /// Sta inseguendo il giocatore.
    pub chase: bool,
    /// Secondi prima della prossima freccia.
    pub shot_cd: f32,
}

/// Velocità del pattugliamento (m/s): a passo, non di corsa.
pub const PATROL_SPEED: f32 = 1.5;
/// Velocità dell'inseguimento (m/s).
pub const CHASE_SPEED: f32 = 4.3;
/// Distanza (m) a cui si fermano per colpire in mischia.
pub const MELEE_STOP: f32 = 1.3;

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
    /// Clip vocali di ogni riga (vuoto = nessuna voce), in parallelo a `lines`.
    pub voices: Vec<Vec<String>>,
    /// Scelta confermata alla chiusura (`None` = nessuna scelta, es. Esc).
    pub result: Option<usize>,
    /// La E che chiude il dialogo non deve riaprirlo nello stesso frame.
    pub block_interact: bool,
    /// NPC con cui si sta parlando (per il congedo se ci si allontana).
    pub speaker_entity: Option<Entity>,
    /// Il dialogo è stato letto/confermato fino in fondo (non chiuso con Esc
    /// né per allontanamento): solo allora la quest avanza.
    pub finished: bool,
    /// Le scelte mostrate sono l'offerta della IA ("Parliamo d'altro…"), non una scelta di quest.
    pub ai_offer: bool,
    /// Azioni dell'offerta finale (stesso ordine di `choices`).
    pub offer_actions: Vec<Offer>,
    /// Assortimento del venditore con cui si parla (se c'è).
    pub shop: Option<&'static str>,
}

/// Azione dell'offerta mostrata a fine dialogo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Offer {
    Shop,
    Chat,
    /// Passaggio in barca verso l'altra regione (Gaspare).
    Passage,
    Bye,
}

/// Chi porta in barca fra l'Isola e il Continente (ADR 0033).
pub const FERRYMAN: &str = "Gaspare";

/// Dove porta il traghettatore partendo dalla regione corrente, e come lo si chiede.
pub fn passage() -> (crate::region::Region, &'static str) {
    match crate::region::current() {
        crate::region::Region::Isola => (
            crate::region::Region::Nord,
            "Portami al Continente del Nord",
        ),
        crate::region::Region::Nord => (crate::region::Region::Isola, "Riportami a Fontevivo"),
    }
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
    dialog.ai_offer = false;
}

/// Spawn dell'NPC (Viandante) come figlio di `parent` (la casetta), alla
/// posizione locale `local`. Il fronte (-Z) segue quello del genitore. Non è
/// `SceneNode`: il despawn ricorsivo del genitore lo elimina.
pub fn spawn_npc(
    commands: &mut Commands,
    asset_server: &AssetServer,
    parent: Entity,
    local: Vec3,
) -> Entity {
    let npc = commands
        .spawn((
            Npc {
                name: "Viandante",
                mode: NpcMode::Ready,
                shop: Some("viandante"),
                lines: &[
                    "Salve, viaggiatore! Benvenuto su Planet One.",
                    "Le colline qui intorno sono piene di sorprese... e di nemici.",
                    "Prova a saltare con Space mentre corri: il coyote time ti aiuta!",
                    "Se premi Esc trovi il menu. Buona fortuna!",
                ],
            },
            Interactable {
                prompt: "[E] Parla".to_string(),
                // Solo in prossimità del Viandante (porta/stufa si usano col mirino).
                range: 3.0,
                // Si parla solo guardandolo col mirino.
                aim: Some(0.5),
                use_from: None,
            },
            crate::hud::NameAnchor(1.3),
            crate::hut::Keeper::new(),
            // Collider figlio: si somma al corpo statico della casetta.
            Collider::capsule(0.35, 1.0),
            Transform::from_translation(local),
            Visibility::default(),
        ))
        .with_children(|c| crate::figure::spawn_character(c, asset_server, "viandante"))
        .id();
    commands.entity(parent).add_child(npc);
    npc
}

/// Spawn del nemico con patrol tra due waypoint.
pub fn spawn_enemy(
    commands: &mut Commands,
    asset_server: &AssetServer,
    start: Vec3,
    end: Vec3,
) -> Entity {
    commands
        .spawn((
            SceneNode,
            Enemy,
            EnemyHealth::default(),
            PatrolState {
                waypoints: vec![start, end],
                current: 0,
                speed: PATROL_SPEED,
            },
            RigidBody::Kinematic,
            Collider::capsule(0.4, 0.8),
            LockedAxes::ROTATION_LOCKED,
            LinearVelocity::ZERO,
            CustomPositionIntegration,
            crate::figure::ModelName("ladro"),
            crate::hud::NameAnchor(1.1),
            Transform::from_translation(start),
            Visibility::default(),
        ))
        .with_children(|c| crate::figure::spawn_character(c, asset_server, "ladro"))
        .id()
}

/// Interagibile più vicino: (distanza, entità, componente, NPC, proprietà).
type BestInteract<'a> = (
    f32,
    Entity,
    &'a Interactable,
    Option<&'a Npc>,
    Option<&'a crate::hut::PropKind>,
);

/// Sistema di interazione: trova l'`Interactable` più vicino al player entro
/// range e, se il tasto E è premuto, apre il dialogo.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    players: Query<&GlobalTransform, With<Player>>,
    interactables: Query<
        (
            Entity,
            &GlobalTransform,
            &Interactable,
            Option<&Npc>,
            Option<&crate::hut::PropKind>,
        ),
        Without<Player>,
    >,
    mut toggles: MessageWriter<crate::hut::PropToggle>,
    mut barks: ResMut<crate::quests::Barks>,
    mut dialog: ResMut<DialogState>,
    mut nearest: ResMut<NearestInteract>,
    quest: Res<crate::quests::denaro_rubato::QuestState>,
    texts: Res<crate::quests::QuestTexts>,
    mut missions: ResMut<crate::quests::missions::Missions>,
    clock: Res<crate::daynight::GameClock>,
    character: Res<crate::character::Character>,
    inventory: Res<crate::inventory::Inventory>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    spatial: SpatialQuery,
    blockers: Query<(), With<crate::camera::BlocksCamera>>,
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

    // Linea di vista libera da muri (`BlocksCamera`) tra due punti.
    let clear = |from: Vec3, to: Vec3| -> bool {
        let v = to - from;
        let dist = v.length();
        let Ok(d) = Dir3::new(v) else {
            return true;
        };
        spatial
            .cast_ray_predicate(
                from,
                d,
                dist,
                true,
                &SpatialQueryFilter::default(),
                &|ent| blockers.contains(ent),
            )
            .is_none_or(|hit| hit.distance >= dist - 0.08)
    };
    // Occhi: 0,72 m sopra il centro della capsula (come la prima persona).
    let eye = p + Vec3::Y * 0.72;

    // 1) Oggetti (porta, stufa, interruttore…): solo se GUARDATI col mirino
    //    (centro schermo), entro 3 m dal giocatore e non dietro un muro.
    //    Hanno priorità su tutto finché sono guardati.
    let mut best: Option<BestInteract> = None;
    if let Ok(cam) = cameras.single() {
        let origin = cam.translation();
        let dir = cam.forward();
        let mut best_off = f32::MAX;
        for (e, t, inter, npc, prop) in &interactables {
            let Some(radius) = inter.aim else {
                continue;
            };
            // L'NPC impegnato nella routine non è interagibile.
            if npc.is_some_and(|n| n.mode == NpcMode::Busy) {
                continue;
            }
            let pos = t.translation();
            let to = pos - origin;
            let along = to.dot(*dir);
            if along <= 0.0 || pos.distance(p) > inter.range {
                continue;
            }
            let off = (pos - (origin + *dir * along)).length();
            if off > radius || off >= best_off {
                continue;
            }
            // Visibilità: niente uso attraverso i muri.
            let visible = if let Some(local_n) = inter.use_from {
                // Oggetto a parete: giocatore dal lato giusto e linea libera verso il punto
                // davanti al collider (i muri spessi 20 cm inglobano il pulsante).
                let n = (t.compute_transform().rotation * local_n).normalize_or_zero();
                (eye - pos).dot(n) > 0.0 && clear(eye, pos + n * 0.3)
            } else if npc.is_some() {
                // NPC: basta vederne testa, petto o piedi (anche da una finestra).
                [0.4, 1.0, 1.5]
                    .iter()
                    .any(|h| clear(eye, pos + Vec3::Y * (*h - 0.85)))
            } else {
                clear(origin, pos) && clear(eye, pos)
            };
            if !visible {
                continue;
            }
            best_off = off;
            best = Some((off, e, inter, npc, prop));
        }
    }

    // Aggiorna il prompt mostrato dalla UI per l'interagibile piu' vicino.
    nearest.prompt = best.as_ref().map(|b| b.2.prompt.clone());
    // (log debug rimosso)

    // Porta e stufa: E commuta lo stato (gestito da `hut::handle_prop_toggle`).
    if let Some((_, entity, _, None, Some(_))) = best
        && keys.just_pressed(KeyCode::KeyE)
    {
        toggles.write(crate::hut::PropToggle(entity));
        return;
    }
    // L'NPC addormentato non parla: solo un "Zzz…" sopra la testa.
    if let Some((_, entity, _, Some(npc), _)) = best
        && npc.mode == NpcMode::Asleep
        && keys.just_pressed(KeyCode::KeyE)
    {
        barks.say(entity, npc.name, "Zzz…".to_string());
        return;
    }
    if let Some((_, entity, _, Some(npc), _)) = best
        && keys.just_pressed(KeyCode::KeyE)
    {
        dialog.open = true;
        dialog.speaker = npc.name.to_string();
        // Battute e scelte dalla quest (file dialoghi.txt), altrimenti default dell'NPC.
        // Solo il Viandante parla di quest; gli altri abitanti usano il loro saluto.
        // Una tappa di missione che chiede di parlare con lui ha la precedenza (il Viandante
        // solo quando «Denaro Rubato» non ha niente in sospeso da dirgli).
        let facts = crate::quests::missions::Facts {
            pos: p,
            hour: clock.hour,
            quest_accepted: quest.accepted,
            quest_done: quest.stage == crate::quests::denaro_rubato::QuestStage::Completed,
            items: inventory.slots.iter().map(|s| (s.kind, s.qty)).collect(),
            enemies: Vec::new(),
        };
        let busy_keeper = npc.name == "Viandante" && quest.accepted && !facts.quest_done;
        let mission = if busy_keeper {
            None
        } else {
            missions.dialog_for(npc.name, &facts, &character.name)
        };
        let (lines, choices, voices) = if let Some(lines) = mission {
            let n = lines.len();
            (lines, Vec::new(), vec![Vec::new(); n])
        } else if npc.name == "Viandante" {
            crate::quests::npc_dialog(&quest, &texts)
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };
        if lines.is_empty() {
            dialog.lines = npc.lines.iter().map(|s| s.to_string()).collect();
            dialog.voices = if npc.name == "Viandante" {
                vec![Vec::new(); dialog.lines.len()]
            } else {
                crate::voice::greeting_clips(npc.name, dialog.lines.len())
            };
        } else {
            dialog.lines = lines;
            dialog.voices = voices;
        }
        dialog.index = 0;
        dialog.choices = choices;
        dialog.selected = 0;
        dialog.choosing = false;
        dialog.result = None;
        dialog.finished = false;
        dialog.ai_offer = false;
        dialog.shop = npc.shop;
        dialog.speaker_entity = Some(entity);
        info!("dialogo aperto con {}", npc.name);
    }
}

/// Avanza il dialogo con E o Invio; chiude con Esc.
///
/// Se il dialogo ha delle scelte, dopo l'ultima riga si entra in modalità
/// `choosing`: Su/Giù (o W/S) cambiano la scelta evidenziata, E/Invio la
/// confermano. Esc chiude senza scelta (equivale a rifiutare).
#[allow(clippy::too_many_arguments)]
pub fn advance_dialog(
    keys: Res<ButtonInput<KeyCode>>,
    mut travel: ResMut<crate::save::TravelRequest>,
    missions: Res<crate::quests::missions::Missions>,
    mut dialog: ResMut<DialogState>,
    ai: Res<crate::ai::AiState>,
    settings: Res<crate::audio::AudioSettings>,
    mut chat: ResMut<crate::ai::AiChat>,
    mut shop: ResMut<crate::shop::ShopState>,
    mut next: ResMut<NextState<crate::AppState>>,
    mut voice: ResMut<crate::voice::VoiceQueue>,
    time: Res<Time>,
) {
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
            if dialog.ai_offer {
                // Offerta finale (non una scelta di quest): il dialogo scritto si chiude
                // normalmente (la quest avanza), poi eventualmente si apre negozio o chat.
                let name = dialog.speaker.clone();
                let speaker = dialog.speaker_entity;
                let action = dialog.offer_actions.get(choice).copied();
                let key = dialog.shop;
                close_dialog(&mut dialog, None, true);
                if action == Some(Offer::Bye) {
                    let v = crate::ai::persona::load(&name).voice;
                    let salt = (time.elapsed_secs() * 1000.0) as u32;
                    voice.play_from(
                        vec![crate::voice::farewell_clip(&v, salt)],
                        crate::voice::VoiceKind::Bark,
                        speaker,
                    );
                }
                match (action, key) {
                    (Some(Offer::Shop), Some(k)) => {
                        shop.begin(&name, k);
                        next.set(crate::AppState::Shop);
                    }
                    (Some(Offer::Chat), _) => {
                        chat.begin(&name);
                        next.set(crate::AppState::Chat);
                    }
                    (Some(Offer::Passage), _) => {
                        info!("[VIAGGIO] passaggio in barca con {name}");
                        travel.0 = Some(passage().0);
                    }
                    _ => {}
                }
                return;
            }
            info!("[DIALOG] scelta confermata: {}", choice);
            close_dialog(&mut dialog, Some(choice), true);
        }
        return;
    }

    if confirm {
        dialog.index += 1;
        if dialog.index >= dialog.lines.len() {
            if dialog.choices.is_empty() {
                // Dialogo scritto finito, nessuna scelta di quest in sospeso: si offre di comprare
                // (se è un venditore) e/o di chiacchierare (se la IA locale è attiva e pronta).
                let mut actions = Vec::new();
                if dialog.shop.is_some() {
                    actions.push(Offer::Shop);
                }
                // Il passaggio c'è quando la via del nord è aperta (ultima missione del
                // Prologo); dal continente si torna sempre. Nelle prove le missioni sono ferme.
                if dialog.speaker == FERRYMAN
                    && (!missions.enabled
                        || missions.unlocked("continente")
                        || crate::region::current() != crate::region::Region::Isola)
                {
                    actions.push(Offer::Passage);
                }
                if settings.ai_enabled && ai.status == crate::ai::AiStatus::Ready {
                    actions.push(Offer::Chat);
                }
                if actions.is_empty() {
                    close_dialog(&mut dialog, None, true);
                } else {
                    actions.push(Offer::Bye);
                    dialog.index = dialog.lines.len().saturating_sub(1);
                    dialog.choices = actions
                        .iter()
                        .map(|a| {
                            match a {
                                Offer::Shop => "Vorrei comprare qualcosa",
                                Offer::Chat => "Parliamo d'altro…",
                                Offer::Passage => passage().1,
                                Offer::Bye => "Arrivederci",
                            }
                            .to_string()
                        })
                        .collect();
                    dialog.offer_actions = actions;
                    dialog.ai_offer = true;
                    dialog.choosing = true;
                    dialog.selected = 0;
                }
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
    mut voice: ResMut<crate::voice::VoiceQueue>,
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
        if let Some(c) = texts.denaro_rubato.voice("saluto.1") {
            voice.play_from(
                vec![c.to_string()],
                crate::voice::VoiceKind::Bark,
                Some(entity),
            );
        }
        info!("[DIALOG] giocatore allontanato: dialogo chiuso");
    }
}

/// Patrol del nemico: si muove verso il waypoint corrente con MoveAndSlide.
/// Gira in FixedUpdate come il player.
#[allow(clippy::type_complexity)]
pub fn enemy_patrol(
    move_and_slide: MoveAndSlide,
    time: Res<Time>,
    players: Query<&GlobalTransform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<
        (
            Entity,
            &mut Transform,
            &mut LinearVelocity,
            &Collider,
            &mut PatrolState,
            &EnemyMind,
        ),
        With<Enemy>,
    >,
) {
    let me = players.single().ok().map(|p| p.translation());
    for (entity, mut transform, mut velocity, collider, mut patrol, mind) in &mut enemies {
        let (target, speed) = match (mind.chase, me) {
            (true, Some(p)) => {
                let flat = Vec3::new(
                    p.x - transform.translation.x,
                    0.0,
                    p.z - transform.translation.z,
                );
                if flat.length() < MELEE_STOP {
                    // Davanti al giocatore: resta fermo a colpire, guardandolo.
                    velocity.x = 0.0;
                    velocity.z = 0.0;
                    if let Ok(d) = Dir3::new(flat) {
                        transform.rotation = transform.rotation.slerp(
                            Quat::from_rotation_y((-d.x).atan2(-d.z)),
                            (10.0 * time.delta_secs()).clamp(0.0, 1.0),
                        );
                    }
                    continue;
                }
                (Vec3::new(p.x, transform.translation.y, p.z), CHASE_SPEED)
            }
            _ => (patrol.waypoints[patrol.current], patrol.speed),
        };
        let to_target = target - transform.translation;
        let horizontal = Vec3::new(to_target.x, 0.0, to_target.z);

        // Se siamo vicini al waypoint, passa al successivo.
        if !mind.chase && horizontal.length() < 0.5 {
            patrol.current = (patrol.current + 1) % patrol.waypoints.len();
            velocity.0 = Vec3::ZERO;
            continue;
        }

        // Velocità verso il waypoint + gravità per restare sul terreno.
        let dir = horizontal.normalize();
        velocity.x = dir.x * speed;
        velocity.z = dir.z * speed;
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
