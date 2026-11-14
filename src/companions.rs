//! Compagni di strada: Silvia e Guardia Dino (ADR 0049).
//!
//! Chi ha guadagnato la loro amicizia (la Prova dell'Arco o il segreto di Silvia; l'armatura di Dino o
//! il suo segreto) può chiedere nel dialogo «Vieni con me». Il compagno lascia la sua routine, ti segue
//! a qualche passo e colpisce i nemici vicini: Silvia con l'arco, Dino con la spada quando gli sono
//! addosso. «Torna pure a casa» lo rimanda alla sua giornata. Uno alla volta, solo sull'isola.

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

/// Un abitante che ti segue. `cooldown` scandisce i suoi colpi.
#[derive(Component, Default)]
pub struct Companion {
    pub cooldown: f32,
}

/// Richiesta dal dialogo: (abitante, seguire o tornare a casa).
#[derive(Resource, Default)]
pub struct CompanionRequest(pub Option<(Entity, bool)>);

/// Il compagno può venire con te?
pub fn eligible(
    name: &str,
    missions: &crate::quests::missions::Missions,
    karma: &crate::karma::Karma,
) -> bool {
    crate::region::current() == crate::region::Region::Isola
        && CANDIDATES.iter().any(|(who, mission, flag)| {
            *who == name && (missions.done(mission) || karma.unlocks.contains(*flag))
        })
}

/// Sistema: applica le richieste del dialogo (uno alla volta).
pub fn apply_requests(
    mut commands: Commands,
    mut request: ResMut<CompanionRequest>,
    current: Query<Entity, With<Companion>>,
    names: Query<&crate::entities::Npc>,
    mut session: ResMut<crate::save::Session>,
) {
    let Some((entity, follow)) = request.0.take() else {
        return;
    };
    let name = names.get(entity).map_or("", |n| n.name);
    if follow {
        for other in &current {
            commands.entity(other).remove::<Companion>();
        }
        commands.entity(entity).insert(Companion::default());
        session.notify(format!("{name} viene con te"));
        info!("[COMPAGNI] {name} ti segue");
    } else {
        commands.entity(entity).remove::<Companion>();
        session.notify(format!("{name} torna a casa"));
        info!("[COMPAGNI] {name} torna alla sua giornata");
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

pub fn plugin(app: &mut App) {
    app.init_resource::<CompanionRequest>().add_systems(
        Update,
        (apply_requests, companions_fight).run_if(in_state(crate::AppState::Playing)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friends_become_companions_by_mission_or_secret() {
        let missions = crate::quests::missions::Missions::default();
        let mut karma = crate::karma::Karma::default();
        assert!(!eligible("Silvia", &missions, &karma));
        karma.unlocks.insert("silvia-compagna".into());
        assert!(eligible("Silvia", &missions, &karma));
        assert!(!eligible("Guardia Dino", &missions, &karma));
        assert!(!eligible("Nina", &missions, &karma));
        assert!(
            strike("Silvia").2 > strike("Guardia Dino").2,
            "l'arco arriva più lontano"
        );
    }
}
