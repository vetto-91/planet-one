//! Incantesimi dell'«Accordo» (ADR 0041).
//!
//! Cinque incantesimi che spendono la riserva di magia ([`crate::vitals::Vitals`]) e allenano
//! l'abilità Magie: si imparano col grado, si scelgono con `Z` e si lanciano con `Q`.
//!
//! - **Sfera di luce**: un lume che segue il personaggio.
//! - **Dardo di Luce Viva**: un colpo luminoso verso il mirino.
//! - **Soffio di cura**: ridà vita.
//! - **Passo leggero**: per due minuti si portano 15 kg in più.
//! - **Scudo etereo**: per venti secondi i colpi subiti fanno metà danno.

use std::sync::atomic::{AtomicBool, Ordering};

use bevy::prelude::*;

use crate::character::Stat;
use crate::skills::{self, Skill};

/// Il laghetto sotterraneo della grotta (il Cenote), dove le stele risuonano con la magia.
pub const CENOTE: Vec2 = Vec2::new(96.5, 33.2);
const CENOTE_REACH: f32 = 4.5;
/// Pausa (s) fra un incantesimo e l'altro.
const CAST_COOLDOWN: f32 = 0.8;
const DART_SPEED: f32 = 30.0;
const DART_DAMAGE: f32 = 14.0;
const DART_LIFE: f32 = 2.5;
const HEAL_AMOUNT: f32 = 25.0;
const LIGHT_SECS: f32 = 120.0;
const WARD_SECS: f32 = 20.0;
/// Danno che passa attraverso lo scudo etereo.
pub const WARD_FACTOR: f32 = 0.5;
const FEATHER_KG: f32 = 15.0;
const FEATHER_SECS: f32 = 120.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spell {
    Light,
    Dart,
    Heal,
    Feather,
    Ward,
    /// I nemici vicini smettono di cercarti (ADR 0049).
    Calm,
}

impl Spell {
    pub const ALL: [Spell; 6] = [
        Spell::Light,
        Spell::Dart,
        Spell::Heal,
        Spell::Feather,
        Spell::Ward,
        Spell::Calm,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Spell::Light => "Sfera di luce",
            Spell::Dart => "Dardo di Luce Viva",
            Spell::Heal => "Soffio di cura",
            Spell::Feather => "Passo leggero",
            Spell::Ward => "Scudo etereo",
            Spell::Calm => "Calma",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Spell::Light => "luce",
            Spell::Dart => "dardo",
            Spell::Heal => "cura",
            Spell::Feather => "passo",
            Spell::Ward => "scudo",
            Spell::Calm => "calma",
        }
    }

    pub fn parse(text: &str) -> Option<Spell> {
        let wanted = crate::inventory_ui::normalize(text);
        Spell::ALL.into_iter().find(|s| {
            s.key() == wanted || crate::inventory_ui::normalize(s.name()).starts_with(&wanted)
        })
    }

    pub fn effect(self) -> &'static str {
        match self {
            Spell::Light => "un lume ti segue per due minuti",
            Spell::Dart => "un colpo di luce verso il mirino",
            Spell::Heal => "ridà 25 punti vita",
            Spell::Feather => "porti 15 kg in più per due minuti",
            Spell::Ward => "per venti secondi subisci metà danno",
            Spell::Calm => "i nemici vicini smettono di cercarti per trenta secondi",
        }
    }

    /// Magia spesa, prima degli sconti del talento.
    pub fn cost(self) -> f32 {
        match self {
            Spell::Light => 8.0,
            Spell::Dart => 12.0,
            Spell::Heal => 20.0,
            Spell::Feather => 18.0,
            Spell::Ward => 25.0,
            Spell::Calm => 22.0,
        }
    }

    /// Grado di Magie a cui lo si impara.
    pub fn grade(self) -> u8 {
        match self {
            Spell::Light | Spell::Dart => 1,
            Spell::Heal => 5,
            Spell::Feather => 10,
            Spell::Ward => 15,
            Spell::Calm => 20,
        }
    }

    /// Sfera di luce e dardo si conoscono da sé; gli altri li insegna Agnese (ADR 0049).
    pub fn basic(self) -> bool {
        matches!(self, Spell::Light | Spell::Dart)
    }
}

static TAUGHT: AtomicBool = AtomicBool::new(false);

/// Agnese ha insegnato l'Accordo (missione `qs18`)?
pub fn taught() -> bool {
    TAUGHT.load(Ordering::Relaxed)
}

/// Sistema: legge dalle missioni se l'Accordo è stato insegnato.
pub fn sync_taught(missions: Res<crate::quests::missions::Missions>) {
    TAUGHT.store(
        !missions.enabled || missions.unlocked("accordo"),
        Ordering::Relaxed,
    );
}

/// Secondi di calma e raggio dell'incantesimo.
const CALM_SECS: f32 = 30.0;
const CALM_RANGE: f32 = 14.0;

/// Un nemico placato dall'incantesimo «Calma»: non ti cerca e non ti tocca.
#[derive(Component)]
pub struct Calmed(pub f32);

/// Costo vero: ogni rango del talento «Maestro dell'Accordo» toglie l'8%.
pub fn cost(spell: Spell, perk: f32) -> f32 {
    spell.cost() * (1.0 - 0.08 * perk).max(0.5)
}

/// Incantesimi conosciuti a questo grado di Magie.
pub fn known(grade: u8) -> Vec<Spell> {
    known_with(grade, taught())
}

/// Incantesimi conosciuti: col grado, e (quelli avanzati) solo dopo l'insegnamento di Agnese.
pub fn known_with(grade: u8, taught: bool) -> Vec<Spell> {
    Spell::ALL
        .into_iter()
        .filter(|s| grade >= s.grade() && (s.basic() || taught))
        .collect()
}

/// Il prossimo incantesimo conosciuto dopo `current` (tasto Z).
pub fn next(current: Spell, grade: u8) -> Spell {
    next_with(current, grade, taught())
}

fn next_with(current: Spell, grade: u8, taught: bool) -> Spell {
    let list = known_with(grade, taught);
    let at = list.iter().position(|s| *s == current).unwrap_or(0);
    list.get((at + 1) % list.len().max(1))
        .copied()
        .unwrap_or(Spell::Light)
}

/// Danno del dardo: cresce con l'Intelligenza e con il talento.
pub fn dart_damage(intelligence: u8, perk: f32) -> f32 {
    DART_DAMAGE * (1.0 + 0.04 * (f32::from(intelligence) - 10.0)).max(0.5) * (1.0 + 0.08 * perk)
}

/// Incantesimo scelto ed effetti in corso.
#[derive(Resource, Debug)]
pub struct Magic {
    pub selected: Spell,
    cooldown: f32,
    /// Secondi rimasti di scudo etereo e di sfera di luce.
    pub ward: f32,
    pub light: f32,
    /// Incantesimo chiesto dalla shell (lanciato al prossimo frame).
    pub request: Option<Spell>,
    /// Dove è stato lanciato «Calma» (lo applica `apply_calm`).
    calm_at: Option<Vec3>,
}

impl Default for Magic {
    fn default() -> Self {
        Self {
            selected: Spell::Light,
            cooldown: 0.0,
            ward: 0.0,
            light: 0.0,
            request: None,
            calm_at: None,
        }
    }
}

static WARDED: AtomicBool = AtomicBool::new(false);

/// Fattore sul danno subito: lo scudo etereo lo dimezza.
pub fn ward_factor() -> f32 {
    if WARDED.load(Ordering::Relaxed) {
        WARD_FACTOR
    } else {
        1.0
    }
}

/// Dardo di luce in volo.
#[derive(Component)]
pub struct Dart {
    vel: Vec3,
    life: f32,
    damage: f32,
}

/// Lume che segue il personaggio.
#[derive(Component)]
pub struct Orb;

/// Sistema: sceglie (Z) e lancia (Q) gli incantesimi.
#[allow(clippy::too_many_arguments)]
pub fn cast(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    dialog: Res<crate::entities::DialogState>,
    progress: Res<skills::Progress>,
    character: Res<crate::character::Character>,
    cheats: Res<crate::shell::Cheats>,
    mut magic: ResMut<Magic>,
    mut vitals: ResMut<crate::vitals::Vitals>,
    mut effects: ResMut<crate::character::ActiveEffects>,
    mut session: ResMut<crate::save::Session>,
    mut players: Query<
        (&GlobalTransform, &mut crate::player::PlayerHealth),
        With<crate::player::Player>,
    >,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    orbs: Query<Entity, With<Orb>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    magic.cooldown = (magic.cooldown - dt).max(0.0);
    magic.ward = (magic.ward - dt).max(0.0);
    magic.light = (magic.light - dt).max(0.0);
    WARDED.store(magic.ward > 0.0, Ordering::Relaxed);
    if magic.light <= 0.0 {
        for orb in &orbs {
            commands.entity(orb).despawn();
        }
    }
    let grade = progress.grade(Skill::Magic);
    // Un incantesimo non più conosciuto (grado abbassato dalla shell) torna al primo.
    if !known(grade).contains(&magic.selected) {
        magic.selected = Spell::Light;
    }
    if dialog.open {
        return;
    }
    if keys.just_pressed(KeyCode::KeyZ) {
        magic.selected = next(magic.selected, grade);
        session.notify(format!("Incantesimo: {}", magic.selected.name()));
    }
    let asked = magic.request.take();
    let spell = match asked {
        Some(spell) => spell,
        None if keys.just_pressed(KeyCode::KeyQ) => magic.selected,
        None => return,
    };
    if magic.cooldown > 0.0 {
        return;
    }
    if !known(grade).contains(&spell) {
        session.notify(format!(
            "{} si impara al grado {} di Magie{}",
            spell.name(),
            spell.grade(),
            if spell.basic() {
                ""
            } else {
                ", e te lo deve insegnare Agnese"
            }
        ));
        return;
    }
    let Ok((player, mut hp)) = players.single_mut() else {
        return;
    };
    let perk = skills::rank(Skill::Magic);
    let price = cost(spell, perk);
    if vitals.magic < price && !cheats.god {
        session.notify("Non hai abbastanza magia");
        return;
    }
    let p = player.translation();
    match spell {
        Spell::Light => {
            magic.light = LIGHT_SECS;
            if orbs.is_empty() {
                commands.spawn((
                    crate::scene::SceneNode,
                    Orb,
                    Mesh3d(meshes.add(Sphere::new(0.09))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 0.95, 0.75),
                        emissive: LinearRgba::rgb(40.0, 34.0, 20.0),
                        unlit: true,
                        ..default()
                    })),
                    PointLight {
                        intensity: 420_000.0,
                        range: 22.0,
                        color: Color::srgb(1.0, 0.93, 0.78),
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::from_translation(p + Vec3::new(0.5, 1.5, 0.0)),
                ));
            }
        }
        Spell::Dart => {
            let dir = cameras.single().map_or(Dir3::NEG_Z, |c| c.forward());
            commands.spawn((
                crate::scene::SceneNode,
                Dart {
                    vel: *dir * DART_SPEED,
                    life: DART_LIFE,
                    damage: dart_damage(character.stats.get(Stat::Intelligence), perk),
                },
                Mesh3d(meshes.add(Sphere::new(0.12))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.7, 0.9, 1.0),
                    emissive: LinearRgba::rgb(12.0, 26.0, 44.0),
                    unlit: true,
                    ..default()
                })),
                PointLight {
                    intensity: 160_000.0,
                    range: 10.0,
                    color: Color::srgb(0.6, 0.85, 1.0),
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_translation(p + Vec3::Y * 0.5 + *dir * 0.8),
            ));
        }
        Spell::Heal => {
            if hp.current >= hp.max {
                session.notify("La vita è già piena");
                return;
            }
            let wisdom = f32::from(character.stats.get(Stat::Wisdom)) - 10.0;
            let amount = HEAL_AMOUNT * (1.0 + 0.04 * wisdom).max(0.5);
            hp.current = (hp.current + amount).min(hp.max);
        }
        Spell::Feather => effects.add(crate::character::Effect {
            source: Spell::Feather.name().to_string(),
            carry_kg: FEATHER_KG,
            remaining: Some(FEATHER_SECS),
        }),
        Spell::Ward => magic.ward = WARD_SECS,
        Spell::Calm => magic.calm_at = Some(p),
    }
    if !cheats.god {
        vitals.magic = (vitals.magic - price).max(0.0);
    }
    magic.cooldown = CAST_COOLDOWN;
    crate::sfx::play(crate::sfx::Sfx::Spell);
    // Nel laghetto sotto la grotta la magia fa risuonare le quattro stele (MP-04, ADR 0048).
    if p.xz().distance(CENOTE) <= CENOTE_REACH && p.y < 2.2 {
        crate::karma::flag("stele-risonanti");
        session.notify("Le quattro stele rispondono: l'acqua del laghetto vibra");
    }
    skills::train(Skill::Magic, spell.cost() * 0.5);
    session.notify(spell.name());
    info!(
        "[MAGIA] {} (magia {:.0}/{:.0})",
        spell.name(),
        vitals.magic,
        vitals.magic_max
    );
}

/// Sistema: il lume segue il personaggio, appena sopra la spalla.
pub fn follow_orb(
    time: Res<Time>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    mut orbs: Query<&mut Transform, With<Orb>>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let target = player.translation() + Vec3::new(0.5, 1.5, 0.0);
    for mut tf in &mut orbs {
        let bob = (time.elapsed_secs() * 1.7).sin() * 0.06;
        let k = (6.0 * time.delta_secs()).clamp(0.0, 1.0);
        tf.translation = tf.translation.lerp(target + Vec3::Y * bob, k);
    }
}

/// Sistema: i dardi volano dritti, si spengono contro il mondo e feriscono i nemici.
pub fn update_darts(
    mut commands: Commands,
    time: Res<Time>,
    spatial: avian3d::prelude::SpatialQuery,
    solids: Query<(), crate::combat::ArrowSolids>,
    mut darts: Query<(Entity, &mut Transform, &mut Dart)>,
    mut enemies: Query<
        (&GlobalTransform, &mut crate::combat::EnemyHealth),
        With<crate::entities::Enemy>,
    >,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut dart) in &mut darts {
        dart.life -= dt;
        let step = dart.vel * dt;
        let from = tf.translation;
        let blocked = Dir3::new(step).is_ok_and(|dir| {
            spatial
                .cast_ray_predicate(
                    from,
                    dir,
                    step.length(),
                    true,
                    &avian3d::prelude::SpatialQueryFilter::default(),
                    &|ent| solids.contains(ent),
                )
                .is_some()
        });
        if dart.life <= 0.0 || blocked {
            commands.entity(e).despawn();
            continue;
        }
        tf.translation += step;
        let mid = from + step / 2.0;
        for (et, mut hp) in &mut enemies {
            if hp.current > 0.0 && et.translation().distance(mid) < 1.2 {
                hp.current = (hp.current - dart.damage).max(0.0);
                skills::train(Skill::Magic, 4.0);
                info!("[MAGIA] dardo su nemico: vita -> {:.0}", hp.current);
                commands.entity(e).despawn();
                break;
            }
        }
    }
}

/// Sistema: «Calma» placa i nemici vicini; la calma passa da sola.
pub fn apply_calm(
    mut commands: Commands,
    time: Res<Time>,
    mut magic: ResMut<Magic>,
    enemies: Query<(Entity, &GlobalTransform), With<crate::entities::Enemy>>,
    mut calmed: Query<(Entity, &mut Calmed)>,
) {
    if let Some(at) = magic.calm_at.take() {
        for (e, t) in &enemies {
            if t.translation().distance(at) <= CALM_RANGE {
                commands.entity(e).insert(Calmed(CALM_SECS));
                info!("[MAGIA] un nemico si calma");
            }
        }
    }
    for (e, mut c) in &mut calmed {
        c.0 -= time.delta_secs();
        if c.0 <= 0.0 {
            commands.entity(e).remove::<Calmed>();
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Magic>()
        .add_systems(Update, sync_taught)
        .add_systems(
            Update,
            (cast, follow_orb, update_darts, apply_calm).run_if(in_state(crate::AppState::Playing)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spells_are_learned_with_the_grade() {
        assert_eq!(known_with(1, true), vec![Spell::Light, Spell::Dart]);
        assert_eq!(known_with(5, true).len(), 3);
        assert_eq!(
            known_with(5, false).len(),
            2,
            "senza Agnese solo luce e dardo"
        );
        assert_eq!(known_with(20, true), Spell::ALL.to_vec());
        for s in Spell::ALL {
            assert_eq!(Spell::parse(s.key()), Some(s));
            assert!(s.cost() <= 25.0, "{s:?} deve stare nella riserva di base");
        }
        assert_eq!(Spell::parse("Soffio"), Some(Spell::Heal));
    }

    #[test]
    fn z_cycles_only_through_known_spells() {
        assert_eq!(next_with(Spell::Light, 1, true), Spell::Dart);
        assert_eq!(next_with(Spell::Dart, 1, true), Spell::Light);
        assert_eq!(next_with(Spell::Dart, 5, true), Spell::Heal);
        assert_eq!(next_with(Spell::Ward, 100, true), Spell::Calm);
        assert_eq!(next_with(Spell::Calm, 100, true), Spell::Light);
        assert_eq!(
            next_with(Spell::Dart, 5, false),
            Spell::Light,
            "senza Agnese"
        );
    }

    #[test]
    fn the_perk_makes_spells_cheaper_and_wits_make_darts_stronger() {
        assert_eq!(cost(Spell::Dart, 0.0), 12.0);
        assert!(cost(Spell::Dart, 3.0) < 10.0);
        assert!(cost(Spell::Dart, 50.0) >= 6.0);
        assert_eq!(dart_damage(10, 0.0), DART_DAMAGE);
        assert!(dart_damage(16, 0.0) > DART_DAMAGE * 1.2);
        assert!(dart_damage(10, 2.0) > DART_DAMAGE);
    }
}
