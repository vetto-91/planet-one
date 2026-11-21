//! Oggetti veri del mondo (alberi, fiori, panchine, lampioni) creati dai
//! piazzamenti indicati in modalità debug (`debug/placements.txt`, vedi
//! `debug_place`). Sono modelli low-poly a parallelepipedi/coni: si rigenerano
//! da soli quando i piazzamenti cambiano (F4) o dopo il reset della scena.
//!
//! Gli alberi non vengono messi dove darebbero fastidio (spawn del giocatore,
//! casetta, pozione, zona del ladro): vengono saltati e il log `[PROPS]` lo dice.

use crate::daynight::GameClock;
use crate::debug_place::DebugPlace;
use crate::props::SnapToGround;
use crate::scene::SceneNode;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Tipi di piazzamento che hanno un modello vero (niente segnaposto F4).
pub fn has_model(kind: &str) -> bool {
    matches!(
        kind,
        "albero" | "fiori" | "panchina" | "lampione" | "muro" | "cartello" | "campo" | "orto"
    ) || crate::buildings::is_building(kind)
}

/// Marker degli oggetti creati da qui.
#[derive(Component)]
pub struct WorldProp;

/// Luce di un lampione.
#[derive(Component)]
pub struct LampLight;

/// Testa girevole del faro e luci che si accendono di sera.
#[derive(Component)]
pub struct LighthouseHead;
#[derive(Component)]
pub struct LighthouseBeam;

/// Impronta di un edificio visitabile: le sue luci si accendono se il giocatore è dentro.
#[derive(Component)]
pub struct BuildingFootprint {
    pub half: Vec2,
}

/// Luce interna di un edificio.
#[derive(Component)]
pub struct InteriorLight;

/// Potenza delle luci interne (lm) e apertura della porta (rad, verso l'interno).
const INTERIOR_LUMENS: f32 = 90_000.0;
const DOOR_OPEN_ANGLE: f32 = 1.45;

/// Posto letto: materasso, testata e percorso a piedi dalla strada al fianco del letto (mondo).
pub struct HomeBed {
    pub pos: Vec3,
    pub head: Vec3,
    pub path: Vec<Vec3>,
}

/// Finestra con scuri vista dall'interno: dove sta chi la apre, dove guarda, a che piano.
pub struct HomeWindow {
    /// Punto sul pavimento a 0,7 m dalla finestra (mondo).
    pub stand: Vec3,
    /// Direzione verso la finestra (orizzontale, mondo).
    pub face: Vec2,
    /// Maniglia degli scuri.
    pub state: Option<Entity>,
}

impl HomeWindow {
    /// La finestra più vicina al letto, sullo stesso piano (±1 m di quota).
    pub fn pick(windows: &[HomeWindow], bed: &HomeBed) -> Option<usize> {
        let floor = bed.pos.y - 0.53;
        windows
            .iter()
            .enumerate()
            .filter(|(_, w)| w.state.is_some() && (w.stand.y - floor).abs() < 1.0)
            .min_by(|(_, a), (_, b)| {
                a.stand
                    .distance(bed.pos)
                    .total_cmp(&b.stand.distance(bed.pos))
            })
            .map(|(i, _)| i)
    }
}

/// Porta e posti letto di un edificio, in coordinate del mondo.
pub struct HomeEntry {
    pub kind: String,
    pub occurrence: usize,
    /// Punto sulla strada davanti alla porta.
    pub door: Vec2,
    /// Centro dell'anta, per aprirla al passaggio.
    pub door_point: Vec3,
    pub beds: Vec<HomeBed>,
    pub windows: Vec<HomeWindow>,
    /// Maniglia della porta.
    pub state: Option<Entity>,
    /// Aperta da un abitante di passaggio: si richiude da sola.
    pub npc_open: bool,
    pub idle: f32,
    /// Era aperta al pubblico nell'ultimo controllo.
    pub was_open: bool,
}

impl HomeEntry {
    /// Nome della chiave di questo edificio.
    pub fn key(&self) -> String {
        format!("{} {}", self.kind, self.occurrence)
    }

    /// `key == self.key()`, senza costruire la stringa (si chiede a ogni frame, ADR 0056).
    pub fn key_is(&self, key: &str) -> bool {
        let Some(rest) = key
            .strip_prefix(self.kind.as_str())
            .and_then(|r| r.strip_prefix(' '))
        else {
            return false;
        };
        // Solo cifre decimali senza zeri in testa: la stessa forma che scrive `format!`.
        !rest.is_empty()
            && rest.bytes().all(|b| b.is_ascii_digit())
            && (rest == "0" || !rest.starts_with('0'))
            && rest.parse::<usize>() == Ok(self.occurrence)
    }
}

/// Case del paese: gli abitanti vi dormono (`casa:` in `abitanti.txt`).
#[derive(Resource, Default)]
pub struct Homes {
    pub list: Vec<HomeEntry>,
    /// Vero dopo la prima costruzione degli edifici.
    pub built: bool,
}

impl Homes {
    /// La casa `occurrence`-esima di tipo `kind`.
    pub fn find(&self, kind: &str, occurrence: usize) -> Option<&HomeEntry> {
        self.list
            .iter()
            .find(|h| h.kind == kind && h.occurrence == occurrence)
    }
}

/// Lanterna emissiva di un lampione (materiale condiviso).
#[derive(Component)]
pub struct LampBulb(Handle<StandardMaterial>);

/// Zone vietate agli alberi: (centro, raggio, motivo).
const TREE_KEEP_OUT: [(Vec2, f32, &str); 5] = [
    (Vec2::new(0.0, 0.0), 3.5, "spawn del giocatore"),
    (Vec2::new(6.5, 0.7), 5.5, "casetta del Viandante"),
    (Vec2::new(6.0, -5.0), 1.5, "pozione"),
    (Vec2::new(31.2, -34.9), 3.0, "ladro"),
    (Vec2::new(27.2, -34.9), 3.0, "percorso del ladro"),
];

/// Zone vietate agli alberi come (centro, raggio) per il debug F4 (ingombro albero già dedotto).
pub fn tree_keep_out() -> Vec<(Vec2, f32)> {
    TREE_KEEP_OUT
        .iter()
        .map(|(c, r, _)| (*c, (*r - 1.2).max(0.0)))
        .collect()
}

fn tree_blocked(p: Vec2) -> Option<&'static str> {
    TREE_KEEP_OUT
        .iter()
        .find(|(c, r, _)| c.distance(p) < *r)
        .map(|(_, _, why)| *why)
}

/// Numero pseudo-casuale stabile in 0..1 dalle coordinate (stessa variazione a ogni avvio).
pub fn hash01(p: Vec2, salt: f32) -> f32 {
    ((p.x * 12.9898 + p.y * 78.233 + salt * 37.719).sin() * 43_758.545)
        .fract()
        .abs()
}

fn mat(materials: &mut Assets<StandardMaterial>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.95,
        ..default()
    })
}

/// Serratura di un edificio: locanda e chiesa aperte, botteghe negli orari del proprietario, il resto privato.
fn lock_for(
    kind: &str,
    occurrence: usize,
    defs: &[crate::villagers::VillagerDef],
) -> crate::hut::Lock {
    use crate::hut::Lock;
    match kind {
        "locanda" | "chiesa" | "casa-naufrago" => Lock::Public,
        "forno" | "fabbro" | "arcere" => {
            let owner = defs.iter().find(|d| {
                d.home
                    .as_ref()
                    .is_some_and(|(k, n, _)| k == kind && *n == occurrence)
            });
            let (start, end) = owner.map_or((8.0, 18.0), |d| (d.hours.0, d.hours.1));
            Lock::WorkHours(start, end)
        }
        _ => Lock::Private,
    }
}

/// Sistema: (ri)crea gli oggetti quando cambiano i piazzamenti o la scena è stata azzerata.
#[allow(clippy::too_many_arguments)]
pub fn sync_world_props(
    mut commands: Commands,
    dbg: Res<DebugPlace>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    pbr: Res<crate::pbr::PbrMaterials>,
    village: Res<crate::village::VillageInfo>,
    field: Option<Res<crate::terrain::TerrainField>>,
    props: Query<Entity, With<WorldProp>>,
    mut homes: ResMut<Homes>,
    clock: Res<GameClock>,
    mut built: Local<(Option<u32>, usize)>,
) {
    // Prima che il terreno esista non si può appoggiare nulla: si aspetta.
    let Some(field) = field else {
        return;
    };
    let stale = built.0 != Some(dbg.revision());
    let wiped = props.is_empty() && built.1 > 0;
    if !stale && !wiped {
        return;
    }
    for e in &props {
        commands.entity(e).despawn();
    }
    let wood = pbr.get(crate::pbr::Mat::Planks);
    let iron = pbr.get(crate::pbr::Mat::Iron);
    let stem = mat(&mut materials, Color::srgb(0.25, 0.55, 0.22));
    let petals = [
        mat(&mut materials, Color::srgb(0.95, 0.45, 0.7)),
        mat(&mut materials, Color::srgb(0.98, 0.85, 0.25)),
        mat(&mut materials, Color::srgb(0.96, 0.96, 0.92)),
        mat(&mut materials, Color::srgb(0.9, 0.25, 0.25)),
        mat(&mut materials, Color::srgb(0.6, 0.4, 0.9)),
    ];
    let bulb_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.55),
        emissive: LinearRgba::rgb(0.2, 0.15, 0.05),
        ..default()
    });
    let firs = crate::trees::FirAssets::new(&mut meshes, &mut materials, &asset_server);
    let stem_mesh = meshes.add(Cylinder::new(0.015, 0.41));
    let head_mesh = meshes.add(Sphere::new(0.09));
    let seat_mesh = meshes.add(crate::pbr::with_tangents(Cuboid::new(1.5, 0.08, 0.45)));
    let back_mesh = meshes.add(crate::pbr::with_tangents(Cuboid::new(1.5, 0.36, 0.06)));
    let leg_mesh = meshes.add(crate::pbr::with_tangents(Cuboid::new(0.08, 0.7, 0.08)));
    let pole_mesh = meshes.add(crate::pbr::with_tangents(Cylinder::new(0.06, 3.4)));
    let base_mesh = meshes.add(crate::pbr::with_tangents(Cylinder::new(0.15, 0.7)));
    let lantern_mesh = meshes.add(Cuboid::new(0.28, 0.4, 0.28));
    // Cartelli: palo, tabella e faccia uguali per tutti; un materiale per scritta (ADR 0056).
    let sign_post = meshes.add(crate::pbr::with_tangents(Cuboid::new(0.12, 2.9, 0.12)));
    let sign_board = meshes.add(crate::pbr::with_tangents(Cuboid::new(1.74, 0.68, 0.07)));
    let sign_face = meshes.add(Rectangle::new(1.66, 0.62));
    let shop_sign_face = meshes.add(Rectangle::new(
        crate::buildings::SIGN_SIZE.x,
        crate::buildings::SIGN_SIZE.y,
    ));
    let mut sign_mats: std::collections::HashMap<String, Handle<StandardMaterial>> =
        std::collections::HashMap::new();
    let cap_mesh = meshes.add(crate::pbr::with_tangents(Cone::new(0.26, 0.22)));

    let building_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.95,
        ..default()
    });
    let snow_mat = crate::seasons::snow_material(&mut materials);
    let water_mat = materials.add(crate::buildings::clear_water_material());
    // Fiamma dei focolari: come quella dei falò (`village_life::spawn_campfire`), ma segue lo
    // stato del focolare e non ha luce sua (il bagliore è `HearthLight`).
    // Due lingue di fuoco: una arancione larga e una gialla più piccola dentro.
    let flame_mesh = meshes.add(Cone::new(0.15, 0.42));
    let flame_core = meshes.add(Cone::new(0.08, 0.28));
    let flame_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.42, 0.08, 0.6),
        emissive: LinearRgba::rgb(2.6, 0.8, 0.1),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    let flame_core_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.8, 0.35),
        emissive: LinearRgba::rgb(3.0, 1.9, 0.5),
        unlit: true,
        ..default()
    });
    let (mut spawned, mut skipped) = (0usize, 0usize);
    let door_mesh_cache: &mut Vec<(f32, f32, Handle<Mesh>)> = &mut Vec::new();
    let shutter_mats: &mut Vec<([f32; 4], Handle<StandardMaterial>)> = &mut Vec::new();
    let shutter_mesh = meshes.add(Cuboid::new(0.33, 0.9, 0.04));
    // Vetri delle finestre delle case: un poco più grandi del vano, i bordi restano nel muro.
    let window_size = crate::buildings::WINDOW_SIZE;
    let pane_mesh = meshes.add(Cuboid::new(
        window_size.x + 0.06,
        window_size.y + 0.06,
        0.012,
    ));
    let glass_mat = pbr.get(crate::pbr::Mat::Glass);
    homes.list.clear();
    let defs = crate::villagers::load_defs();
    let mut occurrences: std::collections::HashMap<String, usize> = Default::default();
    // I modelli procedurali degli edifici dipendono solo da tipo e seme: si costruiscono tutti
    // insieme su tutti i core; lo spawn qui sotto li prende già pronti (ADR 0056).
    let mut models = building_models(&dbg.items);
    for pl in &dbg.items {
        if matches!(pl.kind.as_str(), "campo" | "orto") {
            // Campo coltivato: una sola mesh per tutto il piazzamento.
            let pts: Vec<Vec2> = pl
                .points
                .iter()
                .copied()
                .filter(|p| !crate::terrain::is_water(p.x, p.y))
                .filter(|p| !village.in_wall_band(*p))
                .collect();
            let mesh =
                crate::buildings::crop_field_mesh(&pl.kind, &pts, &|p| field.height_at(p.x, p.y));
            commands.spawn((
                WorldProp,
                SceneNode,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(building_mat.clone()),
                Transform::default(),
                Visibility::default(),
            ));
            spawned += pts.len();
            continue;
        }
        for (j, &p) in pl.points.iter().enumerate() {
            // Molo e barche stanno apposta in mare.
            let at_sea = matches!(pl.kind.as_str(), "molo" | "barca" | "palafitta");
            if !at_sea && crate::terrain::is_water(p.x, p.y) {
                info!(
                    "[PROPS] oggetto in ({:.1},{:.1}) saltato: in mare",
                    p.x, p.y
                );
                skipped += 1;
                continue;
            }
            let yaw = pl.yaw(j);
            if crate::buildings::is_building(&pl.kind) {
                let (model_kind, seed) = model_key(pl, p);
                let model = models
                    .remove(&(model_kind, seed))
                    .unwrap_or_else(|| crate::buildings::model(model_kind, seed));
                // Molo e barche stanno sul mare (quota fissa), il resto si appoggia al terreno.
                let fixed_y = match pl.kind.as_str() {
                    "molo" | "palafitta" => Some(crate::terrain::SEA_LEVEL),
                    // In acqua galleggia; tirata in secco si appoggia sulla sabbia sotto lo scafo
                    // (prima stava sempre a quota del mare e sulla spiaggia ci affondava,
                    // segnalazione 2026-11-18_23-12-54).
                    "barca" => {
                        let (sn, co) = yaw.sin_cos();
                        let at = |x: f32, z: f32| {
                            field.height_at(p.x + x * co + z * sn, p.y - x * sn + z * co)
                        };
                        let sand = [(0.0, -1.3), (0.0, 1.4), (-0.5, 0.0), (0.5, 0.0), (0.0, 0.0)]
                            .iter()
                            .map(|(x, z)| at(*x, *z))
                            .fold(f32::MIN, f32::max);
                        Some((crate::terrain::SEA_LEVEL + 0.05).max(sand + 0.2))
                    }
                    _ => None,
                };
                let kind = pl.kind.clone();
                let model_half = model.half;
                let interior = model.interior;
                // Posti letto e porta in coordinate del mondo, per gli abitanti.
                let place_id = pl.id;
                let mut home_index = None;
                let mut lock = crate::hut::Lock::Private;
                let mut lived = false;
                let mut home = String::new();
                if let Some(door) = interior.door {
                    let occurrence = *occurrences
                        .entry(kind.clone())
                        .and_modify(|n| *n += 1)
                        .or_insert(0);
                    lock = lock_for(&kind, occurrence, &defs);
                    home = format!("{kind} {occurrence}");
                    lived = defs.iter().any(|d| {
                        d.home
                            .as_ref()
                            .is_some_and(|(k, n, _)| *k == kind && *n == occurrence)
                    });
                    let y = crate::props::ground_under(&field, p, model_half) - 0.05;
                    let origin = Vec3::new(p.x, y, p.y);
                    let rot = Quat::from_rotation_y(yaw);
                    let world = |l: Vec3| origin + rot * l;
                    let outside = world(door.hinge + Vec3::new(door.width / 2.0, 0.0, 1.0));
                    let street =
                        Vec3::new(outside.x, field.height_at(outside.x, outside.z), outside.z);
                    homes.list.push(HomeEntry {
                        kind: kind.clone(),
                        occurrence,
                        door: Vec2::new(outside.x, outside.z),
                        door_point: world(door.hinge + Vec3::new(door.width / 2.0, 1.0, 0.0)),
                        beds: interior
                            .beds
                            .iter()
                            .map(|b| HomeBed {
                                pos: world(b.pos),
                                head: rot * b.head,
                                path: std::iter::once(street)
                                    .chain(interior.route_to_bed(b).into_iter().map(world))
                                    .collect(),
                            })
                            .collect(),
                        windows: interior
                            .windows
                            .iter()
                            .map(|w| HomeWindow {
                                stand: world(w.center - w.normal * 0.7 - Vec3::Y * 1.25),
                                face: (rot * -w.normal).xz().normalize_or_zero(),
                                state: None,
                            })
                            .collect(),
                        state: None,
                        npc_open: false,
                        idle: 0.0,
                        was_open: lock.open_to_all(clock.hour),
                    });
                    home_index = Some(homes.list.len() - 1);
                }
                let mut door_state = None;
                let mut e = commands.spawn((
                    WorldProp,
                    SceneNode,
                    Transform::from_xyz(p.x, fixed_y.unwrap_or(0.0), p.y)
                        .with_rotation(Quat::from_rotation_y(yaw)),
                    Visibility::default(),
                    RigidBody::Static,
                ));
                if fixed_y.is_none() {
                    e.insert(SnapToGround {
                        offset: -0.05,
                        half: model_half,
                    });
                }
                if !interior.lights.is_empty() {
                    e.insert(BuildingFootprint { half: model_half });
                }
                if kind == "grotta" {
                    e.insert(crate::places::CaveMouth);
                }
                if kind == "pozzo" {
                    e.insert(crate::places::WellMouth);
                }
                e.with_children(|c| {
                    if model.snow.count_vertices() > 0 {
                        c.spawn((
                            crate::seasons::SeasonSnowRoof,
                            Mesh3d(meshes.add(model.snow)),
                            MeshMaterial3d(snow_mat.clone()),
                            bevy::light::NotShadowCaster,
                            Visibility::Hidden,
                        ));
                    }
                    for (m, mesh) in model.parts {
                        c.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(pbr.get(m))));
                    }
                    for (i, (center, size)) in model.colliders.into_iter().enumerate() {
                        let mut col = c.spawn((
                            Collider::cuboid(size.x, size.y, size.z),
                            Transform::from_translation(center),
                            crate::camera::BlocksCamera,
                        ));
                        // Il piano del molo è "terreno" per il controllo di appoggio del giocatore.
                        if kind == "molo" && i == 0 {
                            col.insert(crate::scene::Ground);
                        }
                    }
                    for (center, rot, size) in &interior.roofs {
                        c.spawn((
                            Collider::cuboid(size.x, size.y, size.z),
                            Transform::from_translation(*center).with_rotation(*rot),
                            crate::camera::BlocksCamera,
                        ));
                    }
                    for (center, rot, size) in &interior.slides {
                        c.spawn((
                            Collider::cuboid(size.x, size.y, size.z),
                            Transform::from_translation(*center).with_rotation(*rot),
                            crate::player::Slippery,
                        ));
                    }
                    for (center, size) in interior.walls {
                        c.spawn((
                            Collider::cuboid(size.x, size.y, size.z),
                            Transform::from_translation(center),
                            crate::camera::BlocksCamera,
                            crate::scene::Wall,
                        ));
                    }
                    for (center, size) in interior.ground {
                        c.spawn((
                            Collider::cuboid(size.x, size.y, size.z),
                            Transform::from_translation(center),
                            crate::scene::Ground,
                            crate::camera::BlocksCamera,
                        ));
                    }
                    for (center, rot, size) in interior.ramps {
                        c.spawn((
                            Collider::cuboid(size.x, size.y, size.z),
                            Transform::from_translation(center).with_rotation(rot),
                            crate::scene::Ground,
                        ));
                    }
                    for (center, size) in interior.furniture {
                        c.spawn((
                            Collider::cuboid(size.x, size.y, size.z),
                            Transform::from_translation(center),
                        ));
                    }
                    for (i, spot) in interior.containers.iter().enumerate() {
                        c.spawn(crate::containers::bundle_in(
                            spot.kind,
                            spot.pos,
                            format!("{place_id}:{j}:{i}"),
                            &kind,
                            &home,
                        ));
                    }
                    for (i, win) in interior.windows.iter().enumerate() {
                        let n = win.normal;
                        // Sistema locale della finestra: x = sinistra vista da fuori, z = verso dentro.
                        let left = -Vec3::Y.cross(n).normalize();
                        let frame = Quat::from_mat3(&Mat3::from_cols(left, Vec3::Y, -n));
                        let state = c
                            .spawn((
                                crate::hut::PropKind::Window,
                                crate::hut::PropId(format!("scuri:{place_id}:{j}:{i}")),
                                // Di giorno gli scuri stanno aperti.
                                crate::hut::PropState { on: true },
                                crate::entities::Interactable {
                                    prompt: "[E] Chiudi scuri".to_string(),
                                    range: 3.0,
                                    aim: Some(0.45),
                                    // Da fuori: da dentro si passa dal vetro (`WindowInner`).
                                    use_from: Some(n),
                                },
                                Transform::from_translation(win.center + n * 0.12),
                            ))
                            .id();
                        if lived {
                            c.commands()
                                .entity(state)
                                .insert(crate::village_life::ResidentShutter);
                        }
                        if let Some(h) = home_index
                            && let Some(w) = homes.list[h].windows.get_mut(i)
                        {
                            w.state = Some(state);
                        }
                        let shutter_mat = match shutter_mats.iter().find(|(k, _)| *k == win.shutter)
                        {
                            Some((_, m)) => m.clone(),
                            None => {
                                let m = mat(
                                    &mut materials,
                                    Color::srgb(win.shutter[0], win.shutter[1], win.shutter[2]),
                                );
                                shutter_mats.push((win.shutter, m.clone()));
                                m
                            }
                        };
                        for sign in [1.0f32, -1.0] {
                            let leaf_mat = shutter_mat.clone();
                            c.spawn((
                                crate::hut::Hinge {
                                    state,
                                    open_angle: -sign * 1.75,
                                    angle: 0.0,
                                    base: frame,
                                },
                                Transform::from_translation(
                                    win.center + left * (sign * 0.32) + n * 0.05,
                                )
                                .with_rotation(frame),
                                Visibility::default(),
                            ))
                            .with_children(|l| {
                                l.spawn((
                                    Mesh3d(shutter_mesh.clone()),
                                    MeshMaterial3d(leaf_mat),
                                    Transform::from_xyz(-sign * 0.165, 0.0, 0.0),
                                ));
                            });
                        }
                        // Dentro, come nella casetta del Viandante: vetro scorrevole (maniglia in
                        // basso) e scuri raggiungibili col vetro alzato (maniglia in alto).
                        let inner = win.center - n * (win.depth + 0.1);
                        let glass = c
                            .spawn((
                                crate::hut::PropKind::WindowGlass,
                                crate::hut::PropId(format!("vetro:{place_id}:{j}:{i}")),
                                crate::hut::PropState { on: false },
                                crate::entities::Interactable {
                                    prompt: "[E] Alza vetro".to_string(),
                                    range: 3.0,
                                    aim: Some(0.22),
                                    use_from: Some(-n),
                                },
                                Transform::from_translation(inner - Vec3::Y * 0.22),
                            ))
                            .id();
                        c.spawn((
                            crate::hut::PropKind::WindowInner,
                            crate::hut::PropState { on: false },
                            crate::hut::WindowLink {
                                shutters: state,
                                glass,
                            },
                            crate::entities::Interactable {
                                prompt: "[E] Chiudi scuri".to_string(),
                                range: 3.0,
                                aim: Some(0.22),
                                use_from: Some(-n),
                            },
                            Transform::from_translation(inner + Vec3::Y * 0.2),
                        ));
                        // Il vetro sta a metà del muro e, alzato, sparisce dentro l'architrave.
                        // Lontano non si disegna (un vetro trasparente non si nota oltre i 45 m).
                        let pane = win.center - n * (win.depth / 2.0);
                        c.spawn((
                            crate::hut::Slider {
                                state: glass,
                                closed_y: pane.y,
                                open_y: pane.y + window_size.y + 0.05,
                                y: pane.y,
                            },
                            Mesh3d(pane_mesh.clone()),
                            MeshMaterial3d(glass_mat.clone()),
                            Transform::from_translation(pane).with_rotation(frame),
                            bevy::light::NotShadowCaster,
                            bevy::camera::visibility::VisibilityRange::abrupt(0.0, 45.0),
                        ));
                    }
                    let works = matches!(kind.as_str(), "forno" | "fabbro" | "locanda");
                    // Focolari: stato acceso/spento, bagliore e fumo dal camino sopra.
                    let mut hearth_ids: Vec<Entity> = Vec::new();
                    for (i, h) in interior.hearths.iter().enumerate() {
                        let front = Vec3::new(h.x, h.y - 1.2, h.z + 0.5);
                        let state = c
                            .spawn((
                                crate::hut::PropKind::Hearth,
                                crate::hut::PropId(format!("focolare:{place_id}:{j}:{i}")),
                                crate::village_life::OwnHearth(kind == crate::house::KIND),
                                crate::hut::PropState { on: false },
                                crate::entities::Interactable {
                                    prompt: "[E] Accendi il fuoco".to_string(),
                                    range: 2.5,
                                    aim: Some(0.6),
                                    use_from: None,
                                },
                                Transform::from_translation(front),
                            ))
                            .id();
                        c.spawn((
                            crate::village_life::HearthLight {
                                state,
                                seed: i as f32 * 1.7 + j as f32,
                            },
                            PointLight {
                                color: Color::srgb(1.0, 0.55, 0.22),
                                intensity: 0.0,
                                range: 6.0,
                                shadow_maps_enabled: false,
                                ..default()
                            },
                            Transform::from_translation(front + Vec3::Y * 0.1),
                        ));
                        // Fiamma viva nella bocca del focolare, sopra i ceppi.
                        c.spawn((
                            HearthFlame {
                                state,
                                seed: i as f32 * 2.3 + j as f32 * 0.7,
                            },
                            Mesh3d(flame_mesh.clone()),
                            MeshMaterial3d(flame_mat.clone()),
                            Transform::from_translation(Vec3::new(h.x, h.y - 1.8 + 0.42, h.z)),
                            Visibility::Hidden,
                            bevy::light::NotShadowCaster,
                        ))
                        .with_children(|f| {
                            f.spawn((
                                Mesh3d(flame_core.clone()),
                                MeshMaterial3d(flame_core_mat.clone()),
                                Transform::from_xyz(0.0, -0.05, 0.03),
                                bevy::light::NotShadowCaster,
                            ));
                        });
                        hearth_ids.push(state);
                    }
                    for (i, at) in interior.chimneys.iter().enumerate() {
                        c.spawn((
                            crate::village_life::TownChimney {
                                works,
                                always: false,
                                hearth: if interior.hearths.is_empty() {
                                    None
                                } else {
                                    hearth_ids.get(i).copied()
                                },
                            },
                            Transform::from_translation(*at),
                        ));
                    }
                    for (at, half) in &interior.water {
                        c.spawn((
                            Mesh3d(meshes.add(Plane3d::new(Vec3::Y, *half))),
                            MeshMaterial3d(water_mat.clone()),
                            Transform::from_translation(*at),
                            bevy::light::NotShadowCaster,
                        ));
                    }
                    for at in interior.fires {
                        crate::village_life::spawn_campfire(c, at, &mut meshes, &mut materials);
                    }
                    for (at, key) in &interior.signs {
                        // Tabella con la scritta della bottega sulla targa accanto alla porta.
                        let sign_mat = sign_mats
                            .entry(key.to_string())
                            .or_insert_with(|| {
                                materials.add(StandardMaterial {
                                    base_color: Color::WHITE,
                                    base_color_texture: Some(crate::pbr::load_gpu(
                                        &asset_server,
                                        format!("textures/cartelli/{key}.png"),
                                    )),
                                    perceptual_roughness: 0.9,
                                    ..default()
                                })
                            })
                            .clone();
                        c.spawn((
                            // Un rettangolo come nei cartelli (la faccia davanti del `Cuboid`
                            // mostra la texture capovolta).
                            Mesh3d(shop_sign_face.clone()),
                            MeshMaterial3d(sign_mat),
                            Transform::from_translation(*at),
                        ));
                    }
                    if kind == "mulino" {
                        c.spawn((
                            crate::village_life::MillSails,
                            Transform::from_xyz(0.0, 6.0, 3.6),
                            Visibility::default(),
                        ))
                        .with_children(|s| {
                            for (m, mesh) in crate::places::sails() {
                                s.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(pbr.get(m))));
                            }
                        });
                    }
                    // Casa del giocatore: le luci seguono un interruttore accanto alla porta (ADR 0042).
                    let switch = (kind == crate::house::KIND)
                        .then_some(interior.door)
                        .flatten()
                        .map(|door| {
                            let at = door.hinge + Vec3::new(-0.35, 1.3, -0.12);
                            c.spawn((
                                Mesh3d(meshes.add(Cuboid::new(0.12, 0.18, 0.03))),
                                MeshMaterial3d(pbr.get(crate::pbr::Mat::Planks)),
                                Transform::from_translation(at),
                            ));
                            c.spawn((
                                crate::hut::PropKind::Light,
                                crate::hut::PropId(format!("luce:{place_id}:{j}")),
                                crate::hut::PropState { on: false },
                                crate::entities::Interactable {
                                    prompt: "[E] Accendi luce".to_string(),
                                    range: 2.5,
                                    aim: Some(0.3),
                                    use_from: None,
                                },
                                Transform::from_translation(at - Vec3::Z * 0.02),
                            ))
                            .id()
                        });
                    for at in interior.lights {
                        let mut light = c.spawn((
                            InteriorLight,
                            PointLight {
                                color: Color::srgb(1.0, 0.78, 0.5),
                                intensity: INTERIOR_LUMENS,
                                range: 12.0,
                                // L'ombra la decide `graphics::budget_point_shadows`.
                                shadow_maps_enabled: false,
                                ..default()
                            },
                            // La vuole o no secondo le opzioni: `interior_light_shadows`.
                            crate::graphics::ShadowWanted::default(),
                            Transform::from_translation(at),
                            Visibility::Hidden,
                        ));
                        if let Some(state) = switch {
                            light.insert(SwitchedBy(state));
                        }
                    }
                    if let Some(door) = interior.door {
                        // Portone a due battenti: ogni anta è larga metà vano.
                        let leaves = door.leaves.max(1);
                        // I battenti del portone stanno sulla faccia esterna del muro (girano senza
                        // toccarlo); anche la maniglia sta lì, se no il battente stesso la nasconde.
                        let out = if leaves == 2 {
                            Vec3::Z * 0.15
                        } else {
                            Vec3::ZERO
                        };
                        let (w, h) = (door.width / leaves as f32, door.height);
                        let leaf = match door_mesh_cache.iter().find(|(a, b, _)| *a == w && *b == h)
                        {
                            Some((_, _, m)) => m.clone(),
                            None => {
                                let m =
                                    meshes.add(crate::pbr::with_tangents(Cuboid::new(w, h, 0.07)));
                                door_mesh_cache.push((w, h, m.clone()));
                                m
                            }
                        };
                        // Lo stato sta al centro del vano: si usa da fuori e da dentro.
                        let state = c
                            .spawn((
                                crate::hut::PropKind::Door,
                                crate::hut::PropId(format!("porta:{place_id}:{j}")),
                                crate::hut::PropState { on: false },
                                lock,
                                crate::hut::DoorKey(home.clone()),
                                crate::entities::Interactable {
                                    prompt: "[E] Apri porta".to_string(),
                                    range: 3.0,
                                    aim: Some(0.7),
                                    use_from: None,
                                },
                                Transform::from_translation(
                                    door.hinge + out + Vec3::new(door.width / 2.0, 1.0, 0.0),
                                ),
                            ))
                            .id();
                        door_state = Some(state);
                        // (cerniera, verso dell'anta lungo X, angolo d'apertura): la porta si apre
                        // verso dentro, i battenti del portone verso fuori.
                        let hinges: Vec<(Vec3, f32, f32)> = if leaves == 2 {
                            vec![
                                (door.hinge + out, 1.0, -DOOR_OPEN_ANGLE),
                                (
                                    door.hinge + out + Vec3::X * door.width,
                                    -1.0,
                                    DOOR_OPEN_ANGLE,
                                ),
                            ]
                        } else {
                            vec![(door.hinge, 1.0, DOOR_OPEN_ANGLE)]
                        };
                        for (at, along, open_angle) in hinges {
                            let mid = Vec3::new(along * w / 2.0, h / 2.0, 0.0);
                            c.spawn((
                                crate::hut::Hinge {
                                    state,
                                    open_angle,
                                    angle: 0.0,
                                    base: Quat::IDENTITY,
                                },
                                Transform::from_translation(at),
                                Visibility::default(),
                            ))
                            .with_children(|d| {
                                d.spawn((
                                    Mesh3d(leaf.clone()),
                                    MeshMaterial3d(wood.clone()),
                                    Transform::from_translation(mid),
                                ));
                                d.spawn((
                                    crate::hut::DoorCollider,
                                    Collider::cuboid(w, h, 0.1),
                                    Transform::from_translation(mid),
                                    crate::camera::BlocksCamera,
                                ));
                            });
                        }
                    }
                    if kind == "faro" {
                        // Testa del faro: ruota; dentro c'è un faro orientabile e un bagliore fisso.
                        c.spawn((
                            LighthouseHead,
                            Transform::from_xyz(0.0, 14.55, 0.0),
                            Visibility::default(),
                        ))
                        .with_children(|h| {
                            h.spawn((
                                LighthouseBeam,
                                SpotLight {
                                    color: Color::srgb(1.0, 0.92, 0.7),
                                    intensity: 0.0,
                                    range: 160.0,
                                    outer_angle: 0.18,
                                    inner_angle: 0.05,
                                    shadow_maps_enabled: false,
                                    ..default()
                                },
                                Transform::from_xyz(0.0, 0.0, -0.4)
                                    .looking_to(Vec3::new(0.0, -0.06, -1.0), Vec3::Y),
                            ));
                        });
                        c.spawn((
                            LighthouseBeam,
                            PointLight {
                                color: Color::srgb(1.0, 0.88, 0.6),
                                intensity: 0.0,
                                range: 14.0,
                                shadow_maps_enabled: false,
                                ..default()
                            },
                            Transform::from_xyz(0.0, 14.7, 0.0),
                        ));
                    }
                });
                if let Some(i) = home_index {
                    homes.list[i].state = door_state;
                }
                spawned += 1;
                continue;
            }
            if pl.kind == "muro" {
                continue; // il muro è generato da `village` dai vertici
            }
            let half = match pl.kind.as_str() {
                "panchina" => Vec2::new(0.75, 0.25),
                "albero" => Vec2::splat(0.3),
                "lampione" => Vec2::splat(0.15),
                _ => Vec2::ZERO,
            };
            let root = (
                WorldProp,
                SceneNode,
                SnapToGround {
                    offset: -0.05,
                    half,
                },
                Transform::from_xyz(p.x, 0.0, p.y).with_rotation(Quat::from_rotation_y(yaw)),
                Visibility::default(),
            );
            let h = hash01(p, 1.0);
            match pl.kind.as_str() {
                "cartello" => {
                    // Palo con tabella: la chiave della texture è nella nota del piazzamento.
                    let key = if pl.note.is_empty() {
                        "fontevivo"
                    } else {
                        pl.note.as_str()
                    };
                    let sign_mat = sign_mats
                        .entry(key.to_string())
                        .or_insert_with(|| {
                            materials.add(StandardMaterial {
                                base_color: Color::WHITE,
                                base_color_texture: Some(crate::pbr::load_gpu(
                                    &asset_server,
                                    format!("textures/cartelli/{key}.png"),
                                )),
                                perceptual_roughness: 0.9,
                                ..default()
                            })
                        })
                        .clone();
                    commands
                        .spawn((root, RigidBody::Static))
                        .with_children(|c| {
                            c.spawn((
                                // Il palo sta dietro la tabella: non copre la scritta.
                                Mesh3d(sign_post.clone()),
                                MeshMaterial3d(wood.clone()),
                                Transform::from_xyz(0.0, 1.25, -0.1),
                            ));
                            c.spawn((
                                Mesh3d(sign_board.clone()),
                                MeshMaterial3d(wood.clone()),
                                Transform::from_xyz(0.0, 2.05, 0.0),
                            ));
                            c.spawn((
                                Mesh3d(sign_face.clone()),
                                MeshMaterial3d(sign_mat),
                                Transform::from_xyz(0.0, 2.05, 0.04),
                            ));
                            c.spawn((
                                Collider::cuboid(0.3, 2.8, 0.3),
                                Transform::from_xyz(0.0, 1.25, -0.05),
                            ));
                        });
                }
                "albero" => {
                    if crate::village::in_polygon(&village.poly, p) {
                        info!(
                            "[PROPS] albero in ({:.1},{:.1}) saltato: dentro le mura",
                            p.x, p.y
                        );
                        skipped += 1;
                        continue;
                    }
                    if village.in_wall_band(p) {
                        info!(
                            "[PROPS] albero in ({:.1},{:.1}) saltato: nelle mura",
                            p.x, p.y
                        );
                        skipped += 1;
                        continue;
                    }
                    if field.in_fresh_water(p.x, p.y) {
                        skipped += 1;
                        continue;
                    }
                    if let Some(why) = tree_blocked(p) {
                        info!("[PROPS] albero in ({:.1},{:.1}) saltato: {why}", p.x, p.y);
                        skipped += 1;
                        continue;
                    }
                    let scale = 0.85 + hash01(p, 2.0) * 0.45;
                    let variant = (h * 3.0) as usize % 3;
                    commands
                        .spawn((root, RigidBody::Static))
                        .insert(
                            Transform::from_xyz(p.x, 0.0, p.y)
                                .with_rotation(Quat::from_rotation_y(yaw))
                                .with_scale(Vec3::splat(scale)),
                        )
                        .with_children(|c| {
                            c.spawn((
                                Mesh3d(firs.trunk.clone()),
                                MeshMaterial3d(firs.bark.clone()),
                            ));
                            c.spawn((
                                crate::seasons::SeasonLeaf(variant),
                                Mesh3d(firs.foliage[variant].clone()),
                                MeshMaterial3d(firs.needles[variant].clone()),
                            ));
                            // Neve sui rami: stessa chioma, appena più in alto, solo d'inverno.
                            c.spawn((
                                crate::seasons::SeasonSnowRoof,
                                Mesh3d(firs.foliage[variant].clone()),
                                MeshMaterial3d(firs.snow.clone()),
                                Transform::from_xyz(0.0, 0.04, 0.0),
                                Visibility::Hidden,
                                bevy::light::NotShadowCaster,
                            ));
                            c.spawn((
                                Collider::cylinder(0.3, 2.4),
                                Transform::from_xyz(0.0, 0.8, 0.0),
                            ));
                        });
                }
                "fiori" => {
                    commands
                        .spawn(root)
                        .insert(crate::seasons::SeasonFlower {
                            threshold: hash01(p, 5.0) * 0.7,
                        })
                        .with_children(|c| {
                            for k in 0..3 {
                                let a = k as f32 * 2.1 + h * 6.0;
                                let off = Vec3::new(a.cos() * 0.12, 0.0, a.sin() * 0.12);
                                let petal = petals[((h * 5.0) as usize + k) % 5].clone();
                                c.spawn((
                                    Mesh3d(stem_mesh.clone()),
                                    MeshMaterial3d(stem.clone()),
                                    Transform::from_translation(off + Vec3::Y * 0.145),
                                ));
                                c.spawn((
                                    Mesh3d(head_mesh.clone()),
                                    MeshMaterial3d(petal),
                                    Transform::from_translation(off + Vec3::Y * 0.37),
                                ));
                            }
                        });
                }
                "panchina" => {
                    commands
                        .spawn((root, RigidBody::Static))
                        .with_children(|c| {
                            c.spawn((
                                Mesh3d(seat_mesh.clone()),
                                MeshMaterial3d(wood.clone()),
                                Transform::from_xyz(0.0, 0.47, 0.0),
                            ));
                            c.spawn((
                                Mesh3d(back_mesh.clone()),
                                MeshMaterial3d(wood.clone()),
                                Transform::from_xyz(0.0, 0.78, -0.2),
                            ));
                            for x in [-0.65, 0.65] {
                                for z in [-0.17, 0.17] {
                                    c.spawn((
                                        Mesh3d(leg_mesh.clone()),
                                        MeshMaterial3d(iron.clone()),
                                        Transform::from_xyz(x, 0.1, z),
                                    ));
                                }
                            }
                            c.spawn((
                                Collider::cuboid(1.5, 1.15, 0.5),
                                Transform::from_xyz(0.0, 0.325, 0.0),
                            ));
                        });
                }
                "lampione" => {
                    commands
                        .spawn((root, RigidBody::Static))
                        .with_children(|c| {
                            // Il lampione non fa ombra: palo e lanterna, attorno alla sua luce,
                            // facevano un'ombra spezzata o piena proprio sotto (segnalazione
                            // 2026-11-19_20-41-42). Le ombre restano a chi passa nella luce.
                            use bevy::light::NotShadowCaster;
                            c.spawn((
                                Mesh3d(base_mesh.clone()),
                                MeshMaterial3d(iron.clone()),
                                Transform::from_xyz(0.0, -0.05, 0.0),
                                NotShadowCaster,
                            ));
                            c.spawn((
                                Mesh3d(pole_mesh.clone()),
                                MeshMaterial3d(iron.clone()),
                                Transform::from_xyz(0.0, 1.3, 0.0),
                                NotShadowCaster,
                            ));
                            c.spawn((
                                LampBulb(bulb_mat.clone()),
                                Mesh3d(lantern_mesh.clone()),
                                MeshMaterial3d(bulb_mat.clone()),
                                Transform::from_xyz(0.0, 3.15, 0.0),
                                NotShadowCaster,
                            ));
                            c.spawn((
                                Mesh3d(cap_mesh.clone()),
                                MeshMaterial3d(iron.clone()),
                                Transform::from_xyz(0.0, 3.46, 0.0),
                                NotShadowCaster,
                            ));
                            c.spawn((
                                LampLight,
                                PointLight {
                                    color: Color::srgb(1.0, 0.8, 0.5),
                                    intensity: 0.0,
                                    range: 11.0,
                                    shadow_maps_enabled: false,
                                    shadow_depth_bias: 0.012,
                                    shadow_normal_bias: 0.25,
                                    ..default()
                                },
                                crate::graphics::ShadowWanted::default(),
                                Transform::from_xyz(0.0, 3.1, 0.0),
                            ));
                            c.spawn((
                                Collider::cylinder(0.12, 3.4),
                                Transform::from_xyz(0.0, 1.3, 0.0),
                            ));
                        });
                }
                _ => continue,
            }
            spawned += 1;
        }
    }
    info!("[PROPS] oggetti veri creati: {spawned} (alberi saltati: {skipped})");
    *built = (Some(dbg.revision()), spawned);
    homes.built = true;
}

/// Fiamma di un focolare del paese: visibile e tremolante solo quando il focolare è acceso.
#[derive(Component)]
pub struct HearthFlame {
    pub state: Entity,
    pub seed: f32,
}

/// Luce interna comandata da un interruttore (casa del giocatore): segue il suo stato.
#[derive(Component)]
pub struct SwitchedBy(pub Entity);

/// Sistema: accende le luci dell'edificio in cui si trova il giocatore (e nessun'altra); quelle
/// con l'interruttore seguono soltanto l'interruttore, anche da fuori (si vedono dalle finestre).
pub fn update_interior_lights(
    settings: Res<crate::audio::AudioSettings>,
    player: Query<&GlobalTransform, With<crate::player::Player>>,
    buildings: Query<(&GlobalTransform, &BuildingFootprint, &Children)>,
    mut lights: Query<(&mut Visibility, Option<&SwitchedBy>), With<InteriorLight>>,
    switches: Query<&crate::hut::PropState>,
    mut flames: Query<(&HearthFlame, &mut Visibility, &mut Transform), Without<InteriorLight>>,
    time: Res<Time>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    let p = player.translation();
    // Fiamme dei focolari (qui per non aggiungere un sistema): accese seguono il focolare;
    // la scala si scrive solo quando la fiamma si vede (niente propagazione inutile, ADR 0056).
    let now = time.elapsed_secs();
    for (flame, mut vis, mut tf) in &mut flames {
        let lit = switches.get(flame.state).is_ok_and(|s| s.on);
        vis.set_if_neq(if lit {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if lit {
            let k = 1.0 + 0.2 * (now * 9.0 + flame.seed).sin() * (now * 6.1 + flame.seed).cos();
            tf.scale = Vec3::new(k, k * 1.3, k);
        }
    }
    for (tf, footprint, children) in &buildings {
        let local = tf.affine().inverse().transform_point3(p);
        let inside = settings.gfx.interior_lights
            && local.x.abs() <= footprint.half.x + 2.5
            && local.z.abs() <= footprint.half.y + 2.5
            && local.y > -2.0;
        let wanted = if inside {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        for child in children {
            if let Ok((mut vis, switched)) = lights.get_mut(*child) {
                let wanted = match switched {
                    Some(SwitchedBy(state)) if switches.get(*state).is_ok_and(|s| s.on) => {
                        Visibility::Inherited
                    }
                    Some(_) => Visibility::Hidden,
                    None => wanted,
                };
                vis.set_if_neq(wanted);
            }
        }
    }
}

/// Sistema: le luci interne vogliono l'ombra solo con le ombre delle luci piccole (ADR 0057); quali
/// la fanno davvero lo decide `graphics::budget_point_shadows`. Si scrive solo quando cambiano le
/// opzioni o nascono luci nuove.
pub fn interior_light_shadows(
    settings: Res<crate::audio::AudioSettings>,
    added: Query<(), Added<InteriorLight>>,
    mut lights: Query<&mut crate::graphics::ShadowWanted, With<InteriorLight>>,
) {
    if !settings.is_changed() && added.is_empty() {
        return;
    }
    let on = settings.gfx.small_shadows;
    for mut want in &mut lights {
        want.set(on);
    }
}

/// Tipo di modello e seme di un edificio nel punto `p` del piazzamento.
fn model_key(pl: &crate::debug_place::Placement, p: Vec2) -> (&str, u32) {
    let seed = (p.x * 100.0) as i32 as u32 ^ ((p.y * 100.0) as i32 as u32).rotate_left(11);
    // La casa del giocatore ampliata ha i muri di pietra (ADR 0052).
    let model_kind = if pl.kind == crate::house::KIND && pl.note == crate::house::STONE_NOTE {
        "casa-naufrago-pietra"
    } else {
        pl.kind.as_str()
    };
    (model_kind, seed)
}

/// Tutti i modelli di edificio dei piazzamenti, costruiti in parallelo. Un doppione (stesso tipo
/// e seme) resta fuori dalla mappa e si ricostruisce allo spawn, identico.
fn building_models(
    items: &[crate::debug_place::Placement],
) -> std::collections::HashMap<(&str, u32), crate::buildings::Building> {
    let keys: Vec<(&str, u32)> = items
        .iter()
        .filter(|pl| crate::buildings::is_building(&pl.kind))
        .flat_map(|pl| pl.points.iter().map(move |&p| model_key(pl, p)))
        .collect();
    let built = crate::par::map(keys.len(), |i| {
        crate::buildings::model(keys[i].0, keys[i].1)
    });
    let mut out = std::collections::HashMap::with_capacity(keys.len());
    for (key, model) in keys.into_iter().zip(built) {
        out.entry(key).or_insert(model);
    }
    out
}

/// Potenza dei lampioni (lm), calibrata sulla resa HDR.
const LAMP_LUMENS: f32 = 50_000.0;

/// I lampioni si accendono al tramonto e si spengono all'alba (con dissolvenza).
#[allow(clippy::too_many_arguments)]
pub fn update_street_lamps(
    time: Res<Time>,
    clock: Res<GameClock>,
    weather: Res<crate::weather::WeatherState>,
    settings: Res<crate::audio::AudioSettings>,
    mut lights: Query<(&mut PointLight, &mut crate::graphics::ShadowWanted), With<LampLight>>,
    bulbs: Query<&LampBulb>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Accesi anche con il cielo molto scuro (temporale).
    let on = clock.lamps_on() || weather.cloud > 0.95;
    let k = (time.delta_secs() * 1.5).min(1.0);
    let mut level = 0.0;
    for (mut l, mut want) in &mut lights {
        let target = if on { LAMP_LUMENS } else { 0.0 };
        let intensity = l.intensity + (target - l.intensity) * k;
        // Le ombre (luce che non passa le pareti) le fanno solo i lampioni accesi più vicini alla
        // camera: le sceglie `graphics::budget_point_shadows` (ADR 0056).
        want.set(settings.gfx.lamp_shadows);
        if l.intensity != intensity {
            l.intensity = intensity;
        }
        level = intensity / LAMP_LUMENS;
    }
    // Il materiale delle lampadine si ricarica sulla GPU solo quando la luce cambia davvero.
    let emissive = LinearRgba::rgb(0.2 + 6.0 * level, 0.15 + 4.2 * level, 0.05 + 1.8 * level);
    if let Some(b) = bulbs.iter().next()
        && materials.get(&b.0).is_some_and(|m| m.emissive != emissive)
        && let Some(mut m) = materials.get_mut(&b.0)
    {
        m.emissive = emissive;
    }
}

/// Sistema: il faro si accende al tramonto e il fascio gira (un giro ogni ~8 s).
pub fn update_lighthouse(
    time: Res<Time>,
    clock: Res<GameClock>,
    mut heads: Query<&mut Transform, With<LighthouseHead>>,
    mut spots: Query<&mut SpotLight, With<LighthouseBeam>>,
    mut points: Query<&mut PointLight, With<LighthouseBeam>>,
) {
    for mut tf in &mut heads {
        tf.rotate_y(time.delta_secs() * 0.8);
    }
    let dark = clock.lighthouse_dark();
    let k = (time.delta_secs() * 1.5).min(1.0);
    for mut l in &mut spots {
        let target = if dark { 40_000_000.0 } else { 0.0 };
        l.intensity += (target - l.intensity) * k;
    }
    for mut l in &mut points {
        let target = if dark { 200_000.0 } else { 0.0 };
        l.intensity += (target - l.intensity) * k;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_matches_key_exactly() {
        let home = |kind: &str, occurrence| HomeEntry {
            kind: kind.into(),
            occurrence,
            door: Vec2::ZERO,
            door_point: Vec3::ZERO,
            beds: Vec::new(),
            windows: Vec::new(),
            state: None,
            npc_open: false,
            idle: 0.0,
            was_open: false,
        };
        let candidates = [
            "casa 1",
            "casa 0",
            "casa 10",
            "casa 01",
            "casa +1",
            "casa  1",
            "casa",
            "casa ",
            "casa 1 ",
            "bottega 1",
            "casa grande 3",
            "casa 12",
        ];
        for h in [
            home("casa", 1),
            home("casa", 0),
            home("casa", 10),
            home("casa grande", 3),
        ] {
            for k in candidates {
                assert_eq!(h.key_is(k), h.key() == k, "{} vs {k}", h.key());
            }
        }
    }

    fn window(y: f32, x: f32, with_state: bool) -> HomeWindow {
        HomeWindow {
            stand: Vec3::new(x, y, 0.0),
            face: Vec2::Y,
            state: with_state.then_some(Entity::PLACEHOLDER),
        }
    }

    #[test]
    fn the_nearest_window_on_the_beds_floor_is_picked() {
        let bed = HomeBed {
            pos: Vec3::new(0.0, 3.45, 0.0),
            head: Vec3::Z,
            path: Vec::new(),
        };
        // Piano terra (0,42), primo piano (2,92 + 0,53 = quota del letto 3,45).
        let windows = [
            window(0.42, 0.1, true),
            window(2.92, 4.0, true),
            window(2.92, 1.5, true),
            window(2.92, 0.5, false),
        ];
        assert_eq!(HomeWindow::pick(&windows, &bed), Some(2));
        let ground_only = [window(0.42, 0.1, true)];
        assert_eq!(HomeWindow::pick(&ground_only, &bed), None);
    }
}
