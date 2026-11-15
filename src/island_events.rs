//! Gli eventi dell'isola che prima erano solo raccontati (ADR 0053): ora hanno qualcosa nel mondo.
//!
//! Ogni evento compare solo quando è aperto, nelle sue ore e col suo meteo, e solo vicino al
//! giocatore. Quelli da toccare si completano con `E` (la cassa del relitto, il lampione, il tronco
//! delle farfalle, la mela alla ronda, la bottiglia); quelli da guardare quando li si è visti per
//! intero (la scintilla sulla torre, il serpente di luce sui gradoni, la nave nella nebbia). In tutti i
//! casi il mondo dà uno sblocco (`karma::flag`), che le tappe di `eventi.txt` chiedono insieme a luogo,
//! ora e meteo.

use crate::quests::missions::Missions;
use crate::region::Region;
use crate::weather::Kind;
use bevy::prelude::*;

/// Un evento con qualcosa nel mondo.
#[derive(Clone, Copy, Debug)]
pub struct EventSpec {
    pub id: &'static str,
    /// Dove sta la cosa (x, z).
    pub at: Vec2,
    /// Ore in cui c'è (`from > to` = attraverso la mezzanotte).
    pub hours: (f32, f32),
    pub weather: Option<Kind>,
    /// Da toccare con `E` (testo del prompt), o `None` se basta guardare.
    pub prompt: Option<&'static str>,
    /// Lo sblocco che il mondo dà.
    pub flag: &'static str,
    /// Fin dove si tocca, o fin dove si guarda.
    pub reach: f32,
}

/// Gli otto eventi.
pub const EVENTS: [EventSpec; 8] = [
    EventSpec {
        id: "ec01",
        // Sulla sabbia, tra il bosco di betulle e l'acqua.
        at: Vec2::new(38.0, 90.5),
        hours: (5.0, 8.5),
        weather: None,
        prompt: None,
        flag: "relitto-aperto",
        reach: 2.5,
    },
    EventSpec {
        id: "ec03",
        at: Vec2::new(58.5, -58.5),
        hours: (0.0, 3.5),
        weather: Some(Kind::Clear),
        prompt: None,
        flag: "scintilla-torre",
        reach: 26.0,
    },
    EventSpec {
        id: "ec06",
        at: Vec2::new(90.5, 5.0),
        hours: (5.5, 7.5),
        weather: Some(Kind::Clear),
        prompt: None,
        flag: "serpente-di-luce",
        reach: 9.0,
    },
    EventSpec {
        id: "ec07",
        at: Vec2::new(1.6, 52.6),
        hours: (0.0, 24.0),
        weather: Some(Kind::Rain),
        prompt: Some("[E] Avvicina la mano al vetro del lampione"),
        flag: "lampione-azzurro",
        reach: 2.5,
    },
    EventSpec {
        id: "ec09",
        at: Vec2::new(62.0, -8.0),
        hours: (9.0, 17.0),
        weather: Some(Kind::Clear),
        prompt: None,
        flag: "tronco-delle-farfalle",
        reach: 2.5,
    },
    EventSpec {
        id: "ec10",
        at: Vec2::new(0.5, -78.0),
        hours: (21.0, 4.0),
        weather: Some(Kind::Fog),
        prompt: None,
        flag: "nave-fantasma",
        reach: 40.0,
    },
    EventSpec {
        id: "ec11",
        at: Vec2::new(0.2, -7.6),
        hours: (20.0, 6.0),
        weather: Some(Kind::Snow),
        prompt: Some("[E] Lascia una mela sul parapetto"),
        flag: "mela-alla-ronda",
        reach: 4.0,
    },
    EventSpec {
        id: "ec12",
        at: Vec2::new(14.0, 99.0),
        hours: (5.0, 7.0),
        weather: None,
        prompt: Some("[E] Raccogli la bottiglia"),
        flag: "bottiglia-raccolta",
        reach: 2.5,
    },
];

/// Gli eventi con un mobile vero da aprire (la cassa del relitto, il tronco cavo): si completano
/// quando il mobile è stato aperto (`containers`), e dentro c'è quel che racconta la trama.
pub fn container_key(id: &str) -> Option<String> {
    matches!(id, "ec01" | "ec09").then(|| format!("evento:{id}"))
}

/// Fin dove si vede la cosa di un evento (oltre, non c'è).
const SHOW_RANGE: f32 = 90.0;

/// L'ora `h` cade nella finestra `(from, to)`? (`from > to` = attraverso la mezzanotte)
pub fn in_hours(h: f32, (from, to): (f32, f32)) -> bool {
    if from <= to {
        (from..to).contains(&h)
    } else {
        h >= from || h < to
    }
}

/// L'evento è aperto adesso? (prima tappa in corso, sblocco non ancora dato, ora e meteo giusti)
pub fn open_now(
    spec: &EventSpec,
    step: Option<usize>,
    flagged: bool,
    hour: f32,
    weather: Kind,
) -> bool {
    step == Some(0)
        && !flagged
        && in_hours(hour, spec.hours)
        && spec.weather.is_none_or(|w| w == weather)
}

/// La cosa di un evento nel mondo.
#[derive(Component)]
pub struct EventProp {
    pub id: &'static str,
}

/// Parti animate: il lampo sulla torre, le strisce del serpente di luce, la nave, il vetro del lampione,
/// le farfalle.
#[derive(Component)]
pub enum Animated {
    Spark,
    Stripe(usize),
    Ship { from: Vec3, to: Vec3 },
    Pulse,
    Butterfly(f32),
}

/// Stato degli eventi: secondi passati a guardare, e da quanto c'è la cosa.
#[derive(Resource, Default)]
pub struct EventWatch {
    pub watched: std::collections::HashMap<&'static str, f32>,
    pub shown: std::collections::HashMap<&'static str, f32>,
}

/// Chi sta in `from` guardando verso `facing` (in piano) vede `target`? Entro 50° circa.
pub fn looks_at(facing: Vec2, from: Vec2, target: Vec2) -> bool {
    let to = (target - from).normalize_or_zero();
    to == Vec2::ZERO || facing.dot(to) > 0.64
}

/// Secondi da guardare perché un evento sia «visto».
pub fn watch_needed(id: &str) -> f32 {
    match id {
        "ec03" => 7.0,
        "ec06" => 9.0,
        "ec10" => 10.0,
        _ => 0.0,
    }
}

/// Sistema: fa comparire e sparire le cose degli eventi, le anima, e dà lo sblocco a chi le tocca o le
/// guarda per intero.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn island_events(
    mut commands: Commands,
    (time, clock, weather): (
        Res<Time>,
        Res<crate::daynight::GameClock>,
        Res<crate::weather::WeatherState>,
    ),
    (keys, missions, karma, dialog): (
        Res<ButtonInput<KeyCode>>,
        Res<Missions>,
        Res<crate::karma::Karma>,
        Res<crate::entities::DialogState>,
    ),
    field: Option<Res<crate::terrain::TerrainField>>,
    inventory: Res<crate::inventory::Inventory>,
    containers: Res<crate::containers::Containers>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    roots: Query<&GlobalTransform, With<EventProp>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    caves: Query<&GlobalTransform, With<crate::places::CaveMouth>>,
    props: Query<(Entity, &EventProp)>,
    mut parts: Query<(
        &Animated,
        &mut Transform,
        Option<&mut PointLight>,
        &mut Visibility,
    )>,
    mut assets: (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>),
    (sounds, audio): (
        Option<Res<crate::sfx::SfxSounds>>,
        Res<crate::audio::AudioSettings>,
    ),
    mut watch: ResMut<EventWatch>,
    mut nearest: ResMut<crate::entities::NearestInteract>,
) {
    let (Some(field), Ok(player)) = (field, players.single()) else {
        return;
    };
    let me = player.translation();
    let island = crate::region::current() == Region::Isola && missions.enabled;
    let dt = time.delta_secs();
    let now = weather.shown();
    for spec in &EVENTS {
        let open = island
            && open_now(
                spec,
                missions.step_of(spec.id),
                karma.unlocks.contains(spec.flag),
                clock.hour,
                now,
            );
        let near = me.xz().distance(spec.at) <= SHOW_RANGE;
        let present: Vec<Entity> = props
            .iter()
            .filter(|(_, p)| p.id == spec.id)
            .map(|(e, _)| e)
            .collect();
        if !(open && near) {
            for e in present {
                commands.entity(e).despawn();
            }
            watch.shown.remove(spec.id);
            watch.watched.remove(spec.id);
            continue;
        }
        if present.is_empty() {
            let cave = caves.iter().next().map(|c| c.compute_transform());
            let root = spawn_event(&mut commands, &mut assets, &field, spec, cave);
            // Il lampione azzurro ronza davvero, dal vetro.
            if spec.id == "ec07"
                && let Some(sounds) = &sounds
            {
                let hum = commands
                    .spawn((
                        AudioPlayer::new(sounds.hum()),
                        crate::audio::spatial_loop(audio.ambient_volume * 0.6),
                        Transform::from_xyz(0.0, 3.3, 0.0),
                    ))
                    .id();
                commands.entity(root).add_child(hum);
            }
            info!("[EVENTI] {}: compare nel mondo", spec.id);
        }
        *watch.shown.entry(spec.id).or_default() += dt;
        let dist = me.xz().distance(spec.at);
        match spec.prompt {
            Some(prompt) => {
                // La mela alla ronda si lascia solo se ce n'è una nello zaino.
                let can = spec.id != "ec11" || inventory.qty(crate::inventory::ItemKind::Apple) > 0;
                if dist <= spec.reach && can && !dialog.open {
                    nearest.prompt = Some(match nearest.prompt.take() {
                        Some(p) => format!("{p} · {prompt}"),
                        None => prompt.to_string(),
                    });
                    if keys.just_pressed(KeyCode::KeyE) {
                        crate::karma::flag(spec.flag);
                        info!("[EVENTI] {}: toccato ({})", spec.id, spec.flag);
                    }
                }
            }
            None if let Some(key) = container_key(spec.id) => {
                if containers.was_opened(&key) {
                    crate::karma::flag(spec.flag);
                    info!("[EVENTI] {}: aperto ({})", spec.id, spec.flag);
                }
            }
            None => {
                // Conta solo il tempo in cui la camera è girata verso la cosa (la nave si muove).
                let target = if spec.id == "ec10" {
                    present
                        .first()
                        .and_then(|e| roots.get(*e).ok())
                        .map_or(spec.at, |t| t.translation().xz())
                } else {
                    spec.at
                };
                let facing = cameras
                    .iter()
                    .find(|(c, _)| c.is_active)
                    .map(|(_, t)| t.forward().xz().normalize_or_zero());
                let looking = facing.is_some_and(|f| looks_at(f, me.xz(), target));
                if dist <= spec.reach && looking {
                    let w = watch.watched.entry(spec.id).or_default();
                    *w += dt;
                    if *w >= watch_needed(spec.id) {
                        crate::karma::flag(spec.flag);
                        info!("[EVENTI] {}: visto ({})", spec.id, spec.flag);
                    }
                }
            }
        }
    }
    // Animazioni.
    let t = time.elapsed_secs();
    let ship_t = watch.shown.get("ec10").copied().unwrap_or(0.0);
    let serpent_t = watch.shown.get("ec06").copied().unwrap_or(0.0);
    for (anim, mut tf, light, mut vis) in &mut parts {
        match anim {
            Animated::Spark => {
                // Tre lampi, una pausa: un ciclo di tre secondi.
                let c = t % 3.0;
                let on = c < 1.5 && (c * 2.0).fract() < 0.5;
                if let Some(mut l) = light {
                    l.intensity = if on { 600_000.0 } else { 0.0 };
                }
                vis.set_if_neq(if on {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
            Animated::Stripe(i) => {
                // I sei gradoni si accendono uno dopo l'altro, dall'alto verso il basso, e ricominciano.
                let lit = ((serpent_t / 1.2) as usize) % 8;
                vis.set_if_neq(if lit >= *i && lit < 6 + 1 {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
            Animated::Ship { from, to } => {
                // Passa in 40 secondi, poi ricomincia dopo una pausa: chi l'ha persa la rivede.
                let k = ((ship_t % 50.0) / 40.0).clamp(0.0, 1.0);
                tf.translation = from.lerp(*to, k) + Vec3::Y * (t * 0.6).sin() * 0.08;
            }
            Animated::Pulse => {
                let k = 0.6 + 0.4 * (t * 2.6).sin().abs();
                if let Some(mut l) = light {
                    l.intensity = 90_000.0 * k;
                }
            }
            Animated::Butterfly(phase) => {
                let a = t * 1.4 + phase;
                let base = tf.translation;
                let c = Vec3::new(62.0, base.y, -8.0);
                tf.translation =
                    c + Vec3::new(a.cos() * 0.9, (t * 3.0 + phase).sin() * 0.12, a.sin() * 0.9);
                tf.rotation = Quat::from_rotation_y(-a)
                    * Quat::from_rotation_z((t * 18.0 + phase).sin() * 0.7);
            }
        }
    }
}

/// Crea la cosa di un evento.
fn spawn_event(
    commands: &mut Commands,
    (meshes, materials): &mut (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>),
    field: &crate::terrain::TerrainField,
    spec: &EventSpec,
    cave: Option<Transform>,
) -> Entity {
    let mut mat = |c: Color, glow: Option<LinearRgba>, alpha: bool| {
        materials.add(StandardMaterial {
            base_color: c,
            emissive: glow.unwrap_or(LinearRgba::BLACK),
            alpha_mode: if alpha {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            perceptual_roughness: 0.8,
            ..default()
        })
    };
    let ground = |p: Vec2| {
        field
            .height_at(p.x, p.y)
            .max(crate::terrain::SEA_LEVEL + 0.05)
    };
    let at = Vec3::new(spec.at.x, ground(spec.at), spec.at.y);
    let root = commands
        .spawn((
            EventProp { id: spec.id },
            crate::scene::SceneNode,
            Transform::from_translation(at),
            Visibility::default(),
        ))
        .id();
    let child = |commands: &mut Commands,
                 bundle: (Mesh3d, MeshMaterial3d<StandardMaterial>, Transform)| {
        let e = commands.spawn(bundle).id();
        commands.entity(root).add_child(e);
        e
    };
    match spec.id {
        // Il relitto: una cassa sigillata mezza nella sabbia, e qualche asse spezzata.
        "ec01" => {
            let wood = mat(Color::srgb(0.42, 0.3, 0.18), None, false);
            child(
                commands,
                (
                    Mesh3d(meshes.add(Cuboid::new(0.9, 0.6, 0.6))),
                    MeshMaterial3d(wood.clone()),
                    Transform::from_xyz(0.0, 0.18, 0.0).with_rotation(Quat::from_rotation_z(0.12)),
                ),
            );
            let e = commands
                .spawn(crate::containers::bundle(
                    crate::containers::ContainerKind::Crate,
                    Vec3::new(0.0, 0.5, 0.0),
                    "evento:ec01".to_string(),
                    "relitto",
                ))
                .id();
            commands.entity(root).add_child(e);
            for (x, z, yaw) in [(1.1, 0.4, 0.6), (-0.9, -0.6, 1.9), (0.3, 1.2, 2.6)] {
                child(
                    commands,
                    (
                        Mesh3d(meshes.add(Cuboid::new(1.4, 0.06, 0.18))),
                        MeshMaterial3d(wood.clone()),
                        Transform::from_xyz(x, 0.04, z).with_rotation(Quat::from_rotation_y(yaw)),
                    ),
                );
            }
        }
        // La scintilla azzurra in cima alla torre antica.
        "ec03" => {
            let e = child(
                commands,
                (
                    Mesh3d(meshes.add(Sphere::new(0.25))),
                    MeshMaterial3d(mat(
                        Color::srgb(0.6, 0.8, 1.0),
                        Some(LinearRgba::rgb(10.0, 18.0, 40.0)),
                        false,
                    )),
                    Transform::from_xyz(0.0, 28.6 - at.y, 0.0),
                ),
            );
            commands.entity(e).insert((
                Animated::Spark,
                PointLight {
                    color: Color::srgb(0.5, 0.7, 1.0),
                    intensity: 0.0,
                    range: 40.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
            ));
        }
        // Il serpente di luce: sei strisce dorate sui gradoni della rampa della grotta.
        "ec06" => {
            let gold = mat(
                Color::srgba(1.0, 0.8, 0.45, 0.55),
                Some(LinearRgba::rgb(6.0, 4.0, 1.2)),
                true,
            );
            if let Some(cave) = cave {
                commands.entity(root).insert(Transform::IDENTITY);
                for i in 0..6 {
                    // Rampa: da z 9,2 (quota 0,06) a z 15,6 (quota -2,1), in coordinate della grotta.
                    let k = (i as f32 + 0.5) / 6.0;
                    let local = Vec3::new(0.0, 0.06 - 2.16 * k + 0.05, 9.2 + 6.4 * k);
                    let e = child(
                        commands,
                        (
                            Mesh3d(meshes.add(Cuboid::new(2.6, 0.02, 0.9))),
                            MeshMaterial3d(gold.clone()),
                            Transform::from_translation(cave.transform_point(local))
                                .with_rotation(cave.rotation * Quat::from_rotation_x(-0.32)),
                        ),
                    );
                    commands
                        .entity(e)
                        .insert((Animated::Stripe(i), Visibility::Hidden));
                }
            }
        }
        // Il lampione che ronza azzurro sotto la pioggia.
        "ec07" => {
            let iron = mat(Color::srgb(0.12, 0.12, 0.14), None, false);
            child(
                commands,
                (
                    Mesh3d(meshes.add(Cylinder::new(0.07, 3.2))),
                    MeshMaterial3d(iron.clone()),
                    Transform::from_xyz(0.0, 1.6, 0.0),
                ),
            );
            let e = child(
                commands,
                (
                    Mesh3d(meshes.add(Cuboid::new(0.32, 0.42, 0.32))),
                    MeshMaterial3d(mat(
                        Color::srgb(0.6, 0.85, 1.0),
                        Some(LinearRgba::rgb(4.0, 9.0, 18.0)),
                        false,
                    )),
                    Transform::from_xyz(0.0, 3.3, 0.0),
                ),
            );
            commands.entity(e).insert((
                Animated::Pulse,
                PointLight {
                    color: Color::srgb(0.45, 0.7, 1.0),
                    intensity: 60_000.0,
                    range: 12.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
            ));
        }
        // Il tronco cavo e le tre farfalle che gli girano attorno.
        "ec09" => {
            let bark = mat(Color::srgb(0.35, 0.25, 0.16), None, false);
            child(
                commands,
                (
                    Mesh3d(meshes.add(Cylinder::new(0.38, 2.2))),
                    MeshMaterial3d(bark.clone()),
                    Transform::from_xyz(0.0, 0.32, 0.0).with_rotation(Quat::from_rotation_z(1.5)),
                ),
            );
            child(
                commands,
                (
                    Mesh3d(meshes.add(Cylinder::new(0.26, 0.05))),
                    MeshMaterial3d(mat(Color::srgb(0.05, 0.04, 0.03), None, false)),
                    Transform::from_xyz(1.11, 0.32, 0.0).with_rotation(Quat::from_rotation_z(1.5)),
                ),
            );
            let e = commands
                .spawn(crate::containers::bundle(
                    crate::containers::ContainerKind::Crate,
                    Vec3::new(1.1, 0.35, 0.0),
                    "evento:ec09".to_string(),
                    "tronco",
                ))
                .id();
            commands.entity(root).add_child(e);
            let wing = mat(
                Color::srgb(1.0, 0.55, 0.12),
                Some(LinearRgba::rgb(0.4, 0.2, 0.0)),
                false,
            );
            // Le farfalle girano attorno al tronco nel mondo, non come figlie della radice.
            for k in 0..3 {
                commands.spawn((
                    EventProp { id: spec.id },
                    crate::scene::SceneNode,
                    Animated::Butterfly(k as f32 * 2.1),
                    Mesh3d(meshes.add(Cuboid::new(0.16, 0.01, 0.1))),
                    MeshMaterial3d(wing.clone()),
                    Transform::from_xyz(62.0, at.y + 1.2 + k as f32 * 0.15, -8.0),
                    Visibility::default(),
                ));
            }
        }
        // La nave fantasma: scafo, albero, vela e remi pallidi, al largo nella nebbia, da ovest a est.
        "ec10" => {
            let ghost = mat(
                Color::srgba(0.8, 0.86, 0.92, 0.6),
                Some(LinearRgba::rgb(2.2, 2.6, 3.2)),
                true,
            );
            let sea = crate::terrain::SEA_LEVEL;
            let from = Vec3::new(-70.0, sea + 0.2, -104.0);
            let to = Vec3::new(70.0, sea + 0.2, -104.0);
            commands.entity(root).insert((
                Transform::from_translation(from),
                Animated::Ship { from, to },
            ));
            child(
                commands,
                (
                    Mesh3d(meshes.add(Cuboid::new(11.0, 1.4, 2.6))),
                    MeshMaterial3d(ghost.clone()),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                ),
            );
            child(
                commands,
                (
                    Mesh3d(meshes.add(Cylinder::new(0.15, 8.0))),
                    MeshMaterial3d(ghost.clone()),
                    Transform::from_xyz(0.0, 5.0, 0.0),
                ),
            );
            child(
                commands,
                (
                    Mesh3d(meshes.add(Cuboid::new(0.08, 5.0, 4.5))),
                    MeshMaterial3d(ghost.clone()),
                    Transform::from_xyz(0.1, 5.2, 0.0).with_rotation(Quat::from_rotation_y(1.4)),
                ),
            );
            // Due luci pallide a bordo: nella nebbia si vede la nave, non i contorni.
            for x in [-3.5f32, 3.5] {
                let e = commands
                    .spawn((
                        PointLight {
                            color: Color::srgb(0.7, 0.85, 1.0),
                            intensity: 250_000.0,
                            range: 18.0,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_xyz(x, 2.5, 0.0),
                    ))
                    .id();
                commands.entity(root).add_child(e);
            }
            for k in 0..6 {
                for s in [-1.0f32, 1.0] {
                    child(
                        commands,
                        (
                            Mesh3d(meshes.add(Cuboid::new(0.1, 0.1, 2.4))),
                            MeshMaterial3d(ghost.clone()),
                            Transform::from_xyz(-4.0 + k as f32 * 1.6, 0.4, s * 2.2)
                                .with_rotation(Quat::from_rotation_x(s * 0.5)),
                        ),
                    );
                }
            }
        }
        // La ronda infreddolita: niente da creare, la guardia c'è già (il fumetto lo fa `cold_watch`).
        "ec11" => {}
        // La bottiglia verde tra la schiuma.
        "ec12" => {
            child(
                commands,
                (
                    Mesh3d(meshes.add(Capsule3d::new(0.07, 0.22))),
                    MeshMaterial3d(mat(
                        Color::srgba(0.2, 0.6, 0.35, 0.8),
                        Some(LinearRgba::rgb(0.05, 0.2, 0.1)),
                        true,
                    )),
                    Transform::from_xyz(0.0, 0.08, 0.0).with_rotation(Quat::from_rotation_z(1.45)),
                ),
            );
        }
        _ => {}
    }
    root
}

/// Sistema: nella notte di neve la guardia di Porta Nord batte i denti, finché la ronda è aperta.
#[allow(clippy::type_complexity)]
pub fn cold_watch(
    time: Res<Time>,
    props: Query<&EventProp>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    npcs: Query<(Entity, &GlobalTransform, &crate::entities::Npc)>,
    mut barks: ResMut<crate::quests::Barks>,
    mut wait: Local<f32>,
) {
    *wait = (*wait - time.delta_secs()).max(0.0);
    if *wait > 0.0 || !props.iter().any(|p| p.id == "ec11") {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let gate = Vec3::new(0.2, player.translation().y, -7.6);
    if let Some((e, _, npc)) = npcs
        .iter()
        .filter(|(_, t, n)| n.name.starts_with("Guardia") && t.translation().distance(gate) < 12.0)
        .min_by(|a, b| {
            a.1.translation()
                .distance(gate)
                .total_cmp(&b.1.translation().distance(gate))
        })
        && player.translation().distance(gate) < 15.0
    {
        barks.say(
            e,
            npc.name,
            crate::quests::common_line("freddo-ronda").to_string(),
        );
        *wait = 14.0;
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<EventWatch>().add_systems(
        Update,
        (island_events.after(crate::entities::interact), cold_watch)
            .run_if(in_state(crate::AppState::Playing)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watching_needs_the_camera_turned_to_the_thing() {
        let me = Vec2::ZERO;
        assert!(looks_at(Vec2::new(0.0, -1.0), me, Vec2::new(0.0, -10.0)));
        assert!(looks_at(Vec2::new(0.0, -1.0), me, Vec2::new(5.0, -10.0)));
        assert!(!looks_at(Vec2::new(0.0, -1.0), me, Vec2::new(0.0, 10.0)));
        assert!(!looks_at(Vec2::new(1.0, 0.0), me, Vec2::new(0.0, -10.0)));
    }

    #[test]
    fn hours_wrap_around_midnight() {
        assert!(in_hours(22.0, (21.0, 4.0)));
        assert!(in_hours(2.0, (21.0, 4.0)));
        assert!(!in_hours(12.0, (21.0, 4.0)));
        assert!(in_hours(6.0, (5.0, 8.5)));
        assert!(!in_hours(9.0, (5.0, 8.5)));
    }

    #[test]
    fn an_event_is_open_only_at_its_first_step_hours_and_weather() {
        let tower = EVENTS.iter().find(|e| e.id == "ec03").unwrap();
        assert!(open_now(tower, Some(0), false, 1.0, Kind::Clear));
        assert!(!open_now(tower, Some(0), false, 1.0, Kind::Fog));
        assert!(!open_now(tower, Some(0), false, 12.0, Kind::Clear));
        assert!(!open_now(tower, Some(1), false, 1.0, Kind::Clear));
        assert!(!open_now(tower, Some(0), true, 1.0, Kind::Clear));
    }

    #[test]
    fn every_event_flag_is_asked_by_its_mission() {
        let text = include_str!("../assets/quests/prologo/eventi.txt");
        for e in &EVENTS {
            let section = text
                .split("[missione ")
                .find(|s| s.starts_with(&format!("{}]", e.id)))
                .unwrap_or_else(|| panic!("{} manca in eventi.txt", e.id));
            assert!(
                section.contains(&format!("quando: sbloccato {}", e.flag)),
                "{}: la tappa non chiede «sbloccato {}»",
                e.id,
                e.flag
            );
        }
    }
}
