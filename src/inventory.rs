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
}

impl ItemKind {
    /// Nome mostrato nell'inventario.
    pub fn name(self) -> &'static str {
        match self {
            ItemKind::HealthPotion => "Pozione",
            ItemKind::Coin => "Moneta",
        }
    }

    /// Colore dell'icona placeholder (usato per il disegno egui).
    pub fn color(self) -> Color {
        match self {
            ItemKind::HealthPotion => Color::srgb(0.9, 0.2, 0.2),
            ItemKind::Coin => Color::srgb(0.95, 0.8, 0.2),
        }
    }
}

/// Uno slot dell'inventario: tipo + quantità.
#[derive(Clone, Debug)]
pub struct ItemStack {
    pub kind: ItemKind,
    pub qty: u32,
}

/// Inventario del player (resource, NON soggetta a reset scena).
#[derive(Resource, Default)]
pub struct Inventory {
    pub slots: Vec<ItemStack>,
}

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
        match kind {
            ItemKind::HealthPotion => {
                let Ok(mut hp) = health.single_mut() else {
                    continue;
                };
                if hp.current >= hp.max {
                    info!("[INVENTORY] pozione non usata: vita già piena");
                    continue;
                }
                hp.current = (hp.current + POTION_HEAL).min(hp.max);
                inventory.consume(ItemKind::HealthPotion, 1);
                info!(
                    "[INVENTORY] pozione usata (slot {}): vita {:.0}/{:.0}",
                    i + 1,
                    hp.current,
                    hp.max
                );
            }
            ItemKind::Coin => {}
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
        ItemKind::HealthPotion => 0.4,
        ItemKind::Coin => 0.3,
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
