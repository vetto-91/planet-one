//! Inventario completo (tasto I): categorie, ordine alfabetico, ricerca, filtri, peso e assegnazione ai tasti.

use crate::AppState;
use crate::inventory::{Category, Equipment, Inventory, ItemKind, ItemStack, use_item};
use crate::player::{Player, PlayerHealth};
use crate::ui_style::{GOLD, TEXT};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};
use std::collections::HashMap;

/// Peso massimo del cursore (kg): a fondo scala il filtro è spento.
pub const MAX_WEIGHT_FILTER: f32 = 4.0;

/// Texture delle icone per egui.
#[derive(Resource)]
pub struct ItemIcons(HashMap<ItemKind, egui::TextureId>);

impl ItemIcons {
    pub fn get(&self, kind: ItemKind) -> Option<egui::TextureId> {
        self.0.get(&kind).copied()
    }
}

/// Disegna l'icona di `kind` nel rettangolo (quadrato colorato se le icone non sono pronte).
pub fn paint_icon(ui: &egui::Ui, icons: Option<&ItemIcons>, kind: ItemKind, rect: egui::Rect) {
    if let Some(id) = icons.and_then(|i| i.get(kind)) {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        ui.painter().image(id, rect, uv, egui::Color32::WHITE);
    } else {
        let c = kind.color().to_srgba();
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(3),
            egui::Color32::from_rgb(
                (c.red * 255.0) as u8,
                (c.green * 255.0) as u8,
                (c.blue * 255.0) as u8,
            ),
        );
    }
}

/// Sistema: registra le icone in egui (una volta sola).
pub fn load_item_icons(
    mut commands: Commands,
    mut contexts: EguiContexts,
    server: Res<AssetServer>,
    icons: Option<Res<ItemIcons>>,
) {
    if icons.is_some() {
        return;
    }
    let map = ItemKind::ALL
        .iter()
        .map(|k| {
            let handle = server.load(format!("textures/icone/{}.png", k.icon()));
            (*k, contexts.add_image(EguiTextureHandle::Strong(handle)))
        })
        .collect();
    commands.insert_resource(ItemIcons(map));
}

/// Filtri correnti.
#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    pub category: Option<Category>,
    pub text: String,
    pub only_equipped: bool,
    pub only_hotbar: bool,
    pub max_weight: f32,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            category: None,
            text: String::new(),
            only_equipped: false,
            only_hotbar: false,
            max_weight: MAX_WEIGHT_FILTER,
        }
    }
}

/// Stato della finestra.
#[derive(Resource, Default)]
pub struct InventoryUi {
    pub filter: Filter,
    /// Si sta scrivendo nella ricerca: il tasto I non deve chiudere la finestra.
    pub typing: bool,
    /// Oggetto da gettare a terra (applicato da `inventory::drop_requested_items`).
    pub drop_request: Option<ItemKind>,
}

/// Minuscole e senza accenti, per confronti e ordinamento.
pub fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            c => c,
        })
        .collect()
}

fn is_equipped(kind: ItemKind, equip: &Equipment) -> bool {
    equip.is_worn(kind)
}

/// Oggetti che passano i filtri, in ordine alfabetico.
pub fn filtered(inv: &Inventory, equip: &Equipment, f: &Filter) -> Vec<ItemStack> {
    let text = normalize(f.text.trim());
    let mut out: Vec<ItemStack> = inv
        .slots
        .iter()
        .filter(|s| f.category.is_none_or(|c| s.kind.category() == c))
        .filter(|s| {
            text.is_empty()
                || normalize(s.kind.name()).contains(&text)
                || normalize(s.kind.description()).contains(&text)
        })
        .filter(|s| !f.only_equipped || is_equipped(s.kind, equip))
        .filter(|s| !f.only_hotbar || inv.hotbar_position(s.kind).is_some())
        .filter(|s| {
            f.max_weight >= MAX_WEIGHT_FILTER
                || s.kind.weight_kg().is_none_or(|w| w <= f.max_weight)
        })
        .cloned()
        .collect();
    out.sort_by_key(|s| normalize(s.kind.name()));
    out
}

fn kg(w: f32) -> String {
    format!("{w:.2} kg").replace('.', ",")
}

fn key_label(slot: usize) -> String {
    if slot == 9 {
        "0".to_string()
    } else {
        (slot + 1).to_string()
    }
}

/// Finestra dell'inventario (stato `Inventory`).
#[allow(clippy::too_many_arguments)]
pub fn inventory_window(
    mut contexts: EguiContexts,
    mut inventory: ResMut<Inventory>,
    mut equip: ResMut<Equipment>,
    mut health: Query<&mut PlayerHealth, With<Player>>,
    icons: Option<Res<ItemIcons>>,
    mut state: ResMut<InventoryUi>,
    mut next: ResMut<NextState<AppState>>,
    mut effects: ResMut<crate::character::ActiveEffects>,
    load: Res<crate::character::Load>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let size = ctx.viewport_rect().size();
    let width = 780.0_f32.min(size.x - 30.0);
    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgba_unmultiplied(10, 12, 20, 250))
        .stroke(egui::Stroke::new(2.0, GOLD))
        .inner_margin(egui::Margin::symmetric(20, 14))
        .corner_radius(egui::CornerRadius::same(10));
    let mut close = false;
    let mut used = None;
    let mut assign = None;
    egui::Area::new(egui::Id::new("inventory_panel"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(width);
            frame.show(ui, |ui| {
                ui.set_min_width(width - 40.0);
                // Caselle e cursori ben visibili sul fondo scuro.
                let w = &mut ui.visuals_mut().widgets.inactive;
                w.bg_fill = egui::Color32::from_rgb(70, 76, 96);
                w.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(170, 178, 196));
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Inventario")
                            .strong()
                            .size(26.0)
                            .color(GOLD),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Chiudi (I)").clicked() {
                            close = true;
                        }
                    });
                });
                ui.separator();
                let f = &mut state.filter;
                ui.horizontal_wrapped(|ui| {
                    if ui.selectable_label(f.category.is_none(), "Tutti").clicked() {
                        f.category = None;
                    }
                    for c in Category::ALL {
                        if ui
                            .selectable_label(f.category == Some(c), c.name())
                            .clicked()
                        {
                            f.category = Some(c);
                        }
                    }
                });
                let mut typing = false;
                ui.horizontal(|ui| {
                    ui.label("Cerca:");
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut f.text)
                            .hint_text("nome o descrizione")
                            .desired_width(220.0),
                    );
                    typing = edit.has_focus();
                    if ui.button("Azzera").clicked() {
                        *f = Filter::default();
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut f.only_equipped, "Solo equipaggiati");
                    ui.checkbox(&mut f.only_hotbar, "Solo nella barra");
                    let label = if f.max_weight >= MAX_WEIGHT_FILTER {
                        "Peso massimo: nessuno".to_string()
                    } else {
                        format!("Peso massimo: {}", kg(f.max_weight))
                    };
                    ui.add(
                        egui::Slider::new(&mut f.max_weight, 0.1..=MAX_WEIGHT_FILTER)
                            .show_value(false)
                            .text(label),
                    );
                });
                ui.separator();
                let filter = f.clone();
                let rows = filtered(&inventory, &equip, &filter);
                egui::ScrollArea::vertical()
                    .max_height((size.y - 300.0).max(120.0))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        if rows.is_empty() {
                            ui.label("Nessun oggetto.");
                        }
                        for row in &rows {
                            let kind = row.kind;
                            ui.horizontal(|ui| {
                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::vec2(38.0, 38.0),
                                    egui::Sense::hover(),
                                );
                                paint_icon(ui, icons.as_deref(), kind, rect);
                                if !kind.description().is_empty() {
                                    resp.on_hover_text(kind.description());
                                }
                                ui.vertical(|ui| {
                                    ui.set_width(170.0);
                                    let mut title =
                                        egui::RichText::new(kind.name()).strong().color(TEXT);
                                    if is_equipped(kind, &equip) {
                                        title = title.color(GOLD);
                                    }
                                    ui.label(title);
                                    ui.label(
                                        egui::RichText::new(kind.category().name())
                                            .small()
                                            .color(egui::Color32::from_rgb(170, 178, 196)),
                                    );
                                });
                                ui.vertical(|ui| {
                                    ui.set_width(58.0);
                                    ui.label(format!("×{}", row.qty));
                                });
                                ui.vertical(|ui| {
                                    ui.set_width(120.0);
                                    if let Some(w) = kind.weight_kg() {
                                        ui.label(kg(w * row.qty as f32));
                                        if row.qty > 1 {
                                            ui.label(
                                                egui::RichText::new(format!("{} l'uno", kg(w)))
                                                    .small()
                                                    .color(egui::Color32::from_rgb(170, 178, 196)),
                                            );
                                        }
                                    }
                                });
                                let action = if kind.heal().is_some()
                                    || kind.carry_potion().is_some()
                                {
                                    Some("Usa")
                                } else if kind.backpack_kg().is_some() || kind.charm_kg().is_some()
                                {
                                    Some(if is_equipped(kind, &equip) {
                                        "Togli"
                                    } else {
                                        "Indossa"
                                    })
                                } else if kind.is_weapon() || kind == ItemKind::Shield {
                                    Some(if is_equipped(kind, &equip) {
                                        "Riponi"
                                    } else {
                                        "Impugna"
                                    })
                                } else {
                                    None
                                };
                                if ui.button("Getta").clicked() {
                                    state.drop_request = Some(kind);
                                }
                                ui.vertical(|ui| {
                                    ui.set_width(96.0);
                                    if let Some(label) = action
                                        && ui
                                            .add(
                                                egui::Button::new(label)
                                                    .wrap_mode(egui::TextWrapMode::Extend),
                                            )
                                            .clicked()
                                    {
                                        used = Some(kind);
                                    }
                                });
                                if kind.can_go_in_hotbar() {
                                    let current = inventory.hotbar_position(kind);
                                    let text = current.map_or("Tasto: —".to_string(), |i| {
                                        format!("Tasto: {}", key_label(i))
                                    });
                                    egui::ComboBox::from_id_salt(kind.icon())
                                        .selected_text(text)
                                        .width(92.0)
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(current.is_none(), "Nessuno")
                                                .clicked()
                                            {
                                                assign = Some((kind, None));
                                            }
                                            for slot in 0..10 {
                                                if ui
                                                    .selectable_label(
                                                        current == Some(slot),
                                                        key_label(slot),
                                                    )
                                                    .clicked()
                                                {
                                                    assign = Some((kind, Some(slot)));
                                                }
                                            }
                                        });
                                }
                            });
                            ui.add_space(3.0);
                        }
                    });
                ui.separator();
                let items: u32 = inventory.slots.iter().map(|s| s.qty).sum();
                ui.horizontal(|ui| {
                    ui.label(format!("Oggetti: {items}"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Peso trasportato: {} / {}",
                                kg(load.weight),
                                kg(load.capacity)
                            ))
                            .strong()
                            .color(GOLD),
                        );
                    });
                });
                state.typing = typing;
            });
        });
    if let Some(kind) = used {
        let hp = health.single_mut().ok();
        use_item(kind, &mut inventory, &mut equip, hp, &mut effects);
    }
    if let Some((kind, slot)) = assign {
        inventory.assign_hotbar(kind, slot);
    }
    if close {
        next.set(AppState::Playing);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inv() -> Inventory {
        let mut i = Inventory::default();
        for k in [
            ItemKind::ShortSword,
            ItemKind::Apple,
            ItemKind::Shield,
            ItemKind::Arrow,
        ] {
            i.add(k, 1);
        }
        i
    }

    fn names(v: &[ItemStack]) -> Vec<&'static str> {
        v.iter().map(|s| s.kind.name()).collect()
    }

    #[test]
    fn items_are_sorted_alphabetically() {
        let v = filtered(&inv(), &Equipment::default(), &Filter::default());
        assert_eq!(
            names(&v),
            ["Frecce", "Mela", "Moneta", "Scudo", "Spada corta"]
        );
    }

    #[test]
    fn search_ignores_case_and_accents() {
        assert_eq!(normalize("PIÙ Città"), "piu citta");
        let f = Filter {
            text: "MELÀ".into(),
            ..Default::default()
        };
        assert_eq!(
            names(&filtered(&inv(), &Equipment::default(), &f)),
            ["Mela"]
        );
    }

    #[test]
    fn category_and_weight_filters() {
        let f = Filter {
            category: Some(Category::Weapons),
            ..Default::default()
        };
        assert_eq!(
            names(&filtered(&inv(), &Equipment::default(), &f)),
            ["Spada corta"]
        );
        let f = Filter {
            max_weight: 0.5,
            ..Default::default()
        };
        // Monete e frecce non hanno peso: il filtro non le esclude.
        assert_eq!(
            names(&filtered(&inv(), &Equipment::default(), &f)),
            ["Frecce", "Mela", "Moneta"]
        );
    }

    #[test]
    fn equipped_and_hotbar_filters() {
        let equip = Equipment {
            weapon: Some(ItemKind::ShortSword),
            ..Default::default()
        };
        let f = Filter {
            only_equipped: true,
            ..Default::default()
        };
        assert_eq!(names(&filtered(&inv(), &equip, &f)), ["Spada corta"]);
        let f = Filter {
            only_hotbar: true,
            ..Default::default()
        };
        assert!(!names(&filtered(&inv(), &equip, &f)).contains(&"Moneta"));
    }
}
