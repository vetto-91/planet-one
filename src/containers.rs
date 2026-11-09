//! Contenitori del mondo (ADR 0023): bauli, armadi, botti, casse e scaffali con un loro inventario.
//!
//! I mobili sono registrati dagli edifici (`Interior.containers`); il contenuto vive nella risorsa
//! [`Containers`], con una chiave stabile, così sopravvive alla ricostruzione degli edifici (F4).
//! Come l'inventario del giocatore, non viene salvato su disco.

use crate::AppState;
use crate::entities::Interactable;
use crate::hut::{PropKind, PropState};
use crate::inventory::{Equipment, Inventory, ItemKind, ItemStack};
use crate::inventory_ui::{ItemIcons, normalize, paint_icon};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use std::collections::HashMap;

/// Riferimento per le capienze: quanto porta il giocatore (kg). Non è un limite applicato a lui.
pub const PLAYER_CARRY_KG: f32 = 30.0;

/// Quanto può contenere un mobile. `Unlimited` è per la casa e il deposito del giocatore.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Capacity {
    Limited(f32),
    #[allow(dead_code)] // usato quando arriveranno casa di proprietà e deposito
    Unlimited,
}

impl Capacity {
    /// Peso ancora libero con `used` kg già dentro.
    fn free(self, used: f32) -> f32 {
        match self {
            Capacity::Limited(kg) => (kg - used).max(0.0),
            Capacity::Unlimited => f32::INFINITY,
        }
    }

    fn label(self, used: f32) -> String {
        match self {
            Capacity::Limited(kg) => format!("{used:.1} / {kg:.0} kg"),
            Capacity::Unlimited => format!("{used:.1} kg (illimitata)"),
        }
    }
}

/// Tipo di mobile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContainerKind {
    Shelf,
    Crate,
    Barrel,
    Chest,
    Wardrobe,
}

impl ContainerKind {
    pub fn name(self) -> &'static str {
        match self {
            ContainerKind::Shelf => "scaffale",
            ContainerKind::Crate => "cassa",
            ContainerKind::Barrel => "botte",
            ContainerKind::Chest => "baule",
            ContainerKind::Wardrobe => "armadio",
        }
    }

    /// Multiplo di quanto porta il giocatore, secondo la grandezza del mobile.
    fn size_factor(self) -> f32 {
        match self {
            ContainerKind::Shelf => 5.0,
            ContainerKind::Crate => 6.0,
            ContainerKind::Barrel => 7.0,
            ContainerKind::Chest => 8.0,
            ContainerKind::Wardrobe => 10.0,
        }
    }

    pub fn capacity(self) -> Capacity {
        Capacity::Limited(PLAYER_CARRY_KG * self.size_factor())
    }
}

/// Dove sta un mobile dentro il suo edificio (coordinate locali).
#[derive(Clone, Copy, Debug)]
pub struct ContainerSpot {
    pub pos: Vec3,
    pub kind: ContainerKind,
}

/// Maniglia di un contenitore nel mondo.
#[derive(Component)]
pub struct Container {
    pub key: String,
    pub kind: ContainerKind,
    pub building: String,
}

/// Componenti di un contenitore nel mondo.
pub fn bundle(kind: ContainerKind, pos: Vec3, key: String, building: &str) -> impl Bundle {
    (
        Container {
            key,
            kind,
            building: building.to_string(),
        },
        PropKind::Container,
        PropState { on: false },
        Interactable {
            prompt: format!("[E] Apri {}", kind.name()),
            range: 2.6,
            aim: Some(0.45),
            use_from: None,
        },
        Transform::from_translation(pos),
    )
}

/// Contenuto di un mobile.
#[derive(Clone, Debug)]
pub struct Store {
    pub capacity: Capacity,
    pub items: Vec<ItemStack>,
}

impl Store {
    pub fn empty(capacity: Capacity) -> Self {
        Self {
            capacity,
            items: Vec::new(),
        }
    }

    pub fn weight(&self) -> f32 {
        self.items
            .iter()
            .filter_map(|s| s.kind.weight_kg().map(|w| w * s.qty as f32))
            .sum::<f32>()
            .abs() // la somma vuota è -0.0
    }

    pub fn qty(&self, kind: ItemKind) -> u32 {
        self.items
            .iter()
            .find(|s| s.kind == kind)
            .map_or(0, |s| s.qty)
    }

    fn add(&mut self, kind: ItemKind, qty: u32) {
        match self.items.iter_mut().find(|s| s.kind == kind) {
            Some(slot) => slot.qty += qty,
            None => self.items.push(ItemStack { kind, qty }),
        }
    }

    fn remove(&mut self, kind: ItemKind, qty: u32) {
        if let Some(slot) = self.items.iter_mut().find(|s| s.kind == kind) {
            slot.qty = slot.qty.saturating_sub(qty);
        }
        self.items.retain(|s| s.qty > 0);
    }
}

/// Dal mobile al giocatore; restituisce quanti ne ha spostati.
pub fn take(store: &mut Store, inventory: &mut Inventory, kind: ItemKind, qty: u32) -> u32 {
    let moved = qty.min(store.qty(kind));
    if moved > 0 {
        store.remove(kind, moved);
        inventory.add(kind, moved);
    }
    moved
}

/// Dal giocatore al mobile, senza superare la capienza; restituisce quanti ne ha spostati.
pub fn put(store: &mut Store, inventory: &mut Inventory, kind: ItemKind, qty: u32) -> u32 {
    let mut moved = qty.min(inventory.qty(kind));
    if let Some(weight) = kind.weight_kg() {
        let fits = (store.capacity.free(store.weight()) / weight + 1e-4).floor();
        moved = moved.min(fits as u32);
    }
    if moved > 0 {
        inventory.consume(kind, moved);
        store.add(kind, moved);
    }
    moved
}

/// Numero pseudo-casuale stabile in 0..1 dalla chiave del mobile.
fn roll(key: &str, salt: u32) -> f32 {
    let mut h: u32 = 0x811C_9DC5 ^ salt.wrapping_mul(0x9E37_79B1);
    for b in key.bytes() {
        h = (h ^ u32::from(b)).wrapping_mul(0x0100_0193);
    }
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 13;
    (h & 0xFF_FFFF) as f32 / 0x100_0000 as f32
}

/// Contenuto iniziale: poche cose a tema, sempre uguali per lo stesso mobile. Alcuni sono vuoti.
pub fn starting_items(kind: ContainerKind, building: &str, key: &str) -> Vec<(ItemKind, u32)> {
    use ContainerKind as K;
    use ItemKind::*;
    if roll(key, 0) < 0.2 {
        return Vec::new();
    }
    let n = |salt: u32, lo: u32, hi: u32| lo + (roll(key, salt) * (hi - lo + 1) as f32) as u32;
    let chance = |salt: u32, p: f32| roll(key, salt) < p;
    let mut out = Vec::new();
    match (kind, building) {
        (K::Crate, "bancarella-frutta") => out.push((Apple, n(1, 4, 9))),
        (K::Crate, "bancarella-pesce") | (K::Crate | K::Barrel, "molo") => {
            out.push((Fish, n(1, 2, 6)));
        }
        (K::Crate, "bancarella-erbe") => out.push((HealthPotion, n(1, 1, 3))),
        (_, "arcere") => {
            out.push((Arrow, n(1, 8, 20)));
            if chance(2, 0.15) {
                out.push((Bow, 1));
            }
        }
        (_, "fabbro") => {
            if chance(1, 0.35) {
                out.push((ShortSword, 1));
            }
            out.push((Coin, n(2, 2, 8)));
        }
        (K::Chest | K::Wardrobe, "caserma") => {
            out.push((Arrow, n(1, 5, 12)));
            if chance(2, 0.2) {
                out.push((Shield, 1));
            }
        }
        (K::Chest, _) => {
            out.push((Coin, n(1, 3, 18)));
            if chance(2, 0.35) {
                out.push((HealthPotion, 1));
            }
        }
        (K::Barrel, _) => out.push((Apple, n(1, 2, 6))),
        (K::Crate, _) => out.push((Apple, n(1, 1, 4))),
        (K::Shelf, _) => {
            if chance(1, 0.4) {
                out.push((HealthPotion, 1));
            } else {
                out.push((Apple, n(2, 1, 3)));
            }
        }
        (K::Wardrobe, _) => {
            if chance(1, 0.4) {
                out.push((Coin, n(2, 1, 6)));
            }
        }
    }
    out
}

/// Contenuti di tutti i mobili aperti almeno una volta e mobile aperto ora.
#[derive(Resource, Default)]
pub struct Containers {
    stores: HashMap<String, Store>,
    open: Option<(String, ContainerKind)>,
}

impl Containers {
    /// Apre un mobile; alla prima apertura ne genera il contenuto.
    pub fn open(&mut self, c: &Container) {
        self.stores.entry(c.key.clone()).or_insert_with(|| {
            let mut store = Store::empty(c.kind.capacity());
            for (item, qty) in starting_items(c.kind, &c.building, &c.key) {
                store.add(item, qty);
            }
            store
        });
        self.open = Some((c.key.clone(), c.kind));
        info!("[CONTAINER] aperto {} ({})", c.key, c.kind.name());
    }
}

/// Scelta fatta nella finestra.
enum Action {
    Take(ItemKind, u32),
    Put(ItemKind, u32),
    TakeAll,
}

/// Una riga: icona, nome, quantità e i pulsanti per spostare una unità o tutte.
fn row(
    ui: &mut egui::Ui,
    icons: Option<&ItemIcons>,
    stack: &ItemStack,
    (one, all): (&str, &str),
) -> Option<u32> {
    let mut chosen = None;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
        paint_icon(ui, icons, stack.kind, rect);
        ui.vertical(|ui| {
            ui.set_width(130.0);
            ui.label(egui::RichText::new(stack.kind.name()).strong());
            let weight = stack.kind.weight_kg().map_or_else(
                || format!("×{}", stack.qty),
                |w| format!("×{} · {:.1} kg", stack.qty, w * stack.qty as f32),
            );
            ui.label(egui::RichText::new(weight).size(13.0));
        });
        if ui.button(one).clicked() {
            chosen = Some(1);
        }
        if stack.qty > 1 && ui.button(all).clicked() {
            chosen = Some(stack.qty);
        }
    });
    chosen
}

fn sorted(items: &[ItemStack]) -> Vec<ItemStack> {
    let mut list: Vec<ItemStack> = items.iter().filter(|s| s.qty > 0).cloned().collect();
    list.sort_by_key(|s| normalize(s.kind.name()));
    list
}

/// Sistema (egui): due colonne, oggetti del giocatore e del mobile.
pub fn container_window(
    mut contexts: EguiContexts,
    mut containers: ResMut<Containers>,
    mut inventory: ResMut<Inventory>,
    mut equip: ResMut<Equipment>,
    icons: Option<Res<ItemIcons>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let Some((key, kind)) = containers.open.clone() else {
        next.set(AppState::Playing);
        return;
    };
    let Some(store) = containers.stores.get_mut(&key) else {
        next.set(AppState::Playing);
        return;
    };
    let size = ctx.viewport_rect().size();
    let width = 860.0_f32.min(size.x - 30.0);
    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgba_unmultiplied(10, 12, 20, 250))
        .stroke(egui::Stroke::new(2.0, crate::ui_style::GOLD))
        .inner_margin(egui::Margin::symmetric(20, 14))
        .corner_radius(egui::CornerRadius::same(10));
    let mut action = None;
    let mut close = ctx.input(|i| i.key_pressed(egui::Key::E));
    let list_height = (size.y - 240.0).max(120.0);
    egui::Area::new(egui::Id::new("container_panel"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(width);
            frame.show(ui, |ui| {
                ui.set_min_width(width - 40.0);
                ui.columns(2, |cols| {
                    cols[0].label(
                        egui::RichText::new("Il tuo inventario")
                            .strong()
                            .size(20.0)
                            .color(crate::ui_style::GOLD),
                    );
                    cols[0].label(format!("{:.1} kg", inventory.total_weight()));
                    cols[0].separator();
                    egui::ScrollArea::vertical()
                        .id_salt("container_player")
                        .max_height(list_height)
                        .show(&mut cols[0], |ui| {
                            for s in sorted(&inventory.slots) {
                                if let Some(q) = row(ui, icons.as_deref(), &s, ("Posa", "Tutti")) {
                                    action = Some(Action::Put(s.kind, q));
                                }
                            }
                        });
                    let title = {
                        let name = kind.name();
                        let mut c = name.chars();
                        c.next().map_or_else(String::new, |f| {
                            f.to_uppercase().collect::<String>() + c.as_str()
                        })
                    };
                    cols[1].label(
                        egui::RichText::new(title)
                            .strong()
                            .size(20.0)
                            .color(crate::ui_style::GOLD),
                    );
                    cols[1].label(store.capacity.label(store.weight()));
                    cols[1].separator();
                    egui::ScrollArea::vertical()
                        .id_salt("container_store")
                        .max_height(list_height)
                        .show(&mut cols[1], |ui| {
                            if store.items.is_empty() {
                                ui.label("Vuoto.");
                            }
                            for s in sorted(&store.items) {
                                if let Some(q) = row(ui, icons.as_deref(), &s, ("Prendi", "Tutti"))
                                {
                                    action = Some(Action::Take(s.kind, q));
                                }
                            }
                        });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!store.items.is_empty(), egui::Button::new("Prendi tutto"))
                        .clicked()
                    {
                        action = Some(Action::TakeAll);
                    }
                    if ui.button("Chiudi").clicked() {
                        close = true;
                    }
                    ui.label("E o Esc = chiudi");
                });
            });
        });
    match action {
        Some(Action::Take(item, qty)) => {
            take(store, &mut inventory, item, qty);
        }
        Some(Action::Put(item, qty)) => {
            put(store, &mut inventory, item, qty);
        }
        Some(Action::TakeAll) => {
            for s in store.items.clone() {
                take(store, &mut inventory, s.kind, s.qty);
            }
        }
        None => {}
    }
    // Ciò che non si possiede più non resta in mano.
    if equip.weapon.is_some_and(|w| inventory.qty(w) == 0) {
        equip.weapon = None;
    }
    if equip.shield && inventory.qty(ItemKind::Shield) == 0 {
        equip.shield = false;
    }
    if close {
        next.set(AppState::Playing);
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Containers>();
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [ContainerKind; 5] = [
        ContainerKind::Shelf,
        ContainerKind::Crate,
        ContainerKind::Barrel,
        ContainerKind::Chest,
        ContainerKind::Wardrobe,
    ];

    #[test]
    fn capacity_is_five_to_ten_times_the_player() {
        for k in ALL {
            let Capacity::Limited(kg) = k.capacity() else {
                panic!("{k:?}: i mobili del paese hanno un limite");
            };
            let times = kg / PLAYER_CARRY_KG;
            assert!((5.0..=10.0).contains(&times), "{k:?}: {times}");
        }
        assert_eq!(
            ContainerKind::Shelf.size_factor() as u32,
            5,
            "il più piccolo vale 5"
        );
        assert_eq!(
            ContainerKind::Wardrobe.size_factor() as u32,
            10,
            "il più grande vale 10"
        );
    }

    #[test]
    fn items_move_both_ways() {
        let mut store = Store::empty(Capacity::Limited(10.0));
        let mut inv = Inventory::default();
        inv.add(ItemKind::Apple, 4);
        assert_eq!(put(&mut store, &mut inv, ItemKind::Apple, 3), 3);
        assert_eq!(
            (store.qty(ItemKind::Apple), inv.qty(ItemKind::Apple)),
            (3, 1)
        );
        assert_eq!(take(&mut store, &mut inv, ItemKind::Apple, 9), 3);
        assert!(store.items.is_empty());
        assert_eq!(inv.qty(ItemKind::Apple), 4);
    }

    #[test]
    fn limited_capacity_stops_at_the_weight() {
        // Scudi da 3,5 kg in 8 kg: ne entrano due.
        let mut store = Store::empty(Capacity::Limited(8.0));
        let mut inv = Inventory::default();
        inv.add(ItemKind::Shield, 5);
        assert_eq!(put(&mut store, &mut inv, ItemKind::Shield, 5), 2);
        assert_eq!(inv.qty(ItemKind::Shield), 3);
        // Monete e frecce non pesano: entrano comunque.
        inv.add(ItemKind::Arrow, 100);
        assert_eq!(put(&mut store, &mut inv, ItemKind::Arrow, 100), 100);
    }

    #[test]
    fn unlimited_capacity_never_refuses() {
        let mut store = Store::empty(Capacity::Unlimited);
        let mut inv = Inventory::default();
        inv.add(ItemKind::Shield, 100_000);
        assert_eq!(
            put(&mut store, &mut inv, ItemKind::Shield, 100_000),
            100_000
        );
        assert!(store.capacity.label(store.weight()).contains("illimitata"));
    }

    #[test]
    fn starting_items_repeat_and_fit() {
        let mut filled = 0;
        let mut empty = 0;
        for i in 0..200 {
            let key = format!("12:0:{i}");
            for building in [
                "casa-media",
                "bancarella-frutta",
                "molo",
                "arcere",
                "fabbro",
                "caserma",
            ] {
                for k in ALL {
                    let a = starting_items(k, building, &key);
                    assert_eq!(a, starting_items(k, building, &key));
                    let mut store = Store::empty(k.capacity());
                    for (item, qty) in a.iter().copied() {
                        store.add(item, qty);
                    }
                    let Capacity::Limited(kg) = k.capacity() else {
                        unreachable!();
                    };
                    assert!(store.weight() <= kg);
                    if a.is_empty() {
                        empty += 1;
                    } else {
                        filled += 1;
                    }
                }
            }
        }
        assert!(filled > 0 && empty > 0);
    }

    #[test]
    fn contents_survive_a_rebuilt_building() {
        let mut all = Containers::default();
        let chest = |key: &str| Container {
            key: key.to_string(),
            kind: ContainerKind::Chest,
            building: "casa".to_string(),
        };
        all.open(&chest("7:0:1"));
        let mut inv = Inventory::default();
        inv.add(ItemKind::Shield, 1);
        let store = all.stores.get_mut("7:0:1").unwrap();
        put(store, &mut inv, ItemKind::Shield, 1);
        // La nuova entità (edificio rifatto con F4) ha la stessa chiave: nulla si rigenera.
        all.open(&chest("7:0:1"));
        assert_eq!(all.stores["7:0:1"].qty(ItemKind::Shield), 1);
    }

    #[test]
    fn furniture_registers_containers() {
        let int = crate::buildings::model("casa-media", 3).interior;
        for k in [
            ContainerKind::Shelf,
            ContainerKind::Barrel,
            ContainerKind::Chest,
        ] {
            assert!(int.containers.iter().any(|c| c.kind == k), "{k:?}");
        }
        assert!(
            !crate::buildings::model("bancarella-pesce", 3)
                .interior
                .containers
                .is_empty()
        );
        assert!(
            !crate::buildings::model("molo", 3)
                .interior
                .containers
                .is_empty()
        );
    }
}
