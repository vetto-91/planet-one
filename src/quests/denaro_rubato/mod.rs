//! Quest "Denaro Rubato" (Fase 5): storia, stadi, monete, titolo overlay.
//!
//! Flusso:
//! 1. `Idle`: il player parla con il Viandante (NPC blu) → quest accettata,
//!    overlay titolo "Denaro Rubato" per 3 s, stage = `SeekThief`.
//! 2. `SeekThief`: il ladro (enemy rosso) è in un angolo della mappa. Ucciderlo
//!    → stage `LootSpawned`, spawn di 12 monete gialle (`QuestCoin`) al suo posto.
//! 3. `Collecting { collected, total }`: raccolta automatica delle monete entro
//!    2 m. Quando collected == total → stage `Returning`. Le monete NON vanno
//!    in inventario: sono contate solo dalla quest.
//! 4. `Returning`: torna dal Viandante. Alla chiusura del dialogo finale →
//!    stage `Completed`, ricompensa di [`REWARD_COINS`] in inventario, overlay
//!    "Missione completata".
//!
//! Il ladro può morire anche prima di accettare la quest: in quel caso le
//! monete spawnano lo stesso (stage `Collecting` con `accepted = false`) e il
//! Viandante le ritira alla consegna. Con la quest attiva e il ladro vivo, il
//! Viandante indica la direzione del ladro (`thief_hint`).
//!
//! I testi stanno in `assets/quests/denaro_rubato/dialoghi.txt` (modificabile a mano).
//! Nessun salvataggio su file: la quest vive solo in memoria (come da piano).

/// Distanza (m) entro cui il ladro nota il giocatore e lo apostrofa.
pub const SPOT_DISTANCE: f32 = 12.0;

use crate::entities::DialogState;
use crate::inventory::{Inventory, ItemKind};
use crate::player::Player;
use crate::quests::{Barks, Dialogs, ObjState, QuestEntry, QuestStatus};
use crate::scene::SceneNode;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Numero di monete da recuperare.
pub const LOOT_TOTAL: u32 = 12;

/// Ricompensa in monete consegnata dal Viandante a quest completata.
pub const REWARD_COINS: u32 = 5;

/// Fasi della quest.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum QuestStage {
    /// Nessuna quest attiva (prima di parlare con l'NPC).
    #[default]
    Idle,
    /// Quest accettata: vai dal ladro.
    SeekThief,
    /// Ladro ucciso: raccogli le monete.
    Collecting,
    /// Monete raccolte: torna dal Viandante.
    Returning,
    /// Quest completata.
    Completed,
}

/// Stato globale della quest (resource, resettata dal reset scena).
#[derive(Resource)]
pub struct QuestState {
    pub stage: QuestStage,
    pub coins_collected: u32,
    pub coins_total: u32,
    /// Overlay titolo in alto: testo + tempo rimanente (0 = nascosto).
    pub title: Option<(String, f32)>,
    /// Flag: il primo dialogo con l'NPC è stato completato (quest accettata).
    pub accepted: bool,
    /// Flag: dialogo finale completato.
    pub final_done: bool,
    /// Direzione e distanza del ladro (es. "nord-ovest, circa 60 m"),
    /// aggiornata ogni frame; usata dal Viandante quando la quest è attiva.
    pub thief_hint: Option<String>,
    /// Come `thief_hint` ma strutturato (nome direzione, metri arrotondati a 5): serve alle voci.
    pub thief_hint_parts: Option<(&'static str, u32)>,
    /// Posizione in cui il ladro è stato ucciso (scritta da
    /// `combat::handle_enemy_death`, consumata da `quest_on_thief_death`).
    pub thief_killed_at: Option<Vec3>,
    /// Il ladro ha già notato il giocatore (battuta detta una sola volta).
    pub thief_spotted: bool,
    /// Che fine ha fatto il ladro (ADR 0035).
    pub fate: Fate,
    /// Il giocatore ha letto la tavoletta al campo del ladro.
    pub tablet: bool,
    /// Monete antiche che il giocatore ha tenuto per sé alla consegna.
    pub kept: u32,
    /// Il Viandante gli ha affidato la Moneta Madre.
    pub mother_coin: bool,
    /// Il ladro è stato sfidato o colpito: da ora combatte.
    pub thief_provoked: bool,
    /// Il ladro è tranquillo adesso: il giocatore è senz'arma in pugno e non l'ha provocato.
    pub thief_calm: bool,
}

/// Come è finita con il ladro: conta per il seguito della storia.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fate {
    #[default]
    Unknown,
    /// Sconfitto con le armi (la via dell'acciaio).
    Killed,
    /// Lasciato andare dopo aver riavuto le monete (la via della grazia).
    Spared,
    /// Oro diviso a metà in segreto (la via dell'ombra).
    Pact,
}

impl Fate {
    pub fn key(self) -> &'static str {
        match self {
            Fate::Unknown => "ignota",
            Fate::Killed => "sconfitto",
            Fate::Spared => "risparmiato",
            Fate::Pact => "patto",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        [Fate::Unknown, Fate::Killed, Fate::Spared, Fate::Pact]
            .into_iter()
            .find(|f| f.key() == text.trim())
    }
}

/// Monete che il giocatore può tenersi alla consegna, e metà del bottino nel patto.
pub const KEPT_COINS: u32 = 4;
pub const PACT_COINS: u32 = LOOT_TOTAL / 2;
/// Entro questa distanza si può parlare al ladro, e si legge la tavoletta al campo.
const PARLEY_RANGE: f32 = 4.5;
const CAMP: Vec2 = Vec2::new(29.0, -31.0);

impl Default for QuestState {
    fn default() -> Self {
        Self {
            stage: QuestStage::Idle,
            coins_collected: 0,
            coins_total: LOOT_TOTAL,
            title: None,
            accepted: false,
            final_done: false,
            thief_hint: None,
            thief_hint_parts: None,
            thief_killed_at: None,
            thief_spotted: false,
            fate: Fate::Unknown,
            tablet: false,
            kept: 0,
            mother_coin: false,
            thief_provoked: false,
            thief_calm: false,
        }
    }
}

/// Marker sul ladro (enemy rosso) che fa parte della quest.
#[derive(Component)]
pub struct QuestThief;

/// Marker su una moneta della quest (cubo giallo).
#[derive(Component)]
pub struct QuestCoin;

/// Marker: il ladro deve essere appoggiato al terreno (dopo lo spawn o una caduta).
#[derive(Component)]
pub struct NeedsGroundSnap;

/// Metà altezza del collider capsule del ladro (come il player): centro sopra il suolo.
const THIEF_CENTER_OFFSET: f32 = 0.85;

/// Sistema: appoggia il ladro sul terreno con un raycast verso il basso e allinea
/// i waypoint alla stessa quota. Riprova ogni frame finché il collider del
/// terreno non è pronto, e riporta il ladro sul terreno se cade sotto la mappa.
pub fn place_quest_thief(
    mut commands: Commands,
    spatial: SpatialQuery,
    mut thieves: Query<
        (
            Entity,
            &mut Transform,
            &mut crate::entities::PatrolState,
            Has<NeedsGroundSnap>,
        ),
        With<QuestThief>,
    >,
) {
    for (entity, mut transform, mut patrol, pending) in &mut thieves {
        if !pending && transform.translation.y > -10.0 {
            continue;
        }
        let filter = SpatialQueryFilter::from_excluded_entities([entity]);
        // Quota del suolo sotto un punto (raycast da 40 m, verso il basso).
        let ground_at = |p: Vec3| -> Option<f32> {
            spatial
                .cast_ray(Vec3::new(p.x, 40.0, p.z), Dir3::NEG_Y, 100.0, true, &filter)
                .map(|hit| 40.0 - hit.distance)
        };
        let Some(base) = patrol.waypoints.first().copied() else {
            continue;
        };
        // Il collider del terreno potrebbe non essere ancora pronto: riprova.
        let Some(base_ground) = ground_at(base) else {
            continue;
        };
        for wp in patrol.waypoints.iter_mut() {
            wp.y = ground_at(*wp).unwrap_or(base_ground) + THIEF_CENTER_OFFSET;
        }
        transform.translation = patrol.waypoints[0];
        patrol.current = 0;
        commands.entity(entity).remove::<NeedsGroundSnap>();
        info!(
            "[QUEST] ladro appoggiato al terreno a {:?}",
            transform.translation
        );
    }
}

/// Mostra un titolo temporaneo in alto al centro (durata in secondi).
fn show_title(quest: &mut QuestState, text: &str, secs: f32) {
    quest.title = Some((text.to_string(), secs));
}

/// Sistema: quando il dialogo con l'NPC si chiude, avanza la quest.
///
/// - Primo dialogo (stage Idle, non ancora accettata): quest accettata →
///   stage SeekThief + titolo "Denaro Rubato".
/// - Dialogo in stage Returning (finale): stage Completed + titolo.
pub fn quest_on_dialog_close(
    mut quest: ResMut<QuestState>,
    dialog: Res<DialogState>,
    mut inventory: ResMut<Inventory>,
    mut last_open: Local<bool>,
) {
    // Rileva il fronte di discesa: dialog.open true -> false.
    let was_open = *last_open;
    *last_open = dialog.open;
    // Solo se il dialogo è stato letto fino in fondo (non Esc / allontanamento).
    if was_open && !dialog.open && dialog.finished && dialog.speaker == "Viandante" {
        match quest.stage {
            // La quest parte SOLO se il giocatore risponde "Sì" (scelta 0).
            QuestStage::Idle if !quest.accepted && dialog.result == Some(0) => {
                quest.accepted = true;
                quest.stage = QuestStage::SeekThief;
                show_title(&mut quest, "Denaro Rubato", 3.0);
                info!("[QUEST] accettata: Denaro Rubato");
            }
            QuestStage::Returning if !quest.final_done => {
                quest.final_done = true;
                quest.stage = QuestStage::Completed;
                let kept = dialog.result == Some(1) && quest.fate != Fate::Pact;
                if quest.fate == Fate::Pact {
                    // Patto: le sei monete sono già in tasca, nessuna ricompensa.
                    show_title(&mut quest, "Missione chiusa: metà del denaro", 4.0);
                } else if kept {
                    // La via dell'ombra: quattro monete antiche restano al giocatore.
                    quest.kept = KEPT_COINS;
                    inventory.add(ItemKind::Coin, KEPT_COINS);
                    show_title(&mut quest, "Missione chiusa: otto monete su dodici", 4.0);
                } else {
                    inventory.add(ItemKind::Coin, REWARD_COINS);
                    if quest.fate == Fate::Spared {
                        quest.mother_coin = true;
                        show_title(&mut quest, "Il Viandante ti affida la Moneta Madre", 4.0);
                    } else {
                        show_title(&mut quest, "Missione completata: Denaro recuperato", 4.0);
                    }
                }
                info!(
                    "[QUEST] completata: sorte del ladro {}, monete tenute {}, moneta madre {}",
                    quest.fate.key(),
                    quest.kept,
                    quest.mother_coin
                );
            }
            _ => {}
        }
    }
}

/// Sistema: quando il ladro muore (posizione registrata da
/// `combat::handle_enemy_death` in `thief_killed_at`), spawna le monete al suo
/// posto e passa alla fase Collecting. Non dipende dall'ordine dei sistemi:
/// la morte è comunicata tramite `QuestState`, non leggendo l'entità (che nel
/// frattempo potrebbe essere già stata despawnata).
pub fn quest_on_thief_death(
    mut commands: Commands,
    spatial: SpatialQuery,
    mut quest: ResMut<QuestState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(death_pos) = quest.thief_killed_at.take() else {
        return;
    };
    // Il ladro può morire anche prima di accettare la quest: in quel caso le
    // monete spawnano comunque (e il Viandante le ritira alla consegna).
    if matches!(
        quest.stage,
        QuestStage::Collecting | QuestStage::Returning | QuestStage::Completed
    ) {
        return;
    }
    // Appoggia il bottino al terreno sotto il punto di morte (raycast verso il basso).
    let ground_y = spatial
        .cast_ray(
            death_pos + Vec3::Y * 5.0,
            Dir3::NEG_Y,
            30.0,
            true,
            &SpatialQueryFilter::default(),
        )
        .map(|hit| death_pos.y + 5.0 - hit.distance)
        .unwrap_or(death_pos.y - THIEF_CENTER_OFFSET);
    let center = Vec3::new(death_pos.x, ground_y, death_pos.z);

    // Ladro morto prima di parlare col Viandante (o trovato morto): la quest si
    // attiva da sola, con gli obiettivi precedenti già completati.
    if !quest.accepted {
        quest.accepted = true;
        show_title(&mut quest, "Denaro Rubato", 3.0);
        info!("[QUEST] attivata automaticamente: il ladro era già morto");
    }
    quest.stage = QuestStage::Collecting;
    quest.fate = Fate::Killed;
    quest.coins_collected = 0;
    quest.thief_hint = None;
    if quest.title.is_none() {
        show_title(&mut quest, "Ladro sconfitto: raccogli il denaro", 3.0);
    }
    spawn_loot(
        &mut commands,
        &mut meshes,
        &mut materials,
        center,
        LOOT_TOTAL,
    );
    info!(
        "[QUEST] ladro morto in {:?}, stage=Collecting (accepted={})",
        death_pos, quest.accepted
    );
}

/// Spawn di N monete gialle (cubetti) attorno a una posizione (cerchio).
fn spawn_coins(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    gold: Handle<StandardMaterial>,
    spots: &[Vec3],
) {
    for pos in spots {
        commands.spawn((
            SceneNode,
            QuestCoin,
            RigidBody::Static,
            Collider::cuboid(0.35, 0.35, 0.35),
            Mesh3d(meshes.add(Cuboid::new(0.35, 0.35, 0.35))),
            MeshMaterial3d(gold.clone()),
            Transform::from_translation(*pos),
        ));
    }
}

/// Monete della quest nelle posizioni di un salvataggio.
pub fn spawn_coins_at(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spots: &[Vec3],
) {
    let gold = gold_material(materials);
    spawn_coins(commands, meshes, gold, spots);
}

fn gold_material(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.80, 0.15),
        emissive: LinearRgba::new(0.6, 0.45, 0.05, 0.0),
        perceptual_roughness: 0.3,
        metallic: 0.8,
        ..default()
    })
}

pub fn spawn_loot(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    count: u32,
) {
    let gold = gold_material(materials);
    let spots: Vec<Vec3> = (0..count)
        .map(|i| {
            let angle = (i as f32) / (count as f32) * std::f32::consts::TAU;
            let radius = 1.0 + ((i % 3) as f32) * 0.4;
            center + Vec3::new(angle.cos() * radius, 0.2, angle.sin() * radius)
        })
        .collect();
    spawn_coins(commands, meshes, gold, &spots);
    info!("[QUEST] spawnate {} monete", count);
}

/// Sistema: raccolta monete quest entro 2 m dal player → incrementa contatore.
pub fn collect_quest_coins(
    mut commands: Commands,
    players: Query<&GlobalTransform, With<Player>>,
    coins: Query<(Entity, &GlobalTransform), With<QuestCoin>>,
    mut quest: ResMut<QuestState>,
) {
    if quest.stage != QuestStage::Collecting {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();
    for (entity, t) in &coins {
        if t.translation().distance(p) <= 2.0 {
            commands.entity(entity).despawn();
            quest.coins_collected += 1;
            info!(
                "[QUEST] moneta {}/{}",
                quest.coins_collected, quest.coins_total
            );
            if quest.coins_collected >= quest.coins_total {
                quest.stage = QuestStage::Returning;
                show_title(&mut quest, "Bottino completo: torna dal Viandante", 3.0);
            }
        }
    }
}

/// Aggiorna il timer del titolo overlay (decrementa; a 0 lo nasconde).
pub fn tick_title(time: Res<Time>, mut quest: ResMut<QuestState>) {
    if let Some((_, t)) = &mut quest.title {
        *t -= time.delta_secs();
        if *t <= 0.0 {
            quest.title = None;
        }
    }
}

/// Nome cardinale/intercardinale (8 punti) della direzione da `from` verso `to`.
/// Convenzione mondo: N = -Z, E = +X (come in `draw_compass`).
pub fn compass_name(from: Vec3, to: Vec3) -> &'static str {
    const NAMES: [&str; 8] = [
        "nord",
        "nord-est",
        "est",
        "sud-est",
        "sud",
        "sud-ovest",
        "ovest",
        "nord-ovest",
    ];
    let d = to - from;
    let bearing = d.x.atan2(-d.z).to_degrees().rem_euclid(360.0);
    NAMES[((bearing + 22.5) / 45.0) as usize % 8]
}

/// Sistema: aggiorna `QuestState::thief_hint` con direzione e distanza del
/// ladro rispetto all'NPC (usato nelle battute quando la quest è attiva).
pub fn update_thief_hint(
    mut quest: ResMut<QuestState>,
    thieves: Query<&GlobalTransform, With<QuestThief>>,
    npcs: Query<&GlobalTransform, With<crate::entities::Npc>>,
) {
    let parts = match (thieves.single(), npcs.single()) {
        (Ok(t), Ok(n)) => {
            let from = n.translation();
            let to = t.translation();
            let meters = ((from.distance(to) / 5.0).round() * 5.0) as u32;
            Some((compass_name(from, to), meters))
        }
        _ => None,
    };
    let hint = parts.map(|(name, m)| format!("{name}, circa {m} m"));
    if quest.thief_hint != hint {
        quest.thief_hint = hint;
        quest.thief_hint_parts = parts;
    }
}

/// Battute del Viandante (righe, scelte) in base alla fase; i testi vengono
/// dal file `dialoghi.txt`. Le scelte compaiono solo alla prima richiesta.
pub fn npc_dialog(quest: &QuestState, d: &Dialogs) -> (Vec<String>, Vec<String>, Vec<Vec<String>>) {
    let vars = [
        ("total", quest.coins_total.to_string()),
        ("n", quest.coins_collected.to_string()),
        ("reward", REWARD_COINS.to_string()),
        ("hint", quest.thief_hint.clone().unwrap_or_default()),
    ];
    let section = match quest.stage {
        QuestStage::Idle => "idle",
        QuestStage::SeekThief if quest.thief_hint.is_some() => "seek_thief",
        QuestStage::SeekThief => "seek_thief_nohint",
        QuestStage::Collecting => "collecting",
        QuestStage::Returning if quest.fate == Fate::Pact => "returning_patto",
        QuestStage::Returning if quest.fate == Fate::Spared => "returning_grazia",
        QuestStage::Returning => "returning",
        QuestStage::Completed => "completed",
    };
    let choices = if quest.stage == QuestStage::Idle && !quest.accepted {
        d.choices("idle")
    } else if quest.stage == QuestStage::Returning && quest.fate != Fate::Pact {
        // Alla consegna si può dire la verità o tenersi quattro monete.
        d.choices("returning_scelte")
    } else {
        Vec::new()
    };
    let lines = d.lines(section, &vars);
    let voices = (1..=lines.len())
        .map(|i| voice_clips(d, quest, &format!("{section}.{i}")))
        .collect();
    (lines, choices, voices)
}

/// Clip vocali di una battuta. La voce `@hint` è composta: clip della direzione
/// + clip della distanza più vicina tra quelle registrate.
fn voice_clips(d: &Dialogs, quest: &QuestState, key: &str) -> Vec<String> {
    match d.voice(key) {
        Some("@hint") => {
            let mut clips = Vec::new();
            if let Some((dir, meters)) = quest.thief_hint_parts {
                if let Some(c) = d.voice(&format!("direzione.{dir}")) {
                    clips.push(c.to_string());
                }
                let nearest = (5..=200u32)
                    .step_by(5)
                    .filter(|m| d.voice(&format!("distanza.{m}")).is_some())
                    .min_by_key(|m| m.abs_diff(meters));
                if let Some(m) = nearest
                    && let Some(c) = d.voice(&format!("distanza.{m}"))
                {
                    clips.push(c.to_string());
                }
            }
            clips
        }
        Some(c) => vec![c.to_string()],
        None => Vec::new(),
    }
}

/// Voce del diario: `None` finché la quest non è stata accettata.
pub fn journal_entry(quest: &QuestState, d: &Dialogs) -> Option<QuestEntry> {
    if !quest.accepted {
        return None;
    }
    let stage = quest.stage;
    let after_thief = matches!(
        stage,
        QuestStage::Collecting | QuestStage::Returning | QuestStage::Completed
    );
    let after_coins = matches!(stage, QuestStage::Returning | QuestStage::Completed);
    let state = |done: bool, current: bool| {
        if done {
            ObjState::Done
        } else if current {
            ObjState::Current
        } else {
            ObjState::Pending
        }
    };
    let vars = [
        ("n", quest.coins_collected.to_string()),
        ("total", quest.coins_total.to_string()),
    ];
    // Come è andata, in fondo al riassunto.
    let mut summary = d.text("diario_riassunto", &vars);
    for (on, key) in [
        (quest.mother_coin, "diario_esito_grazia"),
        (
            quest.fate == Fate::Pact && stage == QuestStage::Completed,
            "diario_esito_patto",
        ),
        (quest.kept > 0, "diario_esito_ombra"),
    ] {
        if on {
            summary.push(' ');
            summary.push_str(&d.text(key, &vars));
        }
    }
    let thief_goal = match quest.fate {
        Fate::Spared => "obiettivo_ladro_grazia",
        Fate::Pact => "obiettivo_ladro_patto",
        _ => "obiettivo_ladro",
    };
    Some(QuestEntry {
        title: d.text("diario_titolo", &vars),
        summary,
        status: if stage == QuestStage::Completed {
            QuestStatus::Completed
        } else {
            QuestStatus::Active
        },
        objectives: vec![
            (d.text("obiettivo_parla", &vars), ObjState::Done),
            (
                d.text(thief_goal, &vars),
                state(after_thief, stage == QuestStage::SeekThief),
            ),
            (
                d.text("obiettivo_monete", &vars),
                state(after_coins, stage == QuestStage::Collecting),
            ),
            (
                d.text("obiettivo_ritorno", &vars),
                state(
                    stage == QuestStage::Completed,
                    stage == QuestStage::Returning,
                ),
            ),
        ],
    })
}

/// Sistema: il ladro vivo nota il giocatore entro [`SPOT_DISTANCE`] e lo
/// apostrofa con un fumetto (una sola volta).
pub fn thief_notice_player(
    mut quest: ResMut<QuestState>,
    mut barks: ResMut<Barks>,
    mut voice: ResMut<crate::voice::VoiceQueue>,
    texts: Res<crate::quests::QuestTexts>,
    players: Query<&GlobalTransform, With<Player>>,
    thieves: Query<(Entity, &GlobalTransform), With<QuestThief>>,
    cheats: Res<crate::shell::Cheats>,
) {
    if quest.thief_spotted || cheats.invisible {
        return;
    }
    let (Ok(player), Ok((thief, tt))) = (players.single(), thieves.single()) else {
        return;
    };
    // Chi arriva senz'arma in pugno non viene apostrofato: gli si può parlare.
    if !quest.thief_calm && player.translation().distance(tt.translation()) <= SPOT_DISTANCE {
        quest.thief_spotted = true;
        barks.say(
            thief,
            "Ladro",
            texts.denaro_rubato.text("ladro_avvistato", &[]),
        );
        if let Some(c) = texts.denaro_rubato.voice("ladro_avvistato.1") {
            voice.play_from(
                vec![c.to_string()],
                crate::voice::VoiceKind::Bark,
                Some(thief),
            );
        }
        info!("[QUEST] il ladro ha notato il giocatore");
    }
}

/// Spawn del ladro (enemy rosso con marker QuestThief) in un angolo della mappa.
pub fn spawn_thief(
    commands: &mut Commands,
    asset_server: &AssetServer,
    start: Vec3,
    end: Vec3,
) -> Entity {
    commands
        .spawn((
            crate::scene::SceneNode,
            crate::entities::Enemy,
            QuestThief,
            crate::figure::ModelName("ladro"),
            crate::hud::NameAnchor(1.1),
            crate::combat::EnemyHealth {
                current: 30.0,
                max: 30.0,
            },
            crate::entities::PatrolState {
                waypoints: vec![start, end],
                current: 0,
                speed: crate::entities::PATROL_SPEED,
            },
            RigidBody::Kinematic,
            Collider::capsule(0.4, 0.9),
            LockedAxes::ROTATION_LOCKED,
            LinearVelocity::ZERO,
            CustomPositionIntegration,
            Visibility::default(),
            Transform::from_translation(start),
            NeedsGroundSnap,
        ))
        .with_children(|c| crate::figure::spawn_character(c, asset_server, "ladro"))
        .id()
}

/// Sistema: il ladro resta tranquillo finché il giocatore non ha un'arma in pugno e non lo
/// provoca; al campo si legge la tavoletta; con E gli si parla (ADR 0035).
#[allow(clippy::too_many_arguments)]
pub fn thief_parley(
    keys: Res<ButtonInput<KeyCode>>,
    mut quest: ResMut<QuestState>,
    texts: Res<crate::quests::QuestTexts>,
    equip: Res<crate::inventory::Equipment>,
    mut dialog: ResMut<DialogState>,
    mut nearest: ResMut<crate::entities::NearestInteract>,
    mut barks: ResMut<Barks>,
    mut session: ResMut<crate::save::Session>,
    players: Query<&GlobalTransform, With<Player>>,
    thieves: Query<(Entity, &GlobalTransform, &crate::combat::EnemyHealth), With<QuestThief>>,
    mut greeted: Local<bool>,
) {
    let d = &texts.denaro_rubato;
    let (Ok(player), true) = (players.single(), quest.stage == QuestStage::SeekThief) else {
        quest.thief_calm = false;
        return;
    };
    let p = player.translation();
    // La tavoletta, al campo del ladro.
    if !quest.tablet && p.xz().distance(CAMP) < 3.0 {
        quest.tablet = true;
        session.notify(d.text("tavoletta", &[]));
        info!("[QUEST] tavoletta del ladro letta");
    }
    let Ok((thief, tt, hp)) = thieves.single() else {
        quest.thief_calm = false;
        return;
    };
    if hp.current < hp.max {
        quest.thief_provoked = true;
    }
    quest.thief_calm = !quest.thief_provoked && equip.weapon.is_none();
    let close = tt.translation().distance(p) < PARLEY_RANGE;
    if !quest.thief_calm || !close || dialog.open {
        return;
    }
    if !*greeted {
        *greeted = true;
        barks.say(thief, "Ladro", d.text("ladro_calmo", &[]));
    }
    if nearest.prompt.is_none() {
        nearest.prompt = Some("[E] Parla al ladro".to_string());
    }
    if keys.just_pressed(KeyCode::KeyE) && !dialog.block_interact {
        dialog.open = true;
        dialog.speaker = "Ladro".to_string();
        dialog.lines = d.lines("ladro_parla", &[]);
        dialog.voices = vec![Vec::new(); dialog.lines.len()];
        dialog.choices = d.choices("ladro_parla");
        dialog.index = 0;
        dialog.selected = 0;
        dialog.choosing = false;
        dialog.result = None;
        dialog.finished = false;
        dialog.ai_offer = false;
        dialog.shop = None;
        dialog.speaker_entity = Some(thief);
        info!("dialogo aperto con Ladro");
    }
}

/// Sistema: la scelta fatta parlando al ladro (grazia, patto o sfida).
#[allow(clippy::too_many_arguments)]
pub fn thief_choice(
    mut commands: Commands,
    mut quest: ResMut<QuestState>,
    texts: Res<crate::quests::QuestTexts>,
    dialog: Res<DialogState>,
    character: Res<crate::character::Character>,
    mut inventory: ResMut<Inventory>,
    mut session: ResMut<crate::save::Session>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    thieves: Query<(Entity, &GlobalTransform), With<QuestThief>>,
    mut barks: ResMut<Barks>,
    mut last_open: Local<bool>,
) {
    let was_open = std::mem::replace(&mut *last_open, dialog.open);
    if !(was_open && !dialog.open && dialog.finished && dialog.speaker == "Ladro") {
        return;
    }
    let d = &texts.denaro_rubato;
    let Ok((thief, tt)) = thieves.single() else {
        return;
    };
    // Si convince chi ha letto la sua tavoletta, o chi sa parlare (Carisma alto).
    let persuasive = quest.tablet || character.stats.get(crate::character::Stat::Charisma) >= 7;
    match dialog.result {
        Some(0) if persuasive => {
            quest.fate = Fate::Spared;
            quest.stage = QuestStage::Collecting;
            quest.coins_collected = 0;
            quest.thief_hint = None;
            let at = tt.translation() - Vec3::Y * THIEF_CENTER_OFFSET;
            spawn_loot(&mut commands, &mut meshes, &mut materials, at, LOOT_TOTAL);
            commands.entity(thief).despawn();
            session.notify(d.text("ladro_grazia", &[]));
            show_title(&mut quest, "Il ladro se ne va: raccogli il denaro", 3.0);
            info!("[QUEST] ladro risparmiato: lascia le monete e se ne va");
        }
        Some(0) => {
            quest.thief_provoked = true;
            barks.say(thief, "Ladro", d.text("ladro_rifiuta", &[]));
            info!("[QUEST] il ladro non si lascia convincere");
        }
        Some(1) => {
            quest.fate = Fate::Pact;
            quest.stage = QuestStage::Returning;
            quest.coins_collected = PACT_COINS;
            quest.thief_hint = None;
            inventory.add(ItemKind::Coin, PACT_COINS);
            commands.entity(thief).despawn();
            session.notify(d.text("ladro_patto", &[]));
            show_title(&mut quest, "Patto stretto: torna dal Viandante", 3.0);
            info!("[QUEST] patto con il ladro: {PACT_COINS} monete a testa");
        }
        Some(2) => {
            quest.thief_provoked = true;
            barks.say(thief, "Ladro", d.text("ladro_sfida", &[]));
            info!("[QUEST] ladro sfidato");
        }
        _ => {}
    }
}
