//! Inventario e oggetti raccoglibili (Fase 4.1).
//!
//! Modello minimale in memoria (nessun salvataggio su file):
//! - [`ItemKind`]: tipi di oggetto (Pozione, Moneta).
//! - [`ItemStack`]: quantità posseduta per un tipo.
//! - [`Inventory`]: resource con gli slot.
//! - [`Pickup`]: entità fisica raccoglibile nella scena (sp disappear on pickup).

use crate::scene::SceneNode;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Tipi di oggetto raccoglibili.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ItemKind {
    HealthPotion,
    Coin,
    ShortSword,
    LongSword,
    Axe,
    Shield,
    Bow,
    Arrow,
    Apple,
    Fish,
    /// Passepartout: apre ogni porta (solo dalla shell).
    MasterKey,
}

/// Categoria dell'inventario.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Category {
    Weapons,
    Defense,
    Ammo,
    Food,
    Valuables,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Weapons,
        Category::Defense,
        Category::Ammo,
        Category::Food,
        Category::Valuables,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Category::Weapons => "Armi",
            Category::Defense => "Difesa",
            Category::Ammo => "Munizioni",
            Category::Food => "Cibo e cure",
            Category::Valuables => "Valori",
        }
    }
}

impl ItemKind {
    /// Tutti i tipi di oggetto.
    pub const ALL: [ItemKind; 11] = [
        ItemKind::HealthPotion,
        ItemKind::Coin,
        ItemKind::ShortSword,
        ItemKind::LongSword,
        ItemKind::Axe,
        ItemKind::Shield,
        ItemKind::Bow,
        ItemKind::Arrow,
        ItemKind::Apple,
        ItemKind::Fish,
        ItemKind::MasterKey,
    ];

    /// Peso reale di un'unità (kg). Monete e munizioni non pesano.
    pub fn weight_kg(self) -> Option<f32> {
        match self {
            ItemKind::HealthPotion => Some(0.4),
            ItemKind::ShortSword => Some(1.0),
            ItemKind::LongSword => Some(1.5),
            ItemKind::Axe => Some(1.8),
            ItemKind::Shield => Some(3.5),
            ItemKind::Bow => Some(0.9),
            ItemKind::Apple => Some(0.15),
            ItemKind::Fish => Some(0.8),
            ItemKind::MasterKey => Some(0.05),
            ItemKind::Coin | ItemKind::Arrow => None,
        }
    }

    /// Categoria nell'inventario.
    pub fn category(self) -> Category {
        match self {
            ItemKind::ShortSword | ItemKind::LongSword | ItemKind::Axe | ItemKind::Bow => {
                Category::Weapons
            }
            ItemKind::Shield => Category::Defense,
            ItemKind::Arrow => Category::Ammo,
            ItemKind::HealthPotion | ItemKind::Apple | ItemKind::Fish => Category::Food,
            ItemKind::Coin | ItemKind::MasterKey => Category::Valuables,
        }
    }

    /// File dell'icona (`assets/textures/icone/<nome>.png`).
    pub fn icon(self) -> &'static str {
        match self {
            ItemKind::HealthPotion => "pozione",
            ItemKind::Coin => "moneta",
            ItemKind::ShortSword => "spada-corta",
            ItemKind::LongSword => "spada-lunga",
            ItemKind::Axe => "ascia",
            ItemKind::Shield => "scudo",
            ItemKind::Bow => "arco",
            ItemKind::Arrow => "frecce",
            ItemKind::Apple => "mela",
            ItemKind::Fish => "pesce",
            ItemKind::MasterKey => "passepartout",
        }
    }

    /// Le monete hanno il loro contatore: nella barra non vanno mai.
    pub fn can_go_in_hotbar(self) -> bool {
        !matches!(self, ItemKind::Coin | ItemKind::MasterKey)
    }

    /// Nome mostrato nell'inventario.
    pub fn name(self) -> &'static str {
        match self {
            ItemKind::HealthPotion => "Pozione di cura",
            ItemKind::Coin => "Moneta",
            ItemKind::ShortSword => "Spada corta",
            ItemKind::LongSword => "Spada lunga",
            ItemKind::Axe => "Ascia",
            ItemKind::Shield => "Scudo",
            ItemKind::Bow => "Arco",
            ItemKind::Arrow => "Frecce",
            ItemKind::Apple => "Mela",
            ItemKind::Fish => "Pesce",
            ItemKind::MasterKey => "Passepartout",
        }
    }

    /// Descrizione breve (negozi e tooltip).
    pub fn description(self) -> &'static str {
        match self {
            ItemKind::HealthPotion => "Cura 25 punti vita.",
            ItemKind::Coin => "",
            ItemKind::ShortSword => "Danno 9, veloce.",
            ItemKind::LongSword => "Danno 14, raggio lungo.",
            ItemKind::Axe => "Danno 18, lenta, colpisce più nemici.",
            ItemKind::Shield => "Riduce il danno subito del 60%.",
            ItemKind::Bow => "Colpisce da lontano: serve una freccia a colpo.",
            ItemKind::Arrow => "Munizioni per l'arco (pacco da 10).",
            ItemKind::Apple => "Cura 10 punti vita.",
            ItemKind::Fish => "Cura 20 punti vita.",
            ItemKind::MasterKey => "Apre qualsiasi porta del paese.",
        }
    }

    /// Arma che si impugna (spada, ascia, arco).
    pub fn is_weapon(self) -> bool {
        matches!(
            self,
            ItemKind::ShortSword | ItemKind::LongSword | ItemKind::Axe | ItemKind::Bow
        )
    }

    /// Punti vita curati se si mangia/beve.
    pub fn heal(self) -> Option<f32> {
        match self {
            ItemKind::HealthPotion => Some(POTION_HEAL),
            ItemKind::Apple => Some(10.0),
            ItemKind::Fish => Some(20.0),
            _ => None,
        }
    }

    /// Colore dell'icona placeholder (usato per il disegno egui).
    pub fn color(self) -> Color {
        match self {
            ItemKind::HealthPotion => Color::srgb(0.9, 0.2, 0.2),
            ItemKind::Coin => Color::srgb(0.95, 0.8, 0.2),
            ItemKind::ShortSword => Color::srgb(0.7, 0.72, 0.78),
            ItemKind::LongSword => Color::srgb(0.85, 0.87, 0.95),
            ItemKind::Axe => Color::srgb(0.55, 0.38, 0.22),
            ItemKind::Shield => Color::srgb(0.25, 0.4, 0.75),
            ItemKind::Bow => Color::srgb(0.6, 0.42, 0.2),
            ItemKind::Arrow => Color::srgb(0.85, 0.78, 0.55),
            ItemKind::Apple => Color::srgb(0.5, 0.8, 0.3),
            ItemKind::Fish => Color::srgb(0.45, 0.6, 0.75),
            ItemKind::MasterKey => Color::srgb(0.9, 0.75, 0.3),
        }
    }
}

/// Uno slot dell'inventario: tipo + quantità.
#[derive(Clone, Debug)]
pub struct ItemStack {
    pub kind: ItemKind,
    pub qty: u32,
}

/// Monete con cui si inizia (per i primi acquisti).
pub const START_COINS: u32 = 30;

/// Inventario del player (resource, NON soggetta a reset scena).
#[derive(Resource)]
pub struct Inventory {
    pub slots: Vec<ItemStack>,
    /// Oggetto assegnato a ogni tasto della barra (1..9 e 0).
    pub hotbar: [Option<ItemKind>; 10],
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: vec![ItemStack {
                kind: ItemKind::Coin,
                qty: START_COINS,
            }],
            hotbar: [None; 10],
        }
    }
}

/// Equipaggiamento: arma impugnata e scudo.
#[derive(Resource, Default)]
pub struct Equipment {
    pub weapon: Option<ItemKind>,
    pub shield: bool,
}

/// Statistiche di un'arma (a mani nude se `None`).
#[derive(Clone, Copy, Debug)]
pub struct WeaponStats {
    pub damage: f32,
    pub cooldown: f32,
    pub range: f32,
    /// Colpisce tutti i nemici nel cono (ascia).
    pub cleave: bool,
    pub ranged: bool,
}

pub fn weapon_stats(w: Option<ItemKind>) -> WeaponStats {
    let (damage, cooldown, range, cleave, ranged) = match w {
        Some(ItemKind::ShortSword) => (9.0, 0.45, 2.6, false, false),
        Some(ItemKind::LongSword) => (14.0, 0.75, 3.2, false, false),
        Some(ItemKind::Axe) => (18.0, 0.95, 2.6, true, false),
        Some(ItemKind::Bow) => (10.0, 0.9, 60.0, false, true),
        _ => (4.0, 0.6, 2.5, false, false),
    };
    WeaponStats {
        damage,
        cooldown,
        range,
        cleave,
        ranged,
    }
}

/// Quanto scudo riduce il danno subito (moltiplicatore).
pub const SHIELD_FACTOR: f32 = 0.4;

impl Inventory {
    /// Aggiunge (o incrementa) una quantità di un tipo. Un oggetto nuovo prende il primo tasto libero.
    pub fn add(&mut self, kind: ItemKind, qty: u32) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.kind == kind) {
            slot.qty += qty;
            return;
        }
        self.slots.push(ItemStack { kind, qty });
        if kind.can_go_in_hotbar()
            && self.hotbar_position(kind).is_none()
            && let Some(free) = self.hotbar.iter().position(Option::is_none)
        {
            self.hotbar[free] = Some(kind);
        }
    }

    /// Tasto (0..10) a cui è assegnato l'oggetto.
    pub fn hotbar_position(&self, kind: ItemKind) -> Option<usize> {
        self.hotbar.iter().position(|k| *k == Some(kind))
    }

    /// Oggetto nello slot `i` della barra, se posseduto.
    pub fn hotbar_slot(&self, i: usize) -> Option<&ItemStack> {
        let kind = (*self.hotbar.get(i)?)?;
        self.slots.iter().find(|s| s.kind == kind && s.qty > 0)
    }

    /// Assegna un oggetto a un tasto (o lo toglie con `None`). Se il tasto è occupato i due si scambiano.
    pub fn assign_hotbar(&mut self, kind: ItemKind, slot: Option<usize>) {
        if !kind.can_go_in_hotbar() {
            return;
        }
        let previous = self.hotbar_position(kind);
        match slot {
            None => {
                if let Some(p) = previous {
                    self.hotbar[p] = None;
                }
            }
            Some(i) if i < self.hotbar.len() => {
                let occupant = self.hotbar[i];
                self.hotbar[i] = Some(kind);
                if let Some(p) = previous
                    && p != i
                {
                    self.hotbar[p] = occupant;
                }
            }
            Some(_) => {}
        }
    }

    /// Peso totale trasportato (kg): monete e munizioni non contano.
    pub fn total_weight(&self) -> f32 {
        self.slots
            .iter()
            .filter_map(|s| s.kind.weight_kg().map(|w| w * s.qty as f32))
            .sum::<f32>()
            .abs() // la somma vuota è -0.0
    }

    /// Quantità posseduta di un tipo (0 se assente).
    pub fn qty(&self, kind: ItemKind) -> u32 {
        self.slots
            .iter()
            .find(|s| s.kind == kind)
            .map_or(0, |s| s.qty)
    }

    /// Consuma `qty` unità di un tipo; lo slot sparisce a quantità 0.
    pub fn consume(&mut self, kind: ItemKind, qty: u32) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.kind == kind) {
            slot.qty = slot.qty.saturating_sub(qty);
        }
        self.slots.retain(|s| s.qty > 0);
        // Finito un oggetto, il suo tasto si libera.
        for i in 0..self.hotbar.len() {
            if let Some(k) = self.hotbar[i]
                && self.qty(k) == 0
            {
                self.hotbar[i] = None;
            }
        }
    }
}

/// Punti vita ripristinati da una pozione.
pub const POTION_HEAL: f32 = 25.0;

/// Usa un oggetto: cura (si consuma; non se la vita è piena), impugna o ripone l'arma, imbraccia lo scudo.
/// Condivisa da barra e inventario.
pub fn use_item(
    kind: ItemKind,
    inventory: &mut Inventory,
    equip: &mut Equipment,
    health: Option<Mut<crate::player::PlayerHealth>>,
) {
    if let Some(heal) = kind.heal() {
        let Some(mut hp) = health else {
            return;
        };
        if hp.current >= hp.max {
            info!("[INVENTORY] {} non usato: vita già piena", kind.name());
            return;
        }
        hp.current = (hp.current + heal).min(hp.max);
        inventory.consume(kind, 1);
        info!(
            "[INVENTORY] {} usato: vita {:.0}/{:.0}",
            kind.name(),
            hp.current,
            hp.max
        );
    } else if kind.is_weapon() {
        // Impugna / rimette via l'arma.
        equip.weapon = if equip.weapon == Some(kind) {
            None
        } else {
            Some(kind)
        };
        info!("[INVENTORY] arma impugnata: {:?}", equip.weapon);
    } else if kind == ItemKind::Shield {
        equip.shield = !equip.shield;
        info!("[INVENTORY] scudo: {}", equip.shield);
    }
}

/// Sistema: i tasti 1..9 e 0 usano l'oggetto assegnato al riquadro corrispondente della barra.
/// Disattivato mentre un dialogo è aperto.
pub fn use_hotbar_items(
    keys: Res<ButtonInput<KeyCode>>,
    dialog: Res<crate::entities::DialogState>,
    mut inventory: ResMut<Inventory>,
    mut equip: ResMut<Equipment>,
    mut health: Query<&mut crate::player::PlayerHealth, With<crate::player::Player>>,
) {
    if dialog.open {
        return;
    }
    const SLOT_KEYS: [KeyCode; 10] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::Digit0,
    ];
    for (i, key) in SLOT_KEYS.iter().enumerate() {
        if !keys.just_pressed(*key) {
            continue;
        }
        let Some(kind) = inventory.hotbar_slot(i).map(|s| s.kind) else {
            continue;
        };
        use_item(kind, &mut inventory, &mut equip, health.single_mut().ok());
    }
}

/// Marker su entità raccoglibili nella scena 3D.
#[derive(Component)]
pub struct Pickup {
    pub kind: ItemKind,
    pub qty: u32,
}

/// Spawn di un pickup (mesh + collider sensore non-bloccante? usiamo collider
/// statico piccolissimo e controllo distanza nel sistema di raccolta).
pub fn spawn_pickup(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    kind: ItemKind,
    qty: u32,
    position: Vec3,
) -> Entity {
    let color = kind.color();
    let c = color.to_srgba();
    let size = match kind {
        ItemKind::Coin => 0.3,
        _ => 0.4,
    };
    commands
        .spawn((
            SceneNode,
            Pickup { kind, qty },
            RigidBody::Static,
            Collider::cuboid(size, size, size),
            Mesh3d(meshes.add(Cuboid::new(size, size, size))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::new(c.red, c.green, c.blue, 0.0) * 0.3,
                ..default()
            })),
            Transform::from_translation(position),
        ))
        .id()
}

/// Sistema di raccolta: se il player è vicino a un Pickup (<= 2.0 m),
/// lo aggiunge all'inventario e despawna l'entità (raccolta automatica al
/// passaggio, stile action-RPG).
pub fn collect_pickups(
    mut commands: Commands,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    pickups: Query<(Entity, &GlobalTransform, &Pickup)>,
    mut inventory: ResMut<Inventory>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();

    for (entity, t, pickup) in &pickups {
        if t.translation().distance(p) <= 2.0 {
            inventory.add(pickup.kind, pickup.qty);
            commands.entity(entity).despawn();
            info!("raccolto {:?} x{}", pickup.kind, pickup.qty);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coins_and_ammo_have_no_weight_the_rest_does() {
        for k in ItemKind::ALL {
            match k {
                ItemKind::Coin | ItemKind::Arrow => assert!(k.weight_kg().is_none()),
                _ => assert!(k.weight_kg().unwrap() > 0.0, "{k:?}"),
            }
        }
    }

    #[test]
    fn total_weight_ignores_coins_and_ammo() {
        let mut i = Inventory::default();
        i.add(ItemKind::Arrow, 20);
        i.add(ItemKind::Apple, 2);
        assert!((i.total_weight() - 0.3).abs() < 1e-5);
    }

    #[test]
    fn coins_never_enter_the_hotbar() {
        let mut i = Inventory::default();
        i.add(ItemKind::Coin, 50);
        assert!(i.hotbar_position(ItemKind::Coin).is_none());
        i.assign_hotbar(ItemKind::Coin, Some(2));
        assert!(i.hotbar_position(ItemKind::Coin).is_none());
    }

    #[test]
    fn new_items_take_free_keys_and_finished_ones_free_them() {
        let mut i = Inventory::default();
        i.add(ItemKind::Apple, 1);
        i.add(ItemKind::Bow, 1);
        assert_eq!(i.hotbar_position(ItemKind::Apple), Some(0));
        assert_eq!(i.hotbar_position(ItemKind::Bow), Some(1));
        i.consume(ItemKind::Apple, 1);
        assert_eq!(i.hotbar[0], None);
        i.add(ItemKind::Fish, 1);
        assert_eq!(i.hotbar_position(ItemKind::Fish), Some(0));
    }

    #[test]
    fn assigning_to_an_occupied_key_swaps() {
        let mut i = Inventory::default();
        i.add(ItemKind::Apple, 1);
        i.add(ItemKind::Bow, 1);
        i.assign_hotbar(ItemKind::Bow, Some(0));
        assert_eq!(i.hotbar_position(ItemKind::Bow), Some(0));
        assert_eq!(i.hotbar_position(ItemKind::Apple), Some(1));
        i.assign_hotbar(ItemKind::Apple, None);
        assert_eq!(i.hotbar_position(ItemKind::Apple), None);
        // Un'altra aggiunta non lo rimette in barra da solo.
        i.add(ItemKind::Apple, 1);
        assert_eq!(i.hotbar_position(ItemKind::Apple), None);
    }

    #[test]
    fn every_item_has_its_icon_file() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/textures/icone");
        for k in ItemKind::ALL {
            assert!(dir.join(format!("{}.png", k.icon())).exists(), "{k:?}");
        }
    }
}
