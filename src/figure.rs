//! Personaggi: modelli glTF CC0 con scheletro comune (ADR 0015).
//!
//! I file in `assets/models/characters/` sono composti da `scripts/build_characters.py`: ogni aspetto ha
//! lo stesso scheletro (65 ossa), le clip stanno in `animations.gltf`. Il modello è un figlio del
//! corpo, abbassato ai piedi e girato di 180° (il glTF guarda +Z, il gioco -Z). Quando la scena è pronta
//! parte la clip di riposo, `animation.rs` prende il controllo e le ossa delle mani diventano disponibili per
//! gli oggetti impugnati.

use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use std::time::Duration;

/// Altezza del centro del corpo sul terreno (capsula da 1,7 m).
pub const CENTER_Y: f32 = 0.85;
/// Clip in `animations.gltf`, nell'ordine scritto da `scripts/build_characters.py`.
pub const CLIPS: [&str; 15] = [
    "Idle_Loop",
    "Walk_Loop",
    "Jog_Fwd_Loop",
    "Sprint_Loop",
    "Jump_Start",
    "Jump_Loop",
    "Jump_Land",
    "Sword_Idle",
    "Sword_Attack",
    "Punch_Jab",
    "Hit_Chest",
    "Death01",
    "Idle_Talking_Loop",
    "Interact",
    "Spell_Simple_Shoot",
];
const ANIMATIONS: &str = "models/characters/animations.gltf";

/// Grafo con tutte le clip, condiviso dai personaggi.
#[derive(Resource)]
pub struct CharacterAnims {
    graph: Handle<AnimationGraph>,
    clips: Vec<AnimationNodeIndex>,
}

impl CharacterAnims {
    /// Nodo del grafo della clip `index` di `CLIPS`.
    pub fn clip(&self, index: usize) -> AnimationNodeIndex {
        self.clips[index]
    }
}

/// Radice del modello (figlio del corpo).
#[derive(Component)]
pub struct CharacterModel;

/// Aspetto del corpo: serve a ricreare il modello quando il personaggio muore.
#[derive(Component, Clone)]
pub struct ModelName(pub &'static str);

/// Corpo senza vita: mostra la clip di morte, poi sparisce.
#[derive(Component)]
pub struct Corpse {
    pub timer: f32,
}

/// Durata (s) della permanenza di un corpo a terra.
const CORPSE_SECONDS: f32 = 6.0;
const DEATH_CLIP: usize = 11;

/// Radice del modello, in attesa di sapere se il file si è caricato.
#[derive(Component)]
pub struct PendingModel;

/// Ossa delle mani del personaggio (sul corpo, cioè sul genitore del modello).
#[derive(Component, Clone, Copy)]
pub struct HandBones {
    pub right: Entity,
    pub left: Entity,
}

/// Oggetto da agganciare a una mano del corpo a cui è figlio, appena le ossa sono pronte.
#[derive(Component)]
pub struct AttachToHand {
    pub right: bool,
}

/// Avvio: carica le clip e costruisce il grafo delle animazioni.
pub fn load_character_anims(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let handles: Vec<Handle<AnimationClip>> = (0..CLIPS.len())
        .map(|i| asset_server.load(GltfAssetLabel::Animation(i).from_asset(ANIMATIONS)))
        .collect();
    let (graph, clips) = AnimationGraph::from_clips(handles);
    commands.insert_resource(CharacterAnims {
        graph: graphs.add(graph),
        clips,
    });
}

/// Crea il modello `name` come figlio del corpo.
pub fn spawn_character(parent: &mut ChildSpawnerCommands, asset_server: &AssetServer, name: &str) {
    let scene: Handle<WorldAsset> = asset_server
        .load(GltfAssetLabel::Scene(0).from_asset(format!("models/characters/{name}.gltf")));
    parent
        .spawn((
            PendingModel,
            CharacterModel,
            WorldAssetRoot(scene),
            Transform::from_xyz(0.0, -CENTER_Y, 0.0)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        ))
        .observe(on_character_ready);
}

/// Scena pronta: avvia la clip di riposo (fase diversa per ciascuno) e registra le ossa delle mani.
#[allow(clippy::too_many_arguments)]
fn on_character_ready(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    anims: Res<CharacterAnims>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    names: Query<&Name>,
    corpses: Query<(), With<Corpse>>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
) {
    let root = ready.entity;
    let body = parents.get(root).ok().map(|p| p.parent());
    let dying = body.is_some_and(|b| corpses.contains(b));
    let (mut right, mut left) = (None, None);
    for e in children.iter_descendants(root) {
        if let Ok(name) = names.get(e) {
            match name.as_str() {
                "hand_r" => right = Some(e),
                "hand_l" => left = Some(e),
                _ => {}
            }
        }
        if let Ok((player_entity, mut player)) = players.get_mut(e) {
            let mut transitions = AnimationTransitions::new();
            if dying {
                transitions.play(&mut player, anims.clips[DEATH_CLIP], Duration::ZERO);
            } else {
                let phase = (root.to_bits().wrapping_mul(2_654_435_761) % 1000) as f32 / 1000.0;
                transitions
                    .play(&mut player, anims.clips[0], Duration::ZERO)
                    .repeat()
                    .seek_to(phase);
                if let Some(body) = body {
                    commands
                        .entity(body)
                        .insert(crate::animation::Animator::new(player_entity, root));
                }
            }
            commands
                .entity(player_entity)
                .insert((AnimationGraphHandle(anims.graph.clone()), transitions));
        }
    }
    if let (Some(right), Some(left), Ok(body)) = (right, left, parents.get(root)) {
        commands
            .entity(body.parent())
            .insert(HandBones { right, left });
    } else {
        warn!("[FIGURE] modello senza ossa delle mani");
    }
}

/// Sistema: aggancia alle ossa delle mani gli oggetti in attesa (armi, scudo, lancia, lanterna).
pub fn attach_to_hands(
    mut commands: Commands,
    pending: Query<(Entity, &AttachToHand, &ChildOf)>,
    bones: Query<&HandBones>,
) {
    for (e, hand, parent) in &pending {
        if let Ok(b) = bones.get(parent.parent()) {
            let bone = if hand.right { b.right } else { b.left };
            commands
                .entity(e)
                .remove::<AttachToHand>()
                .insert(ChildOf(bone));
        }
    }
}

/// Sistema: se il file di un modello non si carica, mostra una figura a blocchi al suo posto.
pub fn model_fallback(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pending: Query<(Entity, &WorldAssetRoot), With<PendingModel>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, root) in &pending {
        let state = asset_server.load_state(&root.0);
        if state.is_loaded() {
            commands.entity(e).remove::<PendingModel>();
        } else if state.is_failed() {
            error!("[FIGURE] modello non caricato: figura di ripiego");
            let block = (
                Mesh3d(meshes.add(Cuboid::new(0.6, 1.7, 0.4))),
                MeshMaterial3d(materials.add(Color::srgb(0.5, 0.45, 0.4))),
                Transform::from_xyz(0.0, CENTER_Y, 0.0),
            );
            commands
                .entity(e)
                .remove::<PendingModel>()
                .with_children(|c| {
                    c.spawn(block);
                });
        }
    }
}

/// Crea un corpo senza collider che esegue la clip di morte.
pub fn spawn_corpse(
    commands: &mut Commands,
    asset_server: &AssetServer,
    model: &str,
    at: Vec3,
    yaw: Quat,
) {
    commands
        .spawn((
            crate::scene::SceneNode,
            Corpse {
                timer: CORPSE_SECONDS,
            },
            Transform::from_translation(at).with_rotation(yaw),
            Visibility::default(),
        ))
        .with_children(|c| spawn_character(c, asset_server, model));
}

/// Sistema: toglie i corpi dopo qualche secondo.
pub fn tick_corpses(
    mut commands: Commands,
    time: Res<Time>,
    mut corpses: Query<(Entity, &mut Corpse)>,
) {
    for (e, mut c) in &mut corpses {
        c.timer -= time.delta_secs();
        if c.timer <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, load_character_anims)
        .add_systems(Update, (attach_to_hands, model_fallback, tick_corpses));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models/characters")
    }

    #[test]
    fn every_clip_is_in_the_animation_file_in_order() {
        let text = std::fs::read_to_string(dir().join("animations.gltf")).unwrap();
        let mut last = 0;
        for clip in CLIPS {
            let at = text
                .find(&format!("\"name\":\"{clip}\""))
                .unwrap_or_else(|| panic!("manca la clip {clip}"));
            assert!(at > last, "clip fuori ordine: {clip}");
            last = at;
        }
    }

    #[test]
    fn every_villager_model_exists() {
        for def in crate::villagers::load_defs() {
            assert!(
                dir().join(format!("{}.gltf", def.model)).exists(),
                "{}: manca il modello '{}'",
                def.name,
                def.model
            );
        }
        for model in ["giocatore", "viandante", "ladro"] {
            assert!(dir().join(format!("{model}.gltf")).exists(), "{model}");
        }
    }

    #[test]
    fn textures_referenced_by_models_exist() {
        for entry in std::fs::read_dir(dir()).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "gltf") {
                let text = std::fs::read_to_string(&path).unwrap();
                for part in text.split("\"uri\":\"").skip(1) {
                    let uri = part.split('"').next().unwrap();
                    assert!(dir().join(uri).exists(), "{path:?}: manca {uri}");
                }
            }
        }
    }
}
