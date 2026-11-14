//! Negozi e bancarelle: pannello di acquisto con prezzi in monete (stato `AppState::Shop`).
//!
//! Ogni venditore ha una chiave di assortimento (`Npc.shop`). Dopo il dialogo scritto compare
//! l'opzione "Vorrei comprare qualcosa". Le monete vengono dai nemici (e dalle quest).

use crate::AppState;
use crate::inventory::{Inventory, ItemKind};
use bevy::prelude::*;
use bevy_egui::egui;

/// Una riga dell'assortimento: oggetto, prezzo e quantità per acquisto.
#[derive(Clone, Copy)]
pub struct ShopItem {
    pub kind: ItemKind,
    pub price: u32,
    pub qty: u32,
}

/// Assortimento di un negozio (per chiave).
pub fn stock(key: &str) -> Vec<ShopItem> {
    let i = |kind, price, qty| ShopItem { kind, price, qty };
    match key {
        "viandante" => vec![
            i(ItemKind::HealthPotion, 12, 1),
            i(ItemKind::CarryBelt, 450, 1),
        ],
        "erborista" => vec![
            i(ItemKind::HealthPotion, 10, 1),
            i(ItemKind::MulePotion, 35, 1),
        ],
        "frutta" => vec![i(ItemKind::Apple, 2, 1)],
        "pesce" => vec![i(ItemKind::Fish, 4, 1)],
        "armaiolo" => vec![
            i(ItemKind::ShortSword, 45, 1),
            i(ItemKind::LongSword, 90, 1),
            i(ItemKind::Axe, 70, 1),
            i(ItemKind::Shield, 55, 1),
            i(ItemKind::Satchel, 25, 1),
            i(ItemKind::CanvasPack, 60, 1),
            i(ItemKind::LeatherPack, 140, 1),
            i(ItemKind::ExpeditionPack, 300, 1),
            i(ItemKind::Lockpick, 6, 1),
            i(ItemKind::LeatherArmor, 80, 1),
            i(ItemKind::MailArmor, 180, 1),
            i(ItemKind::PlateArmor, 360, 1),
        ],
        "arciere" => vec![i(ItemKind::Bow, 80, 1), i(ItemKind::Arrow, 15, 10)],
        _ => Vec::new(),
    }
}

/// Negozio aperto.
#[derive(Resource, Default)]
pub struct ShopState {
    pub vendor: String,
    pub key: String,
    pub message: String,
    /// Indice dell'articolo che il giocatore vuole comprare (applicato da `shop_buy`).
    pub pending: Option<usize>,
}

impl ShopState {
    pub fn begin(&mut self, vendor: &str, key: &str) {
        self.vendor = vendor.to_string();
        self.key = key.to_string();
        self.message.clear();
        self.pending = None;
    }
}

/// Sistema (stato Shop): applica l'acquisto richiesto dalla UI.
pub fn shop_buy(
    mut shop: ResMut<ShopState>,
    mut inv: ResMut<Inventory>,
    character: Res<crate::character::Character>,
) {
    let Some(idx) = shop.pending.take() else {
        return;
    };
    let Some(item) = stock(&shop.key).get(idx).copied() else {
        return;
    };
    let price = crate::karma::price(character.stats, item.price, &shop.vendor);
    if inv.qty(ItemKind::Coin) < price {
        shop.message = "Non hai abbastanza monete.".into();
        return;
    }
    inv.consume(ItemKind::Coin, price);
    inv.add(item.kind, item.qty);
    crate::skills::train(crate::skills::Skill::Eloquence, 3.0);
    crate::karma::trust(&shop.vendor, crate::karma::TRADE_TRUST);
    shop.message = format!("Hai comprato: {}", item.kind.name());
    info!(
        "[SHOP] {} comprato da {} per {}",
        item.kind.name(),
        shop.vendor,
        price
    );
}

const GOLD: egui::Color32 = egui::Color32::from_rgb(255, 214, 102);

/// Pannello del negozio (UI egui).
pub fn shop_panel(
    ctx: &egui::Context,
    shop: &mut ShopState,
    coins: u32,
    icons: Option<&crate::inventory_ui::ItemIcons>,
    stats: crate::character::Stats,
) {
    let size = ctx.viewport_rect().size();
    let w = 640.0_f32.min(size.x - 40.0);
    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgba_unmultiplied(10, 12, 20, 250))
        .stroke(egui::Stroke::new(2.0, GOLD))
        .inner_margin(egui::Margin::symmetric(26, 18))
        .corner_radius(egui::CornerRadius::same(10));
    egui::Area::new(egui::Id::new("shop_panel"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(w);
            frame.show(ui, |ui| {
                ui.set_min_width(w - 52.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&shop.vendor)
                            .strong()
                            .size(26.0)
                            .color(GOLD),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("Monete: {coins}"))
                                .size(22.0)
                                .color(egui::Color32::from_rgb(250, 252, 255)),
                        );
                    });
                });
                ui.separator();
                let key = shop.key.clone();
                for (i, item) in stock(&key).iter().enumerate() {
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::hover());
                        crate::inventory_ui::paint_icon(ui, icons, item.kind, rect);
                        ui.vertical(|ui| {
                            let qty = if item.qty > 1 {
                                format!(" ×{}", item.qty)
                            } else {
                                String::new()
                            };
                            ui.label(
                                egui::RichText::new(format!("{}{qty}", item.kind.name()))
                                    .size(21.0)
                                    .color(egui::Color32::from_rgb(250, 252, 255)),
                            );
                            ui.label(
                                egui::RichText::new(item.kind.description())
                                    .size(15.0)
                                    .color(egui::Color32::from_rgb(175, 185, 205)),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let price = crate::karma::price(stats, item.price, &shop.vendor);
                            let ok = coins >= price;
                            if ui
                                .add_enabled(
                                    ok,
                                    egui::Button::new(
                                        egui::RichText::new(format!("Compra  ({price} monete)"))
                                            .size(18.0),
                                    ),
                                )
                                .clicked()
                            {
                                shop.pending = Some(i);
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
                if !shop.message.is_empty() {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(&shop.message).size(18.0).color(GOLD));
                }
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Clic = compra   •   Esc = esci")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(175, 185, 205)),
                );
            });
        });
}

pub fn plugin(app: &mut App) {
    app.init_resource::<ShopState>()
        .add_systems(Update, shop_buy.run_if(in_state(AppState::Shop)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stocks_exist_for_all_vendors() {
        for k in [
            "viandante",
            "erborista",
            "frutta",
            "pesce",
            "armaiolo",
            "arciere",
        ] {
            assert!(!stock(k).is_empty(), "{k}");
        }
        assert!(stock("sconosciuto").is_empty());
    }

    #[test]
    fn requested_items_are_for_sale() {
        let all: Vec<ItemKind> = ["viandante", "armaiolo", "arciere"]
            .iter()
            .flat_map(|k| stock(k))
            .map(|i| i.kind)
            .collect();
        for kind in [
            ItemKind::HealthPotion,
            ItemKind::ShortSword,
            ItemKind::LongSword,
            ItemKind::Axe,
            ItemKind::Shield,
            ItemKind::Bow,
            ItemKind::Arrow,
        ] {
            assert!(all.contains(&kind), "{kind:?}");
        }
    }
}
