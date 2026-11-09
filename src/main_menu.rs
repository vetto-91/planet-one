//! Menu principale: Continua, Nuova partita, Carica partita, Impostazioni, Esci (ADR 0025).

use crate::AppState;
use crate::character::{CUSTOM_POINTS, Character, Class, STAT_MAX, Stat, Stats};
use crate::save::{
    LoadRequest, SaveData, SaveEntry, folder_name, list_saves, name_taken, saves_root,
};
use crate::settings_ui::SettingsPanel;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Screen {
    #[default]
    Main,
    New,
    Load,
}

/// Stato delle schermate.
#[derive(Resource)]
pub struct MenuUi {
    pub screen: Screen,
    name: String,
    class: Class,
    custom: Stats,
    selected: Option<String>,
    saves: Vec<SaveEntry>,
    message: Option<String>,
    confirm_delete: Option<PathBuf>,
    /// Da ricaricare l'elenco dei salvataggi.
    stale: bool,
}

impl Default for MenuUi {
    fn default() -> Self {
        Self {
            screen: Screen::Main,
            name: String::new(),
            class: Class::Warrior,
            custom: Class::Custom.stats(),
            selected: None,
            saves: Vec::new(),
            message: None,
            confirm_delete: None,
            stale: true,
        }
    }
}

impl MenuUi {
    /// Punti ancora da distribuire nella creazione personalizzata.
    fn points_left(&self) -> i32 {
        let base = i32::from(Class::Custom.stats().get(Stat::Strength));
        let spent: i32 = Stat::ALL
            .iter()
            .map(|s| i32::from(self.custom.get(*s)) - base)
            .sum();
        i32::from(CUSTOM_POINTS) - spent
    }

    fn chosen_stats(&self) -> Stats {
        if self.class == Class::Custom {
            self.custom
        } else {
            self.class.stats()
        }
    }

    pub fn open_load(&mut self) {
        self.screen = Screen::Load;
        self.stale = true;
        self.selected = None;
        self.message = None;
    }

    pub fn refresh(&mut self) {
        self.stale = true;
    }
}

/// Cosa si è scelto nella schermata corrente.
enum Choice {
    Start(Character),
    Load(Box<SaveData>),
    Quit,
}

fn big_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> bool {
    ui.add_enabled_ui(enabled, |ui| {
        ui.add_sized(
            [300.0, 46.0],
            egui::Button::new(egui::RichText::new(text).size(22.0).strong()),
        )
        .clicked()
    })
    .inner
}

fn title(ui: &mut egui::Ui, text: &str, size: f32) {
    ui.label(
        egui::RichText::new(text)
            .strong()
            .size(size)
            .color(crate::ui_style::GOLD),
    );
}

fn main_screen(
    ui: &mut egui::Ui,
    menu: &mut MenuUi,
    settings: &mut SettingsPanel,
) -> Option<Choice> {
    let mut choice = None;
    ui.vertical_centered(|ui| {
        title(ui, "Planet One", 54.0);
        ui.label(crate::version::GAME_VERSION);
        ui.add_space(26.0);
        if let Some(latest) = menu.saves.first() {
            if big_button(ui, "Continua", true) {
                choice = Some(Choice::Load(Box::new(latest.data.clone())));
            }
            ui.label(
                egui::RichText::new(format!("{} — {}", latest.data.name, latest.data.saved_at))
                    .size(14.0),
            );
            ui.add_space(8.0);
        }
        if big_button(ui, "Nuova partita", true) {
            menu.screen = Screen::New;
            menu.message = None;
        }
        ui.add_space(8.0);
        if big_button(ui, "Carica partita", !menu.saves.is_empty()) {
            menu.open_load();
        }
        ui.add_space(8.0);
        if big_button(ui, "Impostazioni", true) {
            settings.open = true;
        }
        ui.add_space(8.0);
        if big_button(ui, "Esci", true) {
            choice = Some(Choice::Quit);
        }
    });
    choice
}

fn new_screen(ui: &mut egui::Ui, menu: &mut MenuUi) -> Option<Choice> {
    let mut choice = None;
    title(ui, "Nuova partita", 32.0);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Nome:");
        ui.add(egui::TextEdit::singleline(&mut menu.name).desired_width(300.0));
    });
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.label("Classe:");
        for class in Class::ALL {
            if ui
                .selectable_label(menu.class == class, class.name())
                .clicked()
            {
                menu.class = class;
            }
        }
    });
    ui.separator();
    let custom = menu.class == Class::Custom;
    let stats = menu.chosen_stats();
    let left = menu.points_left();
    if custom {
        ui.label(format!("Punti da distribuire: {left}"));
    }
    for stat in Stat::ALL {
        ui.horizontal(|ui| {
            ui.add_sized(
                [130.0, 24.0],
                egui::Label::new(egui::RichText::new(stat.name()).strong()),
            );
            let value = stats.get(stat);
            let base = Class::Custom.stats().get(stat);
            if custom
                && ui
                    .add_enabled(value > base, egui::Button::new(" − "))
                    .clicked()
            {
                menu.custom.set(stat, value - 1);
            }
            ui.add_sized([34.0, 24.0], egui::Label::new(value.to_string()));
            if custom
                && ui
                    .add_enabled(left > 0 && value < STAT_MAX, egui::Button::new(" + "))
                    .clicked()
            {
                menu.custom.set(stat, value + 1);
            }
            ui.label(egui::RichText::new(stat.effect()).size(14.0));
        });
    }
    ui.separator();
    let name = menu.name.replace(['|', '\n', '='], " ").trim().to_string();
    let root = saves_root();
    let problem = if name.is_empty() {
        Some("Scrivi un nome.")
    } else if folder_name(&name).is_none() {
        Some("Questo nome non si può usare.")
    } else if name_taken(&root, &name) {
        Some("Esiste già un personaggio con questo nome.")
    } else if custom && left != 0 {
        Some("Distribuisci tutti i punti.")
    } else {
        None
    };
    if let Some(p) = problem {
        ui.label(egui::RichText::new(p).color(egui::Color32::from_rgb(255, 170, 130)));
    }
    ui.horizontal(|ui| {
        if ui
            .add_enabled(problem.is_none(), egui::Button::new("Inizia"))
            .clicked()
        {
            choice = Some(Choice::Start(Character {
                name,
                class: menu.class,
                stats,
            }));
        }
        if ui.button("Indietro").clicked() {
            menu.screen = Screen::Main;
        }
    });
    choice
}

fn load_screen(ui: &mut egui::Ui, menu: &mut MenuUi, in_game: bool) -> Option<Choice> {
    let mut choice = None;
    title(ui, "Carica partita", 32.0);
    ui.add_space(6.0);
    let mut folders: Vec<(String, String)> = Vec::new();
    for s in &menu.saves {
        if !folders.iter().any(|(f, _)| *f == s.folder) {
            folders.push((s.folder.clone(), s.data.name.clone()));
        }
    }
    if menu.selected.is_none() {
        menu.selected = folders.first().map(|(f, _)| f.clone());
    }
    ui.horizontal_wrapped(|ui| {
        for (folder, name) in &folders {
            if ui
                .selectable_label(menu.selected.as_deref() == Some(folder), name)
                .clicked()
            {
                menu.selected = Some(folder.clone());
            }
        }
    });
    ui.separator();
    let list: Vec<SaveEntry> = menu
        .saves
        .iter()
        .filter(|s| menu.selected.as_deref() == Some(&s.folder))
        .cloned()
        .collect();
    egui::ScrollArea::vertical()
        .max_height((ui.ctx().viewport_rect().height() - 260.0).max(120.0))
        .show(ui, |ui| {
            if list.is_empty() {
                ui.label("Nessun salvataggio.");
            }
            for s in &list {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(330.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "{} — {}",
                                s.data.saved_at,
                                s.data.kind.label()
                            ))
                            .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {:02}/{:02} ore {:02.0}:{:02.0}",
                                s.data.place,
                                s.data.day,
                                s.data.month,
                                s.data.hour.floor(),
                                s.data.hour.fract() * 60.0,
                            ))
                            .size(14.0),
                        );
                    });
                    if ui.button("Carica").clicked() {
                        choice = Some(Choice::Load(Box::new(s.data.clone())));
                    }
                    if menu.confirm_delete.as_ref() == Some(&s.path) {
                        if ui.button("Conferma eliminazione").clicked() {
                            let _ = std::fs::remove_file(&s.path);
                            menu.confirm_delete = None;
                            menu.stale = true;
                        }
                    } else if ui.button("Elimina").clicked() {
                        menu.confirm_delete = Some(s.path.clone());
                    }
                });
                ui.add_space(4.0);
            }
        });
    ui.separator();
    if ui
        .button(if in_game { "Chiudi" } else { "Indietro" })
        .clicked()
    {
        menu.screen = Screen::Main;
        menu.confirm_delete = None;
    }
    choice
}

/// Sistema (egui): menu principale, e schermata di caricamento aperta dalla pausa.
pub fn main_menu_ui(
    mut contexts: EguiContexts,
    mut menu: ResMut<MenuUi>,
    mut settings: ResMut<SettingsPanel>,
    state: Res<State<AppState>>,
    mut request: ResMut<LoadRequest>,
    mut next: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    let in_game = *state.get() == AppState::Paused;
    // Le impostazioni si aprono sopra il menu: niente testo sovrapposto.
    if (in_game && menu.screen != Screen::Load) || settings.open {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if menu.stale {
        menu.stale = false;
        menu.saves = list_saves(&saves_root());
    }
    let size = ctx.viewport_rect().size();
    egui::Area::new(egui::Id::new("menu_backdrop"))
        .order(egui::Order::Background)
        .fixed_pos(egui::pos2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size),
                egui::CornerRadius::ZERO,
                egui::Color32::from_rgba_unmultiplied(6, 8, 14, 200),
            );
        });
    let frame = egui::Frame::default()
        .fill(egui::Color32::from_rgba_unmultiplied(10, 12, 20, 235))
        .stroke(egui::Stroke::new(2.0, crate::ui_style::GOLD))
        .inner_margin(egui::Margin::symmetric(30, 20))
        .corner_radius(egui::CornerRadius::same(10));
    let mut choice = None;
    egui::Area::new(egui::Id::new("menu_panel"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.set_width(620.0_f32.min(size.x - 90.0));
                ui.set_max_height(size.y - 60.0);
                choice = match menu.screen {
                    Screen::Main => main_screen(ui, &mut menu, &mut settings),
                    Screen::New => new_screen(ui, &mut menu),
                    Screen::Load => load_screen(ui, &mut menu, in_game),
                };
                if let Some(m) = &menu.message {
                    ui.label(m);
                }
            });
        });
    match choice {
        Some(Choice::Start(character)) => {
            *request = LoadRequest::New(character);
            menu.screen = Screen::Main;
            next.set(AppState::Resetting);
        }
        Some(Choice::Load(data)) => {
            *request = LoadRequest::File(data);
            menu.screen = Screen::Main;
            next.set(AppState::Resetting);
        }
        Some(Choice::Quit) => {
            exit.write(AppExit::Success);
        }
        None => {}
    }
}

/// Entrando nel menu principale l'elenco dei salvataggi va riletto.
fn on_enter_menu(mut menu: ResMut<MenuUi>) {
    menu.screen = Screen::Main;
    menu.refresh();
}

/// Sistema: scritta di attesa mentre si ricostruisce la scena.
pub fn loading_ui(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::Area::new(egui::Id::new("loading"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Caricamento…")
                    .size(34.0)
                    .strong()
                    .color(crate::ui_style::GOLD),
            );
        });
}

pub fn plugin(app: &mut App) {
    app.init_resource::<MenuUi>()
        .add_systems(OnEnter(AppState::MainMenu), on_enter_menu);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_points_are_counted() {
        let mut menu = MenuUi {
            class: Class::Custom,
            ..Default::default()
        };
        assert_eq!(menu.points_left(), i32::from(CUSTOM_POINTS));
        menu.custom.set(Stat::Strength, 12);
        assert_eq!(menu.points_left(), i32::from(CUSTOM_POINTS) - 4);
        assert_eq!(menu.chosen_stats().get(Stat::Strength), 12);
    }

    #[test]
    fn classes_ignore_the_custom_points() {
        let menu = MenuUi {
            class: Class::Scholar,
            ..Default::default()
        };
        assert_eq!(menu.chosen_stats(), Class::Scholar.stats());
    }
}
