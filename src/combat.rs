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
use crate::skills::{self, Skill};
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
pub(crate) type ArrowSolids = Or<(
    With<crate::scene::Ground>,
    With<crate::camera::BlocksCamera>,
)>;

/// Danno di una freccia e raggio di impatto sul nemico.
const ARROW_DAMAGE: f32 = 10.0;
pub const ARROW_SPEED: f32 = 38.0;
/// Caduta della freccia (m/s²).
pub const ARROW_DROP: f32 = 6.0;

/// Attacco del player su click sinistro, secondo l'arma impugnata: a mani nude, spada
/// corta/lunga, ascia (colpisce più nemici) oppure arco (consuma una freccia).
/// Gira in Update (input), solo in Playing e se nessun dialogo è aperto.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn player_attack(
    visuals: Query<&GlobalTransform, With<crate::player::PlayerVisual>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut raw_mouse: MessageReader<bevy::input::mouse::MouseButtonInput>,
    mut combat: Query<&mut CombatState, With<Player>>,
    players: Query<&GlobalTransform, With<Player>>,
    mut enemies: Query<
        (
            &GlobalTransform,
            &mut EnemyHealth,
            Entity,
            Has<crate::quests::denaro_rubato::QuestThief>,
        ),
        With<Enemy>,
    >,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    dialog: Res<crate::entities::DialogState>,
    equip: Res<crate::inventory::Equipment>,
    mut inventory: ResMut<crate::inventory::Inventory>,
    mut swing: ResMut<SwingTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    character: Res<crate::character::Character>,
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
        crate::vitals::spend(crate::vitals::SWING_COST);
        let dir = cam.forward();
        let start = p + Vec3::Y * 0.4 + *dir * 0.8;
        loose(&mut commands, &mut meshes, &mut materials, start, dir);
        return;
    }

    // Mischia: nemico più vicino (o tutti nel cono con l'ascia) entro il raggio dell'arma.
    let mut hit: Vec<(f32, Entity)> = Vec::new();
    for (t, _, entity, _) in &enemies {
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
    crate::vitals::spend(crate::vitals::SWING_COST);
    if hit.is_empty() {
        return;
    }
    combat.attack_cd = stats.cooldown;
    for (_, entity) in hit {
        if let Ok((_, mut hp, _, thief)) = enemies.get_mut(entity) {
            let damage = stats.damage
                * character.stats.melee_factor()
                * (1.0 + 0.06 * skills::rank(Skill::Blade));
            hp.current = (hp.current - damage).max(0.0);
            skills::train(Skill::Blade, 4.0);
            info!("attacco su enemy {entity:?}: vita -> {:.0}", hp.current);
            // Disarmo di Grazia (ADR 0049): ferito gravemente, il nemico getta l'arma e se ne va.
            if surrenders(
                hp.current,
                hp.max,
                skills::active_talent(Skill::Blade),
                thief,
            ) {
                commands.entity(entity).despawn();
                inventory.add(crate::inventory::ItemKind::Coin, 4);
                crate::karma::deed_as("pieta", 2, "un nemico disarmato si è arreso");
                info!("[NEMICI] si arrende e se ne va");
            }
        }
    }
}

/// Un nemico ferito gravemente si arrende, se si ha il Disarmo di Grazia (non il ladro della quest,
/// che ha la sua storia).
pub fn surrenders(current: f32, max: f32, talent: bool, thief: bool) -> bool {
    talent && !thief && current > 0.0 && current < max * 0.3
}

/// Scocca una freccia del giocatore da `start` verso `dir`.
pub fn loose(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    start: Vec3,
    dir: Dir3,
) {
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
        a.vel.y -= ARROW_DROP * dt;
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
                let damage = ARROW_DAMAGE * (1.0 + 0.08 * skills::rank(Skill::Archery));
                hp.current = (hp.current - damage).max(0.0);
                skills::train(Skill::Archery, 6.0);
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

/// Danno da contatto player↔nemico (la morte è in [`player_death`]).
#[allow(clippy::type_complexity)]
pub fn contact_damage(
    mut player: Query<(&GlobalTransform, &mut PlayerHealth, &mut CombatState), With<Player>>,
    enemies: Query<
        (
            &GlobalTransform,
            Has<crate::quests::denaro_rubato::QuestThief>,
            Has<crate::magic::Calmed>,
        ),
        (With<Enemy>, Without<Player>),
    >,
    quest: Res<crate::quests::denaro_rubato::QuestState>,
    mut cd: ResMut<ContactDamageCd>,
    time: Res<Time>,
    equip: Res<crate::inventory::Equipment>,
    cheats: Res<crate::shell::Cheats>,
) {
    let Ok((pt, mut hp, mut combat)) = player.single_mut() else {
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
    for (et, thief, calmed) in &enemies {
        if (thief && quest.thief_calm) || calmed {
            continue;
        }
        if et.translation().distance(p) <= CONTACT_RANGE {
            let factor = if equip.shield {
                skills::train(Skill::Defense, 5.0);
                crate::inventory::SHIELD_FACTOR * (1.0 - 0.1 * skills::rank(Skill::Defense))
            } else {
                1.0
            };
            if equip.armor.is_some() {
                skills::train(Skill::Armor, 4.0);
            }
            hp.current -=
                CONTACT_DAMAGE * factor * equip.armor_factor() * crate::magic::ward_factor();
            combat.iframe = PLAYER_IFRAME;
            cd.left = CONTACT_DAMAGE_COOLDOWN;
            info!("colpito: player vita {:.0}", hp.current);
            break;
        }
    }
}

/// Invulnerabilità (s) dopo essere rinati.
const RESPAWN_IFRAME: f32 = 2.0;

/// Il giocatore è morto?
pub fn is_dead(hp: &PlayerHealth) -> bool {
    hp.current <= 0.0
}

/// Sistema: unico punto di morte del giocatore, qualunque sia la causa (contatto, frecce, caduta).
/// Rinasce allo spawn con la vita piena; inventario e monete restano.
pub fn player_death(
    mut player: Query<
        (
            &mut PlayerHealth,
            &mut CombatState,
            &mut Transform,
            &mut avian3d::prelude::LinearVelocity,
        ),
        With<Player>,
    >,
    mut orbit: ResMut<crate::camera::OrbitCamera>,
    mut session: ResMut<crate::save::Session>,
) {
    let Ok((mut hp, mut combat, mut transform, mut velocity)) = player.single_mut() else {
        return;
    };
    if !is_dead(&hp) {
        return;
    }
    info!(
        "[MORTE] vita {:.0} in {:.1} {:.1} {:.1}: rinascita allo spawn",
        hp.current, transform.translation.x, transform.translation.y, transform.translation.z
    );
    hp.current = hp.max;
    combat.iframe = RESPAWN_IFRAME;
    transform.translation = Vec3::new(0.0, 1.0, 0.0);
    velocity.0 = Vec3::ZERO;
    *orbit = crate::camera::OrbitCamera::default();
    session.notify("Sei svenuto… ti risvegli al villaggio");
}

/// Nemico di un evento (i predoni del carretto, i banditi del ponte): ucciso, non ricompare.
#[derive(Component)]
pub struct NoRespawn;

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
        Has<NoRespawn>,
    ),
    With<Enemy>,
>;

/// Gestione morte nemici: despawn + marker di respawn con i waypoint.
#[allow(clippy::too_many_arguments)]
pub fn handle_enemy_death(
    dbg: Res<crate::debug_place::DebugPlace>,
    mut commands: Commands,
    mut quest: ResMut<crate::quests::denaro_rubato::QuestState>,
    enemies: EnemiesQuery,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    names: Query<&crate::figure::ModelName>,
    character: Res<crate::character::Character>,
) {
    for (entity, hp, patrol, transform, is_thief, once) in &enemies {
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
            let bandit = patrol
                .waypoints
                .first()
                .is_some_and(|w| crate::places::is_camp_patrol(&dbg, w.xz()));
            if is_thief.is_none() {
                // I nemici comuni lasciano qualche moneta; chi ama il coraggio lo apprezza.
                crate::karma::praise("coraggio", 1);
                crate::inventory::spawn_pickup(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    crate::inventory::ItemKind::Coin,
                    character.stats.enemy_coins(),
                    transform.translation() + Vec3::Y * 0.2,
                );
                if once {
                    info!("[NEMICI] nemico di un evento ucciso: non torna");
                } else if bandit {
                    // I banditi tornano di notte (`places::keep_bandits`), non subito.
                    info!("[NEMICI] bandito ucciso: tornerà una notte a caso");
                } else {
                    let wps = patrol.waypoints.clone();
                    commands.spawn((
                        crate::scene::SceneNode,
                        EnemyRespawn {
                            timer: ENEMY_RESPAWN_TIME,
                            waypoints: wps,
                        },
                    ));
                    info!("nemico morto -> respawn in {:.0}s", ENEMY_RESPAWN_TIME);
                }
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

// --- Nemici che attaccano -------------------------------------------------------------------

/// Distanza (m) a cui un nemico nota il giocatore, e a cui lo perde se è già all'inseguimento.
const NOTICE_RANGE: f32 = 14.0;
const LOSE_RANGE: f32 = 22.0;
/// Il nemico tira da lontano fra queste distanze.
const SHOOT_MIN: f32 = 4.5;
const SHOOT_MAX: f32 = 16.0;
pub const ENEMY_ARROW_DAMAGE: f32 = 7.0;
const ENEMY_ARROW_SPEED: f32 = 17.0;

/// Freccia scagliata da un nemico.
#[derive(Component)]
pub struct EnemyArrow {
    pub vel: Vec3,
    life: f32,
}

/// Dista abbastanza e c'è visibilità fra `from` e `to`? `true` se nulla di solido sta in mezzo.
pub fn clear_view(
    spatial: &avian3d::prelude::SpatialQuery,
    solids: &Query<(), ArrowSolids>,
    from: Vec3,
    to: Vec3,
) -> bool {
    let Ok(dir) = Dir3::new(to - from) else {
        return true;
    };
    spatial
        .cast_ray_predicate(
            from,
            dir,
            (to - from).length(),
            true,
            &avian3d::prelude::SpatialQueryFilter::default(),
            &|ent| solids.contains(ent),
        )
        .is_none()
}

/// Sistema: i nemici notano il giocatore (se lo vedono), smettono di pattugliare e attaccano:
/// frecce da lontano, mischia da vicino (il contatto fa il resto). Se il compagno che ti segue è
/// più vicino, tirano a lui, e comunque una freccia su due (ADR 0049).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn enemy_ai(
    mut commands: Commands,
    time: Res<Time>,
    cheats: Res<crate::shell::Cheats>,
    spatial: avian3d::prelude::SpatialQuery,
    solids: Query<(), ArrowSolids>,
    player: Query<&GlobalTransform, (With<Player>, Without<Enemy>)>,
    companions: Query<
        &GlobalTransform,
        (
            With<crate::companions::Companion>,
            Without<Player>,
            Without<Enemy>,
        ),
    >,
    mut enemies: Query<
        (
            &GlobalTransform,
            &mut crate::entities::EnemyMind,
            Has<crate::quests::denaro_rubato::QuestThief>,
            Has<crate::magic::Calmed>,
        ),
        With<Enemy>,
    >,
    quest: Res<crate::quests::denaro_rubato::QuestState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    let Ok(me) = player.single() else {
        return;
    };
    let dt = time.delta_secs();
    let target = me.translation() + Vec3::Y * 0.9;
    let (mesh, mat) = assets.get_or_insert_with(|| {
        (
            meshes.add(Cuboid::new(0.05, 0.05, 0.7)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.55, 0.4, 0.22),
                ..default()
            }),
        )
    });
    for (tf, mut mind, thief, calmed) in &mut enemies {
        mind.shot_cd = (mind.shot_cd - dt).max(0.0);
        // Il ladro non attacca chi gli si avvicina senz'arma in pugno (ADR 0035); né chi è placato
        // dall'incantesimo «Calma».
        let calm = (thief && quest.thief_calm) || calmed;
        let eye = tf.translation() + Vec3::Y * 1.3;
        let dist = (target - eye).length();
        let range = if mind.chase {
            LOSE_RANGE
        } else {
            NOTICE_RANGE * (1.0 - 0.08 * skills::rank(Skill::Stealth))
        };
        let sees = !calm
            && !cheats.invisible
            && dist < range
            && clear_view(&spatial, &solids, eye, target);
        // Passare vicino a un nemico senza farsi notare allena la furtività.
        if !sees && !mind.chase && !calm && !cheats.invisible && dist < NOTICE_RANGE * 1.6 {
            skills::train(Skill::Stealth, dt * 0.5);
        }
        if sees != mind.chase {
            mind.chase = sees;
            info!(
                "[NEMICI] {}",
                if sees {
                    "ha visto il giocatore"
                } else {
                    "lo ha perso"
                }
            );
        }
        // Il bersaglio delle frecce: il giocatore, o il compagno in vista se è più vicino (o una
        // freccia su due).
        let share = mind.shots % 2 == 1;
        let ((target, dist), at_companion) = companions
            .iter()
            .map(|c| c.translation() + Vec3::Y * 0.9)
            .map(|c| (c, (c - eye).length()))
            .filter(|(c, d)| {
                (*d < dist || (share && *d < SHOOT_MAX)) && clear_view(&spatial, &solids, eye, *c)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|_| sees)
            .map(|c| (c, true))
            .unwrap_or(((target, dist), false));
        if sees && mind.shot_cd <= 0.0 && (SHOOT_MIN..SHOOT_MAX).contains(&dist) {
            mind.shot_cd = 2.6;
            mind.shots += 1;
            if at_companion {
                info!("[NEMICI] tira al compagno");
            }
            // Mira un po' avanti, con la caduta compensata.
            let aim = (target - eye).normalize_or_zero() + Vec3::Y * (dist / 120.0);
            commands.spawn((
                crate::scene::SceneNode,
                EnemyArrow {
                    vel: aim.normalize_or_zero() * ENEMY_ARROW_SPEED,
                    life: 2.5,
                },
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(eye).looking_to(aim, Vec3::Y),
            ));
        }
    }
}

/// Sistema: muove le frecce dei nemici e fa male al giocatore.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_enemy_arrows(
    mut commands: Commands,
    time: Res<Time>,
    cheats: Res<crate::shell::Cheats>,
    equip: Res<crate::inventory::Equipment>,
    spatial: avian3d::prelude::SpatialQuery,
    solids: Query<(), ArrowSolids>,
    mut arrows: Query<(Entity, &mut Transform, &mut EnemyArrow)>,
    mut player: Query<
        (&GlobalTransform, &mut PlayerHealth, &mut CombatState),
        (With<Player>, Without<EnemyArrow>),
    >,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut a) in &mut arrows {
        a.life -= dt;
        if a.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        a.vel.y -= ARROW_DROP * dt;
        let step = a.vel * dt;
        let from = tf.translation;
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
        let mid = from + step / 2.0;
        if let Ok((pt, mut hp, mut combat)) = player.single_mut()
            && pt.translation().distance(mid + Vec3::Y * -0.2) < 0.9
        {
            commands.entity(e).despawn();
            if !cheats.god && !cheats.invisible && combat.iframe <= 0.0 && hp.current > 0.0 {
                let factor = if equip.shield {
                    crate::inventory::SHIELD_FACTOR
                } else {
                    1.0
                };
                if equip.armor.is_some() {
                    skills::train(Skill::Armor, 4.0);
                }
                hp.current -= ENEMY_ARROW_DAMAGE
                    * factor
                    * equip.armor_factor()
                    * crate::magic::ward_factor();
                combat.iframe = PLAYER_IFRAME;
                info!("[NEMICI] freccia: vita {:.0}", hp.current);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_badly_hurt_enemy_surrenders_only_with_the_talent() {
        assert!(surrenders(10.0, 50.0, true, false));
        assert!(!surrenders(10.0, 50.0, false, false));
        assert!(
            !surrenders(10.0, 50.0, true, true),
            "il ladro della quest no"
        );
        assert!(!surrenders(30.0, 50.0, true, false));
        assert!(
            !surrenders(0.0, 50.0, true, false),
            "chi è a terra non si arrende"
        );
    }

    #[test]
    fn armor_takes_part_of_the_blow() {
        let mut equip = crate::inventory::Equipment::default();
        assert_eq!(equip.armor_factor(), 1.0);
        equip.armor = Some(crate::inventory::ItemKind::PlateArmor);
        assert!((equip.armor_factor() - 0.55).abs() < 1e-5);
        equip.armor = Some(crate::inventory::ItemKind::LeatherArmor);
        assert!(equip.armor_factor() > 0.8);
    }
}
