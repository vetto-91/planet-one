//! Compagni di strada: Silvia e Guardia Dino (ADR 0049).
//!
//! Chi ha guadagnato la loro amicizia (la Prova dell'Arco o il segreto di Silvia; l'armatura di Dino o
//! il suo segreto) può chiedere nel dialogo «Vieni con me». Il compagno lascia la sua routine, ti segue
//! a qualche passo e colpisce i nemici vicini: Silvia con l'arco, Dino con la spada quando gli sono
//! addosso. «Torna pure a casa» lo rimanda alla sua giornata. Uno alla volta, solo sull'isola.
//!
//! I nemici vicini e le loro frecce feriscono anche il compagno. Con poca vita lo dice; senza più
//! forze si ritira a casa e fino al giorno dopo non viene. Lontano dai colpi si riprende. Chi ti
//! segue, la sua vita e chi è ferito restano nel salvataggio (`[compagni]`).

use bevy::prelude::*;

/// I compagni possibili e chi li sblocca (missione compiuta o segreto confidato).
pub const CANDIDATES: [(&str, &str, &str); 2] = [
    ("Silvia", "qs14", "silvia-compagna"),
    ("Guardia Dino", "qs09", "dino-alleato"),
];

/// Distanza a cui il compagno si tiene, e oltre la quale ti raggiunge correndo o di colpo.
pub const KEEP: f32 = 2.4;
pub const RUN_SPEED: f32 = 4.6;
pub const CATCH_UP: f32 = 35.0;

/// Vita piena di un compagno; sotto `ALARM` lo dice (una volta, finché non si riprende).
pub const MAX_HP: f32 = 100.0;
const ALARM: f32 = 35.0;
/// Secondi senza colpi prima di riprendersi, e vita ripresa al secondo.
const REST_AFTER: f32 = 8.0;
const REGEN: f32 = 3.0;

/// Un abitante che ti segue. `cooldown` scandisce i suoi colpi, `hurt_cd` quelli che prende.
#[derive(Component)]
pub struct Companion {
    pub cooldown: f32,
    pub hp: f32,
    pub hurt_cd: f32,
    /// Secondi dall'ultimo colpo preso.
    pub quiet: f32,
    pub warned: bool,
}

impl Default for Companion {
    fn default() -> Self {
        Self::with_hp(MAX_HP)
    }
}

impl Companion {
    pub fn with_hp(hp: f32) -> Self {
        Self {
            cooldown: 0.0,
            hp: hp.clamp(1.0, MAX_HP),
            hurt_cd: 0.0,
            quiet: REST_AFTER,
            warned: false,
        }
    }

    /// Prende un colpo (già ridotto dall'armatura). Restituisce la battuta da dire, se c'è.
    pub fn hurt(&mut self, damage: f32) -> Option<&'static str> {
        self.hp = (self.hp - damage).max(0.0);
        self.quiet = 0.0;
        if self.hp <= 0.0 {
            Some("ferita")
        } else if self.hp < ALARM && !self.warned {
            self.warned = true;
            Some("allarme")
        } else {
            None
        }
    }

    /// Lontano dai colpi si riprende.
    pub fn rest(&mut self, dt: f32) {
        self.quiet += dt;
        if self.quiet >= REST_AFTER {
            self.hp = (self.hp + REGEN * dt).min(MAX_HP);
        }
        if self.hp > ALARM * 1.7 {
            self.warned = false;
        }
    }
}

/// Quanto del danno arriva al compagno: Dino porta la cotta di maglia della guardia.
pub fn toughness(name: &str) -> f32 {
    if name == "Silvia" { 1.0 } else { 0.7 }
}

/// Stato condiviso: richiesta dal dialogo, chi è ferito (e da che giorno), compagno da ripristinare
/// dopo un caricamento (`Some(None)` = nessuno).
#[derive(Resource, Default)]
pub struct CompanionState {
    pub request: Option<(Entity, bool)>,
    pub wounded: Vec<(String, u32)>,
    pub restore: Option<Option<(String, f32)>>,
    /// Colpo di prova dalla shell (`ferisci <danno>`), già senza armatura.
    pub hurt: f32,
}

impl CompanionState {
    /// Ferito, a casa fino a domani?
    pub fn resting(&self, name: &str) -> bool {
        self.wounded.iter().any(|(n, _)| n == name)
    }
}

/// Le battute di `assets/world/compagni.txt`: (compagno, chiave, battuta).
pub fn lines() -> &'static [(String, String, String)] {
    static LINES: std::sync::OnceLock<Vec<(String, String, String)>> = std::sync::OnceLock::new();
    LINES.get_or_init(|| parse_lines(include_str!("../assets/world/compagni.txt")))
}

pub fn parse_lines(text: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut who = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            who = name.trim().to_string();
        } else if let Some((key, said)) = line.split_once(':')
            && !who.is_empty()
        {
            out.push((who.clone(), key.trim().to_string(), said.trim().to_string()));
        }
    }
    out
}

/// La battuta di un compagno per un momento (`segui`, `casa`, `allarme`, `ferita`).
pub fn line(name: &str, key: &str) -> Option<&'static str> {
    lines()
        .iter()
        .find(|(n, k, _)| n == name && k == key)
        .map(|(_, _, s)| s.as_str())
}

/// Il compagno dice la sua battuta: fumetto sopra la testa e voce (se generata).
fn say(
    barks: &mut crate::quests::Barks,
    voice: &mut crate::voice::VoiceQueue,
    entity: Entity,
    name: &str,
    key: &str,
) {
    let Some(text) = line(name, key) else {
        return;
    };
    barks.say(entity, name, text.to_string());
    let clips = crate::voice::line_clip(name, text);
    if !clips.is_empty() {
        voice.play_from(clips, crate::voice::VoiceKind::Bark, Some(entity));
    }
}

/// Distanza di `p` dal segmento `a`–`b`.
pub fn near_segment(a: Vec3, b: Vec3, p: Vec3) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t).distance(p)
}

/// Il compagno può venire con te?
pub fn eligible(
    name: &str,
    missions: &crate::quests::missions::Missions,
    karma: &crate::karma::Karma,
    state: &CompanionState,
) -> bool {
    crate::region::current() == crate::region::Region::Isola
        && !state.resting(name)
        && CANDIDATES.iter().any(|(who, mission, flag)| {
            *who == name && (missions.done(mission) || karma.unlocks.contains(*flag))
        })
}

/// Sistema: applica le richieste del dialogo (uno alla volta).
pub fn apply_requests(
    mut commands: Commands,
    mut state: ResMut<CompanionState>,
    current: Query<Entity, With<Companion>>,
    names: Query<&crate::entities::Npc>,
    mut session: ResMut<crate::save::Session>,
    mut barks: ResMut<crate::quests::Barks>,
    mut voice: ResMut<crate::voice::VoiceQueue>,
) {
    let Some((entity, follow)) = state.request.take() else {
        return;
    };
    let name = names.get(entity).map_or("", |n| n.name);
    if follow {
        for other in &current {
            commands.entity(other).remove::<Companion>();
        }
        commands.entity(entity).insert(Companion::default());
        session.notify(format!("{name} viene con te"));
        say(&mut barks, &mut voice, entity, name, "segui");
        info!("[COMPAGNI] {name} ti segue");
    } else {
        commands.entity(entity).remove::<Companion>();
        session.notify(format!("{name} torna a casa"));
        say(&mut barks, &mut voice, entity, name, "casa");
        info!("[COMPAGNI] {name} torna alla sua giornata");
    }
}

/// Sistema: dopo un caricamento rimette il compagno salvato (aspetta che l'abitante ci sia), e
/// toglie chi seguiva prima; a ogni giorno nuovo i feriti tornano disponibili.
pub fn restore_and_heal(
    mut commands: Commands,
    mut state: ResMut<CompanionState>,
    clock: Res<crate::daynight::GameClock>,
    current: Query<Entity, With<Companion>>,
    npcs: Query<(Entity, &crate::entities::Npc)>,
) {
    let today = clock.date.day_key();
    if state.wounded.iter().any(|(_, day)| *day < today) {
        for (name, _) in state.wounded.iter().filter(|(_, day)| *day < today) {
            info!("[COMPAGNI] {name} si è ripreso dalle ferite");
        }
        state.wounded.retain(|(_, day)| *day >= today);
    }
    let Some(want) = state.restore.clone() else {
        return;
    };
    for e in &current {
        commands.entity(e).remove::<Companion>();
    }
    match want {
        None => state.restore = None,
        Some((name, hp)) => {
            if crate::region::current() != crate::region::Region::Isola {
                state.restore = None;
            } else if let Some((e, _)) = npcs.iter().find(|(_, n)| n.name == name) {
                commands.entity(e).insert(Companion::with_hp(hp));
                info!("[COMPAGNI] {name} ti segue ancora (vita {hp:.0})");
                state.restore = None;
            }
        }
    }
}

/// Danno e cadenza dei colpi di un compagno: (danno, secondi, portata).
pub fn strike(name: &str) -> (f32, f32, f32) {
    if name == "Silvia" {
        (9.0, 1.6, 14.0)
    } else {
        (11.0, 1.2, 2.6)
    }
}

/// Sistema: i compagni colpiscono il nemico vivo più vicino a portata.
pub fn companions_fight(
    time: Res<Time>,
    mut companions: Query<(&GlobalTransform, &crate::entities::Npc, &mut Companion)>,
    mut enemies: Query<
        (&GlobalTransform, &mut crate::combat::EnemyHealth),
        With<crate::entities::Enemy>,
    >,
) {
    for (t, npc, mut c) in &mut companions {
        c.cooldown = (c.cooldown - time.delta_secs()).max(0.0);
        if c.cooldown > 0.0 {
            continue;
        }
        let (damage, every, reach) = strike(npc.name);
        let here = t.translation();
        let target = enemies
            .iter_mut()
            .filter(|(et, hp)| hp.current > 0.0 && et.translation().distance(here) <= reach)
            .min_by(|a, b| {
                a.0.translation()
                    .distance(here)
                    .total_cmp(&b.0.translation().distance(here))
            });
        if let Some((_, mut hp)) = target {
            hp.current = (hp.current - damage).max(0.0);
            c.cooldown = every;
            info!(
                "[COMPAGNI] {} colpisce: vita del nemico {:.0}",
                npc.name, hp.current
            );
        }
    }
}

/// Sistema: i nemici vicini e le loro frecce feriscono il compagno; senza forze si ritira a casa
/// fino al giorno dopo.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn companions_hurt(
    mut commands: Commands,
    time: Res<Time>,
    clock: Res<crate::daynight::GameClock>,
    quest: Res<crate::quests::denaro_rubato::QuestState>,
    mut state: ResMut<CompanionState>,
    mut session: ResMut<crate::save::Session>,
    mut barks: ResMut<crate::quests::Barks>,
    mut voice: ResMut<crate::voice::VoiceQueue>,
    mut companions: Query<(
        Entity,
        &GlobalTransform,
        &crate::entities::Npc,
        &mut Companion,
    )>,
    enemies: Query<
        (
            &GlobalTransform,
            &crate::combat::EnemyHealth,
            Has<crate::magic::Calmed>,
            Has<crate::quests::denaro_rubato::QuestThief>,
        ),
        With<crate::entities::Enemy>,
    >,
    arrows: Query<(Entity, &Transform, &crate::combat::EnemyArrow)>,
) {
    let dt = time.delta_secs();
    for (entity, t, npc, mut c) in &mut companions {
        let here = t.translation();
        let body = here + Vec3::Y * 0.9;
        let mut damage = 0.0;
        c.hurt_cd = (c.hurt_cd - dt).max(0.0);
        if c.hurt_cd <= 0.0
            && enemies.iter().any(|(et, hp, calmed, thief)| {
                hp.current > 0.0
                    && !calmed
                    && !(thief && quest.thief_calm)
                    && crate::combat::in_contact(et.translation(), here)
            })
        {
            damage += crate::combat::CONTACT_DAMAGE;
            c.hurt_cd = crate::combat::CONTACT_DAMAGE_COOLDOWN;
        }
        for (arrow, at, a) in &arrows {
            // Il tratto percorso nell'ultimo fotogramma: una freccia può saltare oltre il corpo.
            let to = at.translation;
            let from = to - a.vel * dt;
            if near_segment(from, to, body) < 0.8 {
                commands.entity(arrow).despawn();
                damage += crate::combat::ENEMY_ARROW_DAMAGE;
            }
        }
        let test = std::mem::take(&mut state.hurt);
        if damage <= 0.0 && test <= 0.0 {
            c.rest(dt);
            continue;
        }
        let said = c.hurt(damage * toughness(npc.name) + test);
        info!("[COMPAGNI] {} colpito: vita {:.0}", npc.name, c.hp);
        if let Some(key) = said {
            say(&mut barks, &mut voice, entity, npc.name, key);
        }
        if c.hp <= 0.0 {
            commands.entity(entity).remove::<Companion>();
            state
                .wounded
                .push((npc.name.to_string(), clock.date.day_key()));
            let hurt = if npc.name == "Silvia" {
                "ferita"
            } else {
                "ferito"
            };
            session.notify(format!(
                "{} è {hurt} e torna a casa: domani starà meglio",
                npc.name
            ));
            info!("[COMPAGNI] {} si ritira ferito", npc.name);
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<CompanionState>().add_systems(
        Update,
        (
            apply_requests,
            restore_and_heal,
            companions_fight,
            companions_hurt,
        )
            .run_if(in_state(crate::AppState::Playing)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friends_become_companions_by_mission_or_secret() {
        let missions = crate::quests::missions::Missions::default();
        let mut karma = crate::karma::Karma::default();
        let mut state = CompanionState::default();
        assert!(!eligible("Silvia", &missions, &karma, &state));
        karma.unlocks.insert("silvia-compagna".into());
        assert!(eligible("Silvia", &missions, &karma, &state));
        assert!(!eligible("Guardia Dino", &missions, &karma, &state));
        assert!(!eligible("Nina", &missions, &karma, &state));
        // Ferita, fino a domani non viene.
        state.wounded.push(("Silvia".into(), 7));
        assert!(!eligible("Silvia", &missions, &karma, &state));
        assert!(
            strike("Silvia").2 > strike("Guardia Dino").2,
            "l'arco arriva più lontano"
        );
    }

    #[test]
    fn a_companion_warns_once_retreats_at_zero_and_recovers_away_from_blows() {
        let mut c = Companion::default();
        assert_eq!(c.hurt(30.0), None);
        assert_eq!(c.hurt(40.0), Some("allarme"));
        assert_eq!(c.hurt(5.0), None, "l'allarme si dice una volta");
        // Subito dopo un colpo non si riprende; dopo qualche secondo sì.
        c.rest(1.0);
        assert_eq!(c.hp, 25.0);
        c.rest(REST_AFTER);
        assert!(c.hp > 25.0);
        assert_eq!(c.hurt(200.0), Some("ferita"));
        assert_eq!(c.hp, 0.0);
        assert!(toughness("Guardia Dino") < toughness("Silvia"));
        // Una freccia che passa in mezzo fra due fotogrammi colpisce lo stesso.
        let body = Vec3::new(0.0, 0.9, 0.0);
        assert!(near_segment(Vec3::new(-0.6, 0.9, 0.1), Vec3::new(0.6, 0.9, 0.1), body) < 0.2);
        assert!(near_segment(Vec3::new(2.0, 0.9, 0.0), Vec3::new(3.0, 0.9, 0.0), body) > 1.9);
        // Ripristinato da un salvataggio con la sua vita (mai a zero).
        assert_eq!(Companion::with_hp(0.0).hp, 1.0);
        assert_eq!(Companion::with_hp(64.0).hp, 64.0);
    }

    #[test]
    fn every_companion_has_its_four_lines() {
        for (name, ..) in CANDIDATES {
            for key in ["segui", "casa", "allarme", "ferita"] {
                assert!(line(name, key).is_some(), "{name}: manca «{key}»");
            }
        }
        let parsed = parse_lines("# nota\n[Nina]\nsegui: Andiamo.\n");
        assert_eq!(
            parsed,
            vec![("Nina".into(), "segui".into(), "Andiamo.".into())]
        );
    }
}
