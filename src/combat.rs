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
    #[allow(dead_code)]
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
pub const ATTACK_COOLDOWN: f32 = 0.6;
pub const ATTACK_RANGE: f32 = 2.5;
pub const ATTACK_DAMAGE: f32 = 4.0;
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

/// Attacco melee del player su click sinistro.
/// Gira in Update (input), solo in Playing e se nessun dialogo è aperto.
pub fn player_attack(
    mouse: Res<ButtonInput<MouseButton>>,
    mut raw_mouse: MessageReader<bevy::input::mouse::MouseButtonInput>,
    mut combat: Query<&mut CombatState, With<Player>>,
    players: Query<&GlobalTransform, With<Player>>,
    mut enemies: Query<(&GlobalTransform, &mut EnemyHealth, Entity), With<Enemy>>,
    dialog: Res<crate::entities::DialogState>,
    time: Res<Time>,
) {
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
    info!("click sinistro: attacco!");

    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();
    let pf = *player.forward();

    // Nemico più vicino entro range e nel cono frontale (dot > 0.3).
    let mut best: Option<(f32, Entity, f32)> = None; // (dist, entity, new_hp)
    for (t, hp, entity) in &mut enemies {
        let to = t.translation() - p;
        let dist = to.length();
        let dir = to.normalize_or_zero();
        if dist > ATTACK_RANGE {
            continue;
        }
        if dir.dot(pf) < 0.3 {
            continue;
        }
        let new_hp = (hp.current - ATTACK_DAMAGE).max(0.0);
        match &best {
            Some((bd, _, _)) if dist >= *bd => {}
            _ => best = Some((dist, entity, new_hp)),
        }
    }

    if let Some((_, entity, new_hp)) = best {
        if let Ok((_, mut hp, _)) = enemies.get_mut(entity) {
            hp.current = new_hp;
        }
        combat.attack_cd = ATTACK_COOLDOWN;
        info!("attacco su enemy {entity:?}: vita -> {:.0}", new_hp);
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
) {
    let Ok((pt, mut hp, mut combat, mut transform)) = player.single_mut() else {
        return;
    };
    combat.iframe = (combat.iframe - time.delta_secs()).max(0.0);
    cd.left = (cd.left - time.delta_secs()).max(0.0);

    if combat.iframe > 0.0 || cd.left > 0.0 || hp.current <= 0.0 {
        return;
    }

    let p = pt.translation();
    for et in &enemies {
        if et.translation().distance(p) <= CONTACT_RANGE {
            hp.current -= CONTACT_DAMAGE;
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

/// Gestione morte nemici: despawn + marker di respawn con i waypoint.
pub fn handle_enemy_death(
    mut commands: Commands,
    enemies: Query<
        (
            Entity,
            &EnemyHealth,
            &PatrolState,
            Option<&crate::quest::QuestThief>,
        ),
        With<Enemy>,
    >,
) {
    for (entity, hp, patrol, is_thief) in &enemies {
        if hp.current <= 0.0 {
            commands.entity(entity).despawn();
            // Il ladro della quest NON respawna: morte definitiva.
            if is_thief.is_none() {
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
        crate::entities::spawn_enemy(&mut commands, &mut meshes, &mut materials, start, end);
    }
}
