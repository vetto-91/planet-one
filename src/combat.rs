//! Combattimento base (Fase 4.2): attacco melee, danno, morte e respawn.
//!
//! Flusso:
//! - [`player_attack`] (Update): click sinistro → daño al nemico più vicino
//!   nel cono frontale entro 2.5 m (cooldown 0.6 s).
//! - [`contact_damage`] (Update): contatto player↔nemico entro 1.2 m → danno
//!   ogni 0.8 s con iframe 0.3 s; se vita player ≤ 0 → respawn allo spawn.
//! - [`handle_enemy_death`] (Update): nemici con vita ≤ 0 vengono despawnati
//!   e sostituiti da un marker `EnemyRespawn`.
//! - [`enemy_respawn_tick`] (Update): dopo 3 s respawna il nemico sul primo
//!   waypoint, usando lo stesso spawn di `entities::spawn_enemy`.

use crate::entities::{Enemy, PatrolState};
use crate::player::{Player, PlayerHealth};
use bevy::prelude::*;

/// Vita del nemico.
#[derive(Component)]
pub struct EnemyHealth {
    pub current: f32,
    pub max: f32,
}

impl Default for EnemyHealth {
    fn default() -> Self {
        Self {
            current: 20.0,
            max: 20.0,
        }
    }
}

/// Stato di combattimento del player: cooldown attacco + iframe dopo danno.
#[derive(Component, Default)]
pub struct CombatState {
    pub attack_cd: f32,
    pub iframe: f32,
}

/// Marker su entità in attesa di respawn (nemico morto).
#[derive(Component)]
pub struct EnemyRespawn {
    pub timer: f32,
    pub waypoints: Vec<Vec3>,
}

/// Costanti di combattimento.
pub const CONTACT_DAMAGE_COOLDOWN: f32 = 0.8;
pub const CONTACT_DAMAGE: f32 = 5.0;
pub const CONTACT_RANGE: f32 = 1.2;
pub const ENEMY_RESPAWN_TIME: f32 = 3.0;
pub const PLAYER_IFRAME: f32 = 0.3;

/// Timer interno per il danno da contatto (resource singleton, semplice).
#[derive(Resource, Default)]
pub struct ContactDamageCd {
    pub left: f32,
}

/// Durata (s) della finestra di oscillazione dell'arma (animazione).
pub const SWING_TIME: f32 = 0.28;

/// Tempo residuo dell'oscillazione dell'arma impugnata (vedi `equipment.rs`).
#[derive(Resource, Default)]
pub struct SwingTimer(pub f32);

/// Freccia in volo.
#[derive(Component)]
pub struct Arrow {
    vel: Vec3,
    life: f32,
}

/// Cose contro cui si ferma una freccia: terreno ed edifici.
type ArrowSolids = Or<(
    With<crate::scene::Ground>,
    With<crate::camera::BlocksCamera>,
)>;

/// Danno di una freccia e raggio di impatto sul nemico.
const ARROW_DAMAGE: f32 = 10.0;
const ARROW_SPEED: f32 = 38.0;

/// Attacco del player su click sinistro, secondo l'arma impugnata: a mani nude, spada
/// corta/lunga, ascia (colpisce più nemici) oppure arco (consuma una freccia).
/// Gira in Update (input), solo in Playing e se nessun dialogo è aperto.
#[allow(clippy::too_many_arguments)]
pub fn player_attack(
    visuals: Query<&GlobalTransform, With<crate::player::PlayerVisual>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut raw_mouse: MessageReader<bevy::input::mouse::MouseButtonInput>,
    mut combat: Query<&mut CombatState, With<Player>>,
    players: Query<&GlobalTransform, With<Player>>,
    mut enemies: Query<(&GlobalTransform, &mut EnemyHealth, Entity), With<Enemy>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    dialog: Res<crate::entities::DialogState>,
    equip: Res<crate::inventory::Equipment>,
    mut inventory: ResMut<crate::inventory::Inventory>,
    mut swing: ResMut<SwingTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    swing.0 = (swing.0 - time.delta_secs()).max(0.0);
    if dialog.open {
        return;
    }

    let Ok(mut combat) = combat.single_mut() else {
        return;
    };
    combat.attack_cd = (combat.attack_cd - time.delta_secs()).max(0.0);

    // Click sinistro: accettiamo sia da ButtonInput sia dagli eventi raw
    // (utile quando il cursore e' warpato al centro in fullscreen).
    let clicked = mouse.just_pressed(MouseButton::Left)
        || raw_mouse
            .read()
            .any(|e| e.button == MouseButton::Left && e.state.is_pressed());
    if !clicked || combat.attack_cd > 0.0 {
        return;
    }
    let stats = crate::inventory::weapon_stats(equip.weapon);
    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();
    // Il corpo fisico non ruota: la direzione è quella del modello visibile.
    let pf = visuals.single().map_or(*player.forward(), |v| *v.forward());

    // Arco: una freccia verso il mirino.
    if stats.ranged {
        if inventory.qty(crate::inventory::ItemKind::Arrow) == 0 {
            info!("arco: niente frecce");
            return;
        }
        let Ok(cam) = cameras.single() else {
            return;
        };
        inventory.consume(crate::inventory::ItemKind::Arrow, 1);
        combat.attack_cd = stats.cooldown;
        swing.0 = SWING_TIME;
        let dir = cam.forward();
        let start = p + Vec3::Y * 0.4 + *dir * 0.8;
        commands.spawn((
            crate::scene::SceneNode,
            Arrow {
                vel: *dir * ARROW_SPEED,
                life: 3.0,
            },
            Mesh3d(meshes.add(Cuboid::new(0.04, 0.04, 0.7))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.78, 0.55),
                ..default()
            })),
            Transform::from_translation(start).looking_to(dir, Vec3::Y),
        ));
        return;
    }

    // Mischia: nemico più vicino (o tutti nel cono con l'ascia) entro il raggio dell'arma.
    let mut hit: Vec<(f32, Entity)> = Vec::new();
    for (t, _, entity) in &enemies {
        let to = t.translation() - p;
        let dist = to.length();
        if dist > stats.range || to.normalize_or_zero().dot(pf) < 0.3 {
            continue;
        }
        hit.push((dist, entity));
    }
    hit.sort_by(|a, b| a.0.total_cmp(&b.0));
    if !stats.cleave {
        hit.truncate(1);
    }
    swing.0 = SWING_TIME;
    if hit.is_empty() {
        return;
    }
    combat.attack_cd = stats.cooldown;
    for (_, entity) in hit {
        if let Ok((_, mut hp, _)) = enemies.get_mut(entity) {
            hp.current = (hp.current - stats.damage).max(0.0);
            info!("attacco su enemy {entity:?}: vita -> {:.0}", hp.current);
        }
    }
}

/// Sistema: muove le frecce (con una lieve caduta), le ferma contro terreno/edifici e
/// danneggia i nemici che colpiscono.
#[allow(clippy::too_many_arguments)]
pub fn update_arrows(
    mut commands: Commands,
    time: Res<Time>,
    spatial: avian3d::prelude::SpatialQuery,
    solids: Query<(), ArrowSolids>,
    mut arrows: Query<(Entity, &mut Transform, &mut Arrow)>,
    mut enemies: Query<(&GlobalTransform, &mut EnemyHealth), With<Enemy>>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut a) in &mut arrows {
        a.life -= dt;
        if a.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        a.vel.y -= 6.0 * dt;
        let step = a.vel * dt;
        let from = tf.translation;
        // Impatto contro il mondo.
        if let Ok(dir) = Dir3::new(step)
            && spatial
                .cast_ray_predicate(
                    from,
                    dir,
                    step.length(),
                    true,
                    &avian3d::prelude::SpatialQueryFilter::default(),
                    &|ent| solids.contains(ent),
                )
                .is_some()
        {
            commands.entity(e).despawn();
            continue;
        }
        tf.translation += step;
        tf.look_to(a.vel, Vec3::Y);
        // Impatto contro un nemico (distanza dal punto medio del tratto).
        let mid = from + step / 2.0;
        let mut hit = false;
        for (et, mut hp) in &mut enemies {
            if et.translation().distance(mid) < 1.1 {
                hp.current = (hp.current - ARROW_DAMAGE).max(0.0);
                info!("freccia su nemico: vita -> {:.0}", hp.current);
                hit = true;
                break;
            }
        }
        if hit {
            commands.entity(e).despawn();
        }
    }
}

/// Danno da contatto player↔nemico + morte/respawn del player.
pub fn contact_damage(
    mut player: Query<
        (
            &GlobalTransform,
            &mut PlayerHealth,
            &mut CombatState,
            &mut Transform,
        ),
        With<Player>,
    >,
    enemies: Query<&GlobalTransform, (With<Enemy>, Without<Player>)>,
    mut cd: ResMut<ContactDamageCd>,
    time: Res<Time>,
    mut orbit: ResMut<crate::camera::OrbitCamera>,
    equip: Res<crate::inventory::Equipment>,
    cheats: Res<crate::shell::Cheats>,
) {
    let Ok((pt, mut hp, mut combat, mut transform)) = player.single_mut() else {
        return;
    };
    combat.iframe = (combat.iframe - time.delta_secs()).max(0.0);
    cd.left = (cd.left - time.delta_secs()).max(0.0);

    // Godmode e invisibilità: i nemici non fanno danno.
    if cheats.god || cheats.invisible {
        return;
    }

    if combat.iframe > 0.0 || cd.left > 0.0 || hp.current <= 0.0 {
        return;
    }

    let p = pt.translation();
    for et in &enemies {
        if et.translation().distance(p) <= CONTACT_RANGE {
            let factor = if equip.shield {
                crate::inventory::SHIELD_FACTOR
            } else {
                1.0
            };
            hp.current -= CONTACT_DAMAGE * factor;
            combat.iframe = PLAYER_IFRAME;
            cd.left = CONTACT_DAMAGE_COOLDOWN;
            info!("colpito: player vita {:.0}", hp.current);

            if hp.current <= 0.0 {
                // Respawn locale: vita piena, posizione allo spawn, camera resettata.
                hp.current = hp.max;
                transform.translation = Vec3::new(0.0, 1.0, 0.0);
                *orbit = crate::camera::OrbitCamera::default();
                info!("player morto -> respawn allo spawn");
            }
            break;
        }
    }
}

/// Query dei nemici con i dati necessari alla gestione della morte.
type EnemiesQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static EnemyHealth,
        &'static PatrolState,
        &'static GlobalTransform,
        Option<&'static crate::quests::denaro_rubato::QuestThief>,
    ),
    With<Enemy>,
>;

/// Gestione morte nemici: despawn + marker di respawn con i waypoint.
pub fn handle_enemy_death(
    mut commands: Commands,
    mut quest: ResMut<crate::quests::denaro_rubato::QuestState>,
    enemies: EnemiesQuery,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    names: Query<&crate::figure::ModelName>,
) {
    for (entity, hp, patrol, transform, is_thief) in &enemies {
        if hp.current <= 0.0 {
            // Il corpo resta a terra qualche secondo: solo visivo, senza collider.
            if let Ok(name) = names.get(entity) {
                let (_, rotation, translation) = transform.to_scale_rotation_translation();
                crate::figure::spawn_corpse(
                    &mut commands,
                    &asset_server,
                    name.0,
                    translation,
                    rotation,
                );
            }
            commands.entity(entity).despawn();
            // Il ladro della quest NON respawna: morte definitiva. La posizione
            // viene registrata qui (prima del despawn) così la quest non dipende
            // dall'ordine di esecuzione dei sistemi.
            if is_thief.is_some() {
                quest.thief_killed_at = Some(transform.translation());
            }
            if is_thief.is_none() {
                // I nemici comuni lasciano qualche moneta.
                crate::inventory::spawn_pickup(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    crate::inventory::ItemKind::Coin,
                    4,
                    transform.translation() + Vec3::Y * 0.2,
                );
                let wps = patrol.waypoints.clone();
                commands.spawn((
                    crate::scene::SceneNode,
                    EnemyRespawn {
                        timer: ENEMY_RESPAWN_TIME,
                        waypoints: wps,
                    },
                ));
                info!("nemico morto -> respawn in {:.0}s", ENEMY_RESPAWN_TIME);
            } else {
                info!("[QUEST] ladro ucciso definitivamente");
            }
        }
    }
}

/// Tick dei respawn nemici: dopo il timer, respawna sul primo waypoint.
pub fn enemy_respawn_tick(
    mut commands: Commands,
    mut respawns: Query<(Entity, &mut EnemyRespawn)>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    let mut to_spawn: Vec<(Vec3, Vec3)> = Vec::new();
    for (entity, mut respawn) in &mut respawns {
        respawn.timer -= time.delta_secs();
        if respawn.timer <= 0.0 {
            let start = respawn.waypoints[0];
            let end = respawn.waypoints.get(1).copied().unwrap_or(start);
            to_spawn.push((start, end));
            commands.entity(entity).despawn();
        }
    }
    for (start, end) in to_spawn {
        crate::entities::spawn_enemy(&mut commands, &asset_server, start, end);
    }
}
