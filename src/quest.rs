//! Quest "Denaro Rubato" (Fase 5): storia, stadi, monete, titolo overlay.
//!
//! Flusso:
//! 1. `Idle`: il player parla con il Viandante (NPC blu) → quest accettata,
//!    overlay titolo "Denaro Rubato" per 3 s, stage = `SeekThief`.
//! 2. `SeekThief`: il ladro (enemy rosso) è in un angolo della mappa. Ucciderlo
//!    → stage `LootSpawned`, spawn di 12 monete gialle (`QuestCoin`) al suo posto.
//! 3. `Collecting { collected, total }`: raccolta automatica delle monete entro
//!    2 m. Quando collected == total → stage `Returning`.
//! 4. `Returning`: torna dal Viandante. Alla chiusura del dialogo finale →
//!    stage `Completed`, overlay "Missione completata".
//!
//! Nessun salvataggio su file: la quest vive solo in memoria (come da piano).

use crate::entities::DialogState;
use crate::player::Player;
use crate::scene::SceneNode;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Numero di monete da recuperare.
pub const LOOT_TOTAL: u32 = 12;

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
}

impl Default for QuestState {
    fn default() -> Self {
        Self {
            stage: QuestStage::Idle,
            coins_collected: 0,
            coins_total: LOOT_TOTAL,
            title: None,
            accepted: false,
            final_done: false,
        }
    }
}

/// Marker sul ladro (enemy rosso) che fa parte della quest.
#[derive(Component)]
pub struct QuestThief;

/// Marker su una moneta della quest (cubo giallo).
#[derive(Component)]
pub struct QuestCoin;

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
    mut last_open: Local<bool>,
) {
    // Rileva il fronte di discesa: dialog.open true -> false.
    let was_open = *last_open;
    *last_open = dialog.open;
    if was_open && !dialog.open {
        match quest.stage {
            QuestStage::Idle if !quest.accepted => {
                quest.accepted = true;
                quest.stage = QuestStage::SeekThief;
                show_title(&mut quest, "Denaro Rubato", 3.0);
                info!("[QUEST] accettata: Denaro Rubato");
            }
            QuestStage::Returning if !quest.final_done => {
                quest.final_done = true;
                quest.stage = QuestStage::Completed;
                show_title(&mut quest, "Missione completata: Denaro recuperato", 4.0);
                info!("[QUEST] completata");
            }
            _ => {}
        }
    }
}

/// Sistema: quando il ladro (QuestThief) muore (vita <= 0), spawna le monete
/// al suo posto e passa alla fase Collecting.
pub fn quest_on_thief_death(
    mut commands: Commands,
    thieves: Query<(&GlobalTransform, &crate::combat::EnemyHealth), With<QuestThief>>,
    mut quest: ResMut<QuestState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if quest.stage != QuestStage::SeekThief {
        return;
    }
    for (t, hp) in &thieves {
        if hp.current <= 0.0 {
            quest.stage = QuestStage::Collecting;
            quest.coins_collected = 0;
            show_title(&mut quest, "Ladro sconfitto: raccogli il denaro", 3.0);
            spawn_loot(
                &mut commands,
                &mut meshes,
                &mut materials,
                t.translation(),
                LOOT_TOTAL,
            );
            info!("[QUEST] ladro morto, stage=Collecting");
        }
    }
}

/// Spawn di N monete gialle (cubetti) attorno a una posizione (cerchio).
pub fn spawn_loot(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    count: u32,
) {
    let gold = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.80, 0.15),
        emissive: LinearRgba::new(0.6, 0.45, 0.05, 0.0),
        perceptual_roughness: 0.3,
        metallic: 0.8,
        ..default()
    });
    for i in 0..count {
        let angle = (i as f32) / (count as f32) * std::f32::consts::TAU;
        let radius = 1.0 + ((i % 3) as f32) * 0.4;
        let pos = center + Vec3::new(angle.cos() * radius, 0.2, angle.sin() * radius);
        commands.spawn((
            SceneNode,
            QuestCoin,
            RigidBody::Static,
            Collider::cuboid(0.35, 0.35, 0.35),
            Mesh3d(meshes.add(Cuboid::new(0.35, 0.35, 0.35))),
            MeshMaterial3d(gold.clone()),
            Transform::from_translation(pos),
        ));
    }
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

/// NPC: dialogo che dipende dalla fase quest (iniziale vs finale).
pub fn npc_quest_lines(quest: &QuestState) -> &'static [&'static str] {
    match quest.stage {
        QuestStage::Idle => &[
            "Salve, viaggiatore! Un ladro mi ha rubato tutte le monete d'oro.",
            "Era un uomo alto e rosso, si nasconde tra le colline a nord-ovest.",
            "Uccidilo e riporta qui il mio denaro. Ti aspetto!",
        ],
        QuestStage::Completed => &[
            "Le mie monete! Grazie, viaggiatore! Sei un vero eroe di Planet One.",
            "Il ladro non darà più fastidio a nessuno.",
        ],
        _ => &[
            "Il ladro è là fuori... uccidilo e raccogli le mie 12 monete.",
            "Poi torna qui con il bottino!",
        ],
    }
}

/// Spawn del ladro (enemy rosso con marker QuestThief) in un angolo della mappa.
pub fn spawn_thief(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    start: Vec3,
    end: Vec3,
) {
    let mesh = meshes.add(Cuboid::new(0.9, 1.4, 0.9));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.15, 0.10),
        perceptual_roughness: 0.5,
        ..default()
    });
    commands.spawn((
        crate::scene::SceneNode,
        crate::entities::Enemy,
        QuestThief,
        crate::combat::EnemyHealth {
            current: 30.0,
            max: 30.0,
        },
        crate::entities::PatrolState {
            waypoints: vec![start, end],
            current: 0,
            speed: 3.0,
        },
        RigidBody::Kinematic,
        Collider::capsule(0.4, 0.9),
        LockedAxes::ROTATION_LOCKED,
        LinearVelocity::ZERO,
        CustomPositionIntegration,
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(start),
    ));
}
