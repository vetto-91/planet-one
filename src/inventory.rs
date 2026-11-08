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
}

impl ItemKind {
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
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: vec![ItemStack {
                kind: ItemKind::Coin,
                qty: START_COINS,
            }],
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
    /// Aggiunge (o incrementa) una quantità di un tipo.
    pub fn add(&mut self, kind: ItemKind, qty: u32) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.kind == kind) {
            slot.qty += qty;
        } else {
            self.slots.push(ItemStack { kind, qty });
        }
    }

    /// Quantità posseduta di un tipo (0 se assente).
    pub fn qty(&self, kind: ItemKind) -> u32 {
        self.slots
            .iter()
            .find(|s| s.kind == kind)
            .map_or(0, |s| s.qty)
    }

    /// Slot mostrati nella barra in basso, in ordine di raccolta: niente
    /// monete (hanno il loro contatore) e solo quantità > 0, massimo 10
    /// (tasti 1..9 e 0). HUD e uso degli oggetti condividono questa lista,
    /// così il numero a schermo corrisponde al tasto da premere.
    pub fn hotbar(&self) -> Vec<&ItemStack> {
        self.slots
            .iter()
            .filter(|s| s.kind != ItemKind::Coin && s.qty > 0)
            .take(10)
            .collect()
    }

    /// Consuma `qty` unità di un tipo; lo slot sparisce a quantità 0.
    pub fn consume(&mut self, kind: ItemKind, qty: u32) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.kind == kind) {
            slot.qty = slot.qty.saturating_sub(qty);
        }
        self.slots.retain(|s| s.qty > 0);
    }
}

/// Punti vita ripristinati da una pozione.
pub const POTION_HEAL: f32 = 25.0;

/// Sistema: i tasti 1..9 e 0 usano l'oggetto nello slot corrispondente della
/// barra in basso. La pozione cura [`POTION_HEAL`] HP (non si consuma se la
/// vita è già piena). Disattivato mentre un dialogo è aperto.
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
        let Some(kind) = inventory.hotbar().get(i).map(|s| s.kind) else {
            continue;
        };
        if let Some(heal) = kind.heal() {
            let Ok(mut hp) = health.single_mut() else {
                continue;
            };
            if hp.current >= hp.max {
                info!("[INVENTORY] {} non usato: vita già piena", kind.name());
                continue;
            }
            hp.current = (hp.current + heal).min(hp.max);
            inventory.consume(kind, 1);
            info!(
                "[INVENTORY] {} usato (slot {}): vita {:.0}/{:.0}",
                kind.name(),
                i + 1,
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
