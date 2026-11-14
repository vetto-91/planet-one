//! La casa del giocatore sul Poggio del Naufrago (ADR 0042).
//!
//! Un terreno fisso a sud-ovest del paese, fuori dalle mura. Si costruisce in due tappe portando
//! sul posto i materiali: prima le **fondamenta** di pietra, poi la **casa** vera (un edificio
//! visitabile come quelli del paese, con porta, arredi e mobili vuoti tutti tuoi).
//!
//! Legname e pietra si raccolgono a due passi dal terreno, col tasto `R`: la catasta al margine
//! del bosco (serve un'ascia) e l'affioramento di roccia. Ogni colpo costa vigore.

use bevy::prelude::*;

use crate::inventory::{Inventory, ItemKind};
use crate::region::Region;
use crate::skills::{self, Skill};

/// Tipo di piazzamento della casa (stesso edificio di `casa-piccola`, ma tuo).
pub const KIND: &str = "casa-naufrago";
/// Centro del terreno, e dove si trovano legname e pietra.
pub const PLOT: Vec2 = Vec2::new(-40.0, 40.0);
pub const WOOD_SPOT: Vec2 = Vec2::new(-51.0, 46.0);
pub const STONE_SPOT: Vec2 = Vec2::new(-31.0, 30.0);
/// Semi-lati dell'impronta (come `casa-piccola`).
pub const HALF: Vec2 = Vec2::new(2.5, 2.0);
const PLOT_RANGE: f32 = 7.0;
const SPOT_RANGE: f32 = 2.6;
const GATHER_COOLDOWN: f32 = 0.9;
const GATHER_VIGOR: f32 = 10.0;

pub const FOUNDATION: u8 = 1;
pub const BUILT: u8 = 2;

/// Materiali (legname, pietra) per arrivare alla tappa `stage` dalla precedente.
pub fn needs(stage: u8) -> Option<(u32, u32)> {
    match stage {
        FOUNDATION => Some((0, 10)),
        BUILT => Some((16, 6)),
        _ => None,
    }
}

pub fn stage_name(stage: u8) -> &'static str {
    match stage {
        0 => "terreno libero",
        FOUNDATION => "fondamenta",
        _ => "casa finita",
    }
}

/// Stato della costruzione.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct House {
    pub stage: u8,
    /// Materiali già messi in opera verso la prossima tappa.
    pub wood: u32,
    pub stone: u32,
}

impl House {
    /// Quanto manca alla prossima tappa (legname, pietra); `None` se la casa è finita.
    pub fn missing(&self) -> Option<(u32, u32)> {
        let (wood, stone) = needs(self.stage + 1)?;
        Some((
            wood.saturating_sub(self.wood),
            stone.saturating_sub(self.stone),
        ))
    }

    /// Mette in opera i materiali portati: rende quanti ne ha usati (legname, pietra) e se la
    /// tappa è compiuta.
    pub fn deliver(&mut self, wood: u32, stone: u32) -> (u32, u32, bool) {
        let Some((need_wood, need_stone)) = self.missing() else {
            return (0, 0, false);
        };
        let (used_wood, used_stone) = (wood.min(need_wood), stone.min(need_stone));
        self.wood += used_wood;
        self.stone += used_stone;
        let done = self.missing() == Some((0, 0));
        if done {
            self.stage += 1;
            self.wood = 0;
            self.stone = 0;
        }
        (used_wood, used_stone, done)
    }
}

/// Oggetti del cantiere (paletti, catasta, roccia, fondamenta): ricreati quando cambia la tappa.
#[derive(Component)]
pub struct SiteProp;

fn here(region_ok: bool, p: Vec3, spot: Vec2, range: f32) -> bool {
    region_ok && p.xz().distance(spot) <= range
}

/// Sistema: raccolta dei materiali e messa in opera, col tasto R.
#[allow(clippy::too_many_arguments)]
pub fn build(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    dialog: Res<crate::entities::DialogState>,
    vitals: Res<crate::vitals::Vitals>,
    mut house: ResMut<House>,
    mut inventory: ResMut<Inventory>,
    mut nearest: ResMut<crate::entities::NearestInteract>,
    mut session: ResMut<crate::save::Session>,
    mut cooldown: Local<f32>,
) {
    *cooldown = (*cooldown - time.delta_secs()).max(0.0);
    if dialog.open {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();
    let island = crate::region::current() == Region::Isola;
    let pressed = keys.just_pressed(KeyCode::KeyR);
    let mut hint = |text: String| {
        nearest.prompt = Some(match nearest.prompt.take() {
            Some(prompt) => format!("{prompt} · {text}"),
            None => text,
        });
    };
    let gather = if here(island, p, WOOD_SPOT, SPOT_RANGE) {
        Some((ItemKind::Wood, "Taglia legname", true))
    } else if here(island, p, STONE_SPOT, SPOT_RANGE) {
        Some((ItemKind::Stone, "Spacca pietra", false))
    } else {
        None
    };
    if let Some((kind, verb, needs_axe)) = gather {
        hint(format!("[R] {verb}"));
        if !pressed || *cooldown > 0.0 {
            return;
        }
        if needs_axe && inventory.qty(ItemKind::Axe) == 0 {
            session.notify("Per tagliare il legname serve un'ascia");
            return;
        }
        if vitals.winded || vitals.vigor < GATHER_VIGOR {
            session.notify("Sei senza fiato: riprendi vigore");
            return;
        }
        *cooldown = GATHER_COOLDOWN;
        crate::vitals::spend(GATHER_VIGOR);
        inventory.add(kind, 1);
        skills::train(Skill::Building, 1.5);
        info!("[CASA] raccolto: {}", kind.name());
        return;
    }
    if !here(island, p, PLOT, PLOT_RANGE) {
        return;
    }
    let Some((need_wood, need_stone)) = house.missing() else {
        return;
    };
    let next = stage_name(house.stage + 1);
    hint(format!(
        "[R] Costruisci: {next} (mancano {need_wood} legname, {need_stone} pietra)"
    ));
    if !pressed {
        return;
    }
    let (wood, stone, done) = house.deliver(
        inventory.qty(ItemKind::Wood),
        inventory.qty(ItemKind::Stone),
    );
    if wood + stone == 0 {
        session.notify("Non hai con te il materiale che serve");
        return;
    }
    inventory.consume(ItemKind::Wood, wood);
    inventory.consume(ItemKind::Stone, stone);
    skills::train(Skill::Building, 5.0 * (wood + stone) as f32);
    if done {
        skills::train(Skill::Building, 40.0);
        crate::karma::deed_as("lavoro", 2, format!("hai costruito: {next}"));
        info!("[CASA] tappa compiuta: {next}");
    } else {
        session.notify(format!("Messi in opera {wood} legname e {stone} pietra"));
        info!("[CASA] messi in opera {wood} legname, {stone} pietra");
    }
}

/// Sistema: la casa finita è un piazzamento in più (non scritto nel file dei piazzamenti).
pub fn sync_placement(house: Res<House>, mut dbg: ResMut<crate::debug_place::DebugPlace>) {
    let wanted = house.stage >= BUILT && crate::region::current() == Region::Isola;
    let present = dbg.items.iter().any(|p| p.kind == KIND);
    if wanted != present {
        dbg.set_runtime(KIND, wanted.then_some(PLOT));
    }
}

/// Sistema: paletti, catasta di legname, roccia e fondamenta del cantiere.
pub fn sync_site(
    mut commands: Commands,
    house: Res<House>,
    field: Option<Res<crate::terrain::TerrainField>>,
    pbr: Res<crate::pbr::PbrMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    props: Query<Entity, With<SiteProp>>,
    mut shown: Local<Option<u8>>,
) {
    let Some(field) = field else {
        return;
    };
    if crate::region::current() != Region::Isola {
        return;
    }
    if *shown == Some(house.stage) && !props.is_empty() {
        return;
    }
    for e in &props {
        commands.entity(e).despawn();
    }
    *shown = Some(house.stage);
    let stone = pbr.get(crate::pbr::Mat::Stone);
    let wood = pbr.get(crate::pbr::Mat::Planks);
    let ground = |p: Vec2| Vec3::new(p.x, field.height_at(p.x, p.y), p.y);
    let mut block = |commands: &mut Commands, at: Vec3, size: Vec3, yaw: f32, mat, solid: bool| {
        let mut e = commands.spawn((
            SiteProp,
            crate::scene::SceneNode,
            Mesh3d(meshes.add(crate::pbr::with_tangents(Cuboid::new(
                size.x, size.y, size.z,
            )))),
            MeshMaterial3d(mat),
            Transform::from_translation(at + Vec3::Y * size.y / 2.0)
                .with_rotation(Quat::from_rotation_y(yaw)),
        ));
        if solid {
            e.insert((
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cuboid(size.x, size.y, size.z),
                crate::camera::BlocksCamera,
            ));
        }
    };
    // Catasta di tronchi al margine del bosco.
    let w = ground(WOOD_SPOT);
    for (i, (dx, dy)) in [
        (-0.5, 0.0),
        (0.0, 0.0),
        (0.5, 0.0),
        (-0.25, 0.4),
        (0.25, 0.4),
    ]
    .into_iter()
    .enumerate()
    {
        block(
            &mut commands,
            w + Vec3::new(dx, dy, 0.0),
            Vec3::new(0.42, 0.4, 2.4 - 0.1 * i as f32),
            0.3,
            wood.clone(),
            i < 3,
        );
    }
    // Affioramento di roccia.
    let s = ground(STONE_SPOT);
    for (dx, dz, size, yaw) in [
        (0.0, 0.0, Vec3::new(1.6, 1.1, 1.3), 0.4),
        (0.9, 0.5, Vec3::new(0.9, 0.6, 0.8), 1.1),
        (-0.8, -0.4, Vec3::new(0.7, 0.45, 0.9), 2.0),
    ] {
        block(
            &mut commands,
            s + Vec3::new(dx, -0.1, dz),
            size,
            yaw,
            stone.clone(),
            true,
        );
    }
    if house.stage >= BUILT {
        return;
    }
    // Quattro paletti agli angoli del terreno; con le fondamenta, il cordolo di pietra.
    let c = ground(PLOT);
    for (sx, sz) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        block(
            &mut commands,
            c + Vec3::new(sx * (HALF.x + 0.3), 0.0, sz * (HALF.y + 0.3)),
            Vec3::new(0.1, 1.0, 0.1),
            0.0,
            wood.clone(),
            false,
        );
    }
    if house.stage >= FOUNDATION {
        for (at, size) in [
            (
                Vec3::new(0.0, 0.0, HALF.y),
                Vec3::new(HALF.x * 2.0, 0.45, 0.4),
            ),
            (
                Vec3::new(0.0, 0.0, -HALF.y),
                Vec3::new(HALF.x * 2.0, 0.45, 0.4),
            ),
            (
                Vec3::new(HALF.x, 0.0, 0.0),
                Vec3::new(0.4, 0.45, HALF.y * 2.0),
            ),
            (
                Vec3::new(-HALF.x, 0.0, 0.0),
                Vec3::new(0.4, 0.45, HALF.y * 2.0),
            ),
        ] {
            block(&mut commands, c + at, size, 0.0, stone.clone(), true);
        }
    }
}

/// Ora a cui ci si sveglia dopo aver dormito nella propria casa.
const WAKE_HOUR: f32 = 7.0;

/// Ore da dormire per svegliarsi alle sette.
pub fn hours_to_morning(hour: f32) -> f32 {
    (WAKE_HOUR - hour).rem_euclid(24.0)
}

/// Sistema: nel letto della propria casa (ADR 0049) si dorme fino al mattino: vita e vigore tornano pieni.
#[allow(clippy::too_many_arguments)]
pub fn sleep_in_bed(
    keys: Res<ButtonInput<KeyCode>>,
    house: Res<House>,
    homes: Res<crate::world_props::Homes>,
    dialog: Res<crate::entities::DialogState>,
    mut clock: ResMut<crate::daynight::GameClock>,
    mut vitals: ResMut<crate::vitals::Vitals>,
    mut players: Query<
        (&GlobalTransform, &mut crate::player::PlayerHealth),
        With<crate::player::Player>,
    >,
    mut nearest: ResMut<crate::entities::NearestInteract>,
    mut session: ResMut<crate::save::Session>,
) {
    if house.stage < BUILT || dialog.open || crate::region::current() != Region::Isola {
        return;
    }
    let Some(bed) = homes
        .list
        .iter()
        .find(|h| h.kind == KIND)
        .and_then(|h| h.beds.first())
    else {
        return;
    };
    let Ok((player, mut hp)) = players.single_mut() else {
        return;
    };
    if player.translation().distance(bed.pos) > 1.8 {
        return;
    }
    nearest.prompt = Some("[R] Dormi fino al mattino".to_string());
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    let hours = hours_to_morning(clock.hour);
    clock.advance(hours);
    hp.current = hp.max;
    vitals.vigor = vitals.vigor_max;
    vitals.magic = vitals.magic_max;
    session.notify("Hai dormito nella tua casa: ti svegli riposato");
    info!("[CASA] dormito {hours:.1} ore");
}

pub fn plugin(app: &mut App) {
    app.init_resource::<House>()
        .add_systems(Update, (sync_placement, sync_site))
        .add_systems(
            Update,
            (build.after(crate::karma::give_gift), sleep_in_bed)
                .run_if(in_state(crate::AppState::Playing)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleeping_wakes_you_at_seven() {
        assert_eq!(hours_to_morning(23.0), 8.0);
        assert_eq!(hours_to_morning(3.0), 4.0);
        assert_eq!(hours_to_morning(7.0), 0.0);
    }

    #[test]
    fn the_house_rises_in_two_stages() {
        let mut h = House::default();
        assert_eq!(h.missing(), Some((0, 10)));
        // Il legname non serve alle fondamenta: resta nello zaino.
        assert_eq!(h.deliver(5, 4), (0, 4, false));
        assert_eq!(h.missing(), Some((0, 6)));
        assert_eq!(h.deliver(0, 20), (0, 6, true));
        assert_eq!((h.stage, h.wood, h.stone), (FOUNDATION, 0, 0));
        assert_eq!(h.missing(), Some((16, 6)));
        assert_eq!(h.deliver(16, 2), (16, 2, false));
        assert_eq!(h.deliver(3, 9), (0, 4, true));
        assert_eq!(h.stage, BUILT);
        assert_eq!(h.missing(), None);
        assert_eq!(h.deliver(10, 10), (0, 0, false));
    }

    #[test]
    fn the_plot_is_outside_the_walls_and_away_from_the_places() {
        let world = include_str!("../assets/world/isola.txt");
        for line in world.lines().filter(|l| l.starts_with("luogo:")) {
            let nums: Vec<f32> = line
                .rsplit('|')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect();
            let place = Vec2::new(nums[0], nums[1]);
            assert!(PLOT.distance(place) > 20.0, "troppo vicino a: {line}");
        }
        // Il paese sta attorno all'origine: il terreno è ben fuori dalle mura.
        assert!(PLOT.length() > 45.0);
        assert!(WOOD_SPOT.distance(PLOT) > 8.0 && STONE_SPOT.distance(PLOT) > 8.0);
        assert_eq!(crate::buildings::interior_half(KIND), Some(HALF));
        assert!(crate::buildings::is_building(KIND));
    }
}
