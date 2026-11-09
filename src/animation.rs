//! Animazioni dei personaggi (ADR 0017): una sola macchina a stati per giocatore, abitanti e nemici.
//!
//! La velocità si misura dallo spostamento del corpo, così vale per tutte le routine senza toccarle. La
//! clip viene accelerata o rallentata in proporzione, per ridurre lo scivolamento dei piedi.

use crate::combat::{EnemyHealth, SwingTimer};
use crate::entities::DialogState;
use crate::figure::{CLIPS, CharacterAnims, CharacterModel};
use crate::inventory::{Equipment, ItemKind};
use crate::player::{GroundedState, Player, PlayerHealth, PlayerVisual};
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use std::time::Duration;

/// Stato di animazione: ciascuno corrisponde a una clip di `CLIPS`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anim {
    Idle,
    Walk,
    Jog,
    Sprint,
    JumpStart,
    JumpLoop,
    JumpLand,
    SwordIdle,
    SwordAttack,
    Punch,
    Hit,
    Talk,
    BowShot,
}

impl Anim {
    fn clip(self) -> &'static str {
        match self {
            Anim::Idle => "Idle_Loop",
            Anim::Walk => "Walk_Loop",
            Anim::Jog => "Jog_Fwd_Loop",
            Anim::Sprint => "Sprint_Loop",
            Anim::JumpStart => "Jump_Start",
            Anim::JumpLoop => "Jump_Loop",
            Anim::JumpLand => "Jump_Land",
            Anim::SwordIdle => "Sword_Idle",
            Anim::SwordAttack => "Sword_Attack",
            Anim::Punch => "Punch_Jab",
            Anim::Hit => "Hit_Chest",
            Anim::Talk => "Idle_Talking_Loop",
            Anim::BowShot => "Spell_Simple_Shoot",
        }
    }

    fn index(self) -> usize {
        CLIPS
            .iter()
            .position(|c| *c == self.clip())
            .expect("clip presente in CLIPS")
    }

    fn looping(self) -> bool {
        matches!(
            self,
            Anim::Idle
                | Anim::Walk
                | Anim::Jog
                | Anim::Sprint
                | Anim::JumpLoop
                | Anim::SwordIdle
                | Anim::Talk
        )
    }

    /// (velocità di riproduzione, durata in secondi prima di poter cambiare stato) delle clip singole.
    fn one_shot(self) -> (f32, f32) {
        match self {
            Anim::SwordAttack => (2.4, 0.62),
            Anim::Punch => (1.6, 0.5),
            Anim::BowShot => (1.6, 0.55),
            Anim::Hit => (0.9, 0.34),
            Anim::JumpStart => (2.6, 0.3),
            Anim::JumpLand => (2.2, 0.25),
            _ => (1.0, 0.0),
        }
    }
}

/// Locomozione alla velocità `speed` (m/s): stato e velocità della clip.
pub fn locomotion(speed: f32, armed: bool) -> (Anim, f32) {
    if speed < 0.25 {
        (if armed { Anim::SwordIdle } else { Anim::Idle }, 1.0)
    } else if speed < 2.6 {
        (Anim::Walk, (speed / 1.35).clamp(0.6, 1.5))
    } else if speed < 7.5 {
        (Anim::Jog, (speed / 3.6).clamp(0.8, 1.8))
    } else {
        (Anim::Sprint, (speed / 6.2).clamp(0.9, 1.7))
    }
}

/// In volo: senza appoggio e con movimento verticale recente. Chi sta fermo su un oggetto
/// senza marker `Ground` (panchine, bancarelle) non resta bloccato nella posa di salto.
pub fn is_flying(grounded: bool, still_seconds: f32) -> bool {
    !grounded && still_seconds < 0.25
}

/// Animazione di attacco secondo l'arma impugnata.
pub fn attack_for(weapon: Option<ItemKind>) -> Anim {
    match weapon {
        None => Anim::Punch,
        Some(ItemKind::Bow) => Anim::BowShot,
        Some(_) => Anim::SwordAttack,
    }
}

/// Stato di animazione di un personaggio (sul corpo, genitore del modello).
#[derive(Component)]
pub struct Animator {
    pub player: Entity,
    pub model: Entity,
    current: Anim,
    lock: f32,
    prev: Option<Vec3>,
    speed: f32,
    air: f32,
    /// Tempo (s) passato senza muoversi in verticale: su oggetti che non sono `Ground` non si è in volo.
    still: f32,
    last_hp: f32,
    last_swing: f32,
    paused: bool,
}

impl Animator {
    pub fn new(player: Entity, model: Entity) -> Self {
        Self {
            player,
            model,
            current: Anim::Idle,
            lock: 0.0,
            prev: None,
            speed: 0.0,
            air: 0.0,
            still: 0.0,
            last_hp: f32::MAX,
            last_swing: 0.0,
            paused: false,
        }
    }
}

/// Oltre questa distanza dalla camera (m) le animazioni si possono fermare.
const FAR_ANIM: f32 = 60.0;
/// Distanza (m) entro cui un nemico colpisce il giocatore.
const ENEMY_REACH: f32 = 1.8;

/// Sistema: sceglie e avvia l'animazione di ogni personaggio.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn drive_animations(
    time: Res<Time>,
    anims: Option<Res<CharacterAnims>>,
    settings: Res<crate::audio::AudioSettings>,
    swing: Res<SwingTimer>,
    equipment: Res<Equipment>,
    dialog: Res<DialogState>,
    mut bodies: Query<(Entity, &mut Animator, &GlobalTransform)>,
    visuals: Query<(), With<PlayerVisual>>,
    players: Query<
        (
            &GroundedState,
            &LinearVelocity,
            &PlayerHealth,
            &GlobalTransform,
        ),
        With<Player>,
    >,
    enemies: Query<&EnemyHealth>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut animation: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    mut models: Query<&mut Transform, With<CharacterModel>>,
) {
    let Some(anims) = anims else {
        return;
    };
    let dt = time.delta_secs().max(1e-4);
    let cam = cameras
        .single()
        .map(|c| c.translation())
        .unwrap_or_default();
    let player_state = players.single().ok();
    for (entity, mut a, global) in &mut bodies {
        let pos = global.translation();
        // Velocità orizzontale misurata e filtrata; i teletrasporti (comparse, reset) non contano.
        let delta = a.prev.map_or(Vec3::ZERO, |p| pos - p);
        a.prev = Some(pos);
        let flat = Vec2::new(delta.x, delta.z);
        let inst = (flat.length() / dt).min(15.0);
        let inst = if flat.length() > 2.0 { 0.0 } else { inst };
        let k = (dt * 12.0).min(1.0);
        a.speed += (inst - a.speed) * k;
        a.lock = (a.lock - dt).max(0.0);

        let Ok((mut player, mut transitions)) = animation.get_mut(a.player) else {
            continue;
        };

        // Personaggi lontani: animazione ferma (solo aspetto visivo).
        let far = settings.gfx.far_anim_cull && pos.distance(cam) > FAR_ANIM;
        if far != a.paused {
            a.paused = far;
            if far {
                player.pause_all();
            } else {
                player.resume_all();
            }
        }
        if far {
            continue;
        }

        let is_player = visuals.contains(entity);
        let enemy = enemies.get(entity).ok();
        let mut want = locomotion(a.speed, is_player && equipment.weapon.is_some());
        let mut one_shot = None;

        if let Some(e) = enemy {
            if e.current < a.last_hp && a.last_hp != f32::MAX {
                one_shot = Some(Anim::Hit);
            }
            a.last_hp = e.current;
            // I nemici guardano dove vanno, e colpiscono se il giocatore è a portata.
            if flat.length() > 1e-3
                && let Ok(mut model) = models.get_mut(a.model)
            {
                let yaw = flat.x.atan2(flat.y);
                model.rotation = model
                    .rotation
                    .slerp(Quat::from_rotation_y(yaw), (dt * 10.0).min(1.0));
            }
            if let Some((_, _, _, p)) = player_state
                && p.translation().distance(pos) < ENEMY_REACH
                && one_shot.is_none()
            {
                one_shot = Some(Anim::Punch);
            }
        }

        if is_player && let Some((grounded, velocity, hp, _)) = player_state {
            if hp.current < a.last_hp && a.last_hp != f32::MAX {
                one_shot = Some(Anim::Hit);
            }
            a.last_hp = hp.current;
            if swing.0 > a.last_swing {
                one_shot = Some(attack_for(equipment.weapon));
            }
            a.last_swing = swing.0;

            let vy = delta.y / dt;
            a.still = if vy.abs() < 0.4 { a.still + dt } else { 0.0 };
            let flying = is_flying(grounded.grounded_now, a.still);
            let was_air = a.air;
            a.air = if flying { a.air + dt } else { 0.0 };
            if flying && a.air > 0.1 {
                want = if velocity.y > 1.5 && was_air < 0.25 && one_shot.is_none() {
                    (Anim::JumpStart, 1.0)
                } else {
                    (Anim::JumpLoop, 1.0)
                };
            } else if !flying && was_air > 0.3 && one_shot.is_none() && a.speed < 1.0 {
                one_shot = Some(Anim::JumpLand);
            }
        }

        if dialog.open && dialog.speaker_entity == Some(entity) && a.speed < 0.25 {
            want = (Anim::Talk, 1.0);
        }

        // Le clip singole hanno precedenza finché non finiscono.
        let (next, clip_speed) =
            if let Some(shot) = one_shot.filter(|s| a.lock <= 0.0 || *s == Anim::Hit) {
                let (speed, lock) = shot.one_shot();
                a.lock = lock;
                (shot, speed)
            } else if a.lock > 0.0 {
                continue;
            } else if !want.0.looping() {
                let (speed, lock) = want.0.one_shot();
                a.lock = lock;
                (want.0, speed)
            } else {
                want
            };

        let node = anims.clip(next.index());
        if next != a.current || one_shot == Some(next) {
            let active = transitions.play(&mut player, node, Duration::from_millis(180));
            active.set_speed(clip_speed);
            if next.looping() {
                active.repeat();
            } else {
                active.replay();
            }
            a.current = next;
        } else if let Some(active) = player.animation_mut(node) {
            active.set_speed(clip_speed);
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, drive_animations);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_state_has_a_clip() {
        for a in [
            Anim::Idle,
            Anim::Walk,
            Anim::Jog,
            Anim::Sprint,
            Anim::JumpStart,
            Anim::JumpLoop,
            Anim::JumpLand,
            Anim::SwordIdle,
            Anim::SwordAttack,
            Anim::Punch,
            Anim::Hit,
            Anim::Talk,
            Anim::BowShot,
        ] {
            assert!(a.index() < CLIPS.len());
        }
    }

    #[test]
    fn locomotion_follows_speed() {
        assert_eq!(locomotion(0.0, false).0, Anim::Idle);
        assert_eq!(locomotion(0.0, true).0, Anim::SwordIdle);
        assert_eq!(locomotion(1.3, false).0, Anim::Walk);
        assert_eq!(locomotion(6.0, false).0, Anim::Jog);
        assert_eq!(locomotion(10.0, false).0, Anim::Sprint);
        // La clip della camminata degli NPC va quasi a velocità naturale.
        assert!((locomotion(1.3, false).1 - 0.96).abs() < 0.05);
        // Più si va veloci, più la clip accelera.
        assert!(locomotion(6.0, false).1 > locomotion(3.0, false).1);
    }

    #[test]
    fn attack_depends_on_the_weapon() {
        assert_eq!(attack_for(None), Anim::Punch);
        assert_eq!(attack_for(Some(ItemKind::Bow)), Anim::BowShot);
        assert_eq!(attack_for(Some(ItemKind::Axe)), Anim::SwordAttack);
    }

    #[test]
    fn standing_on_props_is_not_flying() {
        assert!(is_flying(false, 0.0));
        assert!(!is_flying(true, 0.0));
        assert!(!is_flying(false, 1.0));
    }
}
