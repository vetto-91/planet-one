//! Impostazioni a schede (Gioco, Audio, Video, Controlli): dal menu principale e dal menu Esc.

use crate::AppState;
use crate::audio::AudioSettings;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Game,
    Audio,
    Video,
    Controls,
}

/// Finestra delle impostazioni.
#[derive(Resource, Default)]
pub struct SettingsPanel {
    pub open: bool,
    tab: Tab,
}

#[derive(SystemParam)]
pub struct SettingsCtx<'w> {
    audio: ResMut<'w, AudioSettings>,
    ai: Res<'w, crate::ai::AiState>,
    tts: Res<'w, crate::ai::tts::TtsState>,
    stt: Res<'w, crate::ai::stt::SttState>,
}

fn game_tab(ui: &mut egui::Ui, cx: &mut SettingsCtx) {
    let a = &mut *cx.audio;
    ui.horizontal(|ui| {
        ui.label("Velocità del tempo:");
        for speed in [1u32, 2, 4, 8] {
            if ui
                .selectable_label(a.time_speed == speed, format!("x{speed}"))
                .clicked()
            {
                a.time_speed = speed;
                a.dirty = true;
            }
        }
    });
    if ui
        .add(egui::Slider::new(&mut a.mouse_sensitivity, 0.0005..=0.010).text("Sensibilità mouse"))
        .changed()
    {
        a.dirty = true;
    }
    if ui.checkbox(&mut a.show_fps, "Mostra FPS").changed() {
        a.dirty = true;
    }
    ui.separator();
    ui.label(
        egui::RichText::new("Dialoghi con IA (locale)")
            .strong()
            .color(crate::ui_style::GOLD),
    );
    if ui
        .checkbox(&mut a.ai_enabled, "Dialoghi IA (spegni sui PC lenti)")
        .changed()
    {
        a.dirty = true;
    }
    ui.label(format!("Stato: {}", cx.ai.status.label()));
    if ui
        .checkbox(&mut a.tts_enabled, "Voce IA (Piper locale)")
        .changed()
    {
        a.dirty = true;
    }
    ui.label(format!("Voce: {}", cx.tts.status.label()));
    if ui
        .checkbox(&mut a.stt_enabled, "Microfono (Whisper locale)")
        .changed()
    {
        a.dirty = true;
    }
    ui.label(format!("Microfono: {}", cx.stt.status.label()));
    ui.label("Modelli e voci si cambiano dalla shell (tasto \\).");
}

fn audio_tab(ui: &mut egui::Ui, a: &mut AudioSettings) {
    let volume = |ui: &mut egui::Ui, value: &mut f32, text: &str, dirty: &mut bool| {
        let mut pct = *value * 100.0;
        if ui
            .add(
                egui::Slider::new(&mut pct, 0.0..=100.0)
                    .text(text)
                    .fixed_decimals(0),
            )
            .changed()
        {
            *value = pct / 100.0;
            *dirty = true;
        }
    };
    let mut dirty = a.dirty;
    volume(ui, &mut a.music_volume, "Volume musica %", &mut dirty);
    volume(ui, &mut a.ambient_volume, "Volume ambiente %", &mut dirty);
    volume(ui, &mut a.voice_volume, "Volume voci %", &mut dirty);
    a.dirty = dirty;
}

fn video_tab(ui: &mut egui::Ui, a: &mut AudioSettings) {
    if ui
        .checkbox(&mut a.fullscreen, "Schermo intero (F11)")
        .changed()
    {
        a.dirty = true;
    }
    ui.separator();
    crate::graphics::graphics_ui(ui, a);
}

fn controls_tab(ui: &mut egui::Ui) {
    for line in [
        "W/A/S/D o frecce — movimento",
        "Shift — corsa, Spazio — salto",
        "Mouse — camera, V — prima persona, tasto destro — prima persona temporanea",
        "E / Invio — interagisci e dialoga",
        "I — inventario, C — personaggio, J — diario, M — mappa, T — aspetta",
        "1–0 — barra degli oggetti, TAB (tenuto) — obiettivi",
        "F5 — salvataggio rapido, F9 — carica il salvataggio rapido",
        "F11 — schermo intero, Esc — menu, \\ — shell dei trucchi",
    ] {
        ui.label(line);
    }
}

/// Sistema (egui): finestra a schede.
pub fn settings_window(
    mut contexts: EguiContexts,
    mut panel: ResMut<SettingsPanel>,
    mut cx: SettingsCtx,
) {
    if !panel.open {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let mut open = true;
    egui::Window::new("Impostazioni")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(520.0_f32.min(ctx.viewport_rect().width() - 70.0));
            ui.horizontal(|ui| {
                for (tab, name) in [
                    (Tab::Game, "Gioco"),
                    (Tab::Audio, "Audio"),
                    (Tab::Video, "Video"),
                    (Tab::Controls, "Controlli"),
                ] {
                    if ui.selectable_label(panel.tab == tab, name).clicked() {
                        panel.tab = tab;
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height((ctx.viewport_rect().height() - 160.0).max(120.0))
                .show(ui, |ui| match panel.tab {
                    Tab::Game => game_tab(ui, &mut cx),
                    Tab::Audio => audio_tab(ui, &mut cx.audio),
                    Tab::Video => video_tab(ui, &mut cx.audio),
                    Tab::Controls => controls_tab(ui),
                });
        });
    panel.open = open;
}

/// Sistema: applica sensibilità, FPS a schermo e modalità finestra.
pub fn apply_settings(
    audio: Res<AudioSettings>,
    mut orbit: ResMut<crate::camera::OrbitCamera>,
    mut show_fps: ResMut<crate::menu::ShowFps>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    mut last_fullscreen: Local<Option<bool>>,
) {
    orbit.sensitivity = audio.mouse_sensitivity;
    show_fps.0 = audio.show_fps;
    if *last_fullscreen != Some(audio.fullscreen)
        && let Ok(mut window) = windows.single_mut()
    {
        use bevy::window::{MonitorSelection, VideoModeSelection, WindowMode};
        window.mode = if audio.fullscreen {
            WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
        } else {
            WindowMode::Windowed
        };
        *last_fullscreen = Some(audio.fullscreen);
    }
}

/// Sistema: chiudendo menu e pausa si chiude anche la finestra.
pub fn close_on_play(mut panel: ResMut<SettingsPanel>, mut menu: ResMut<crate::main_menu::MenuUi>) {
    panel.open = false;
    menu.screen = crate::main_menu::Screen::Main;
}

pub fn plugin(app: &mut App) {
    app.init_resource::<SettingsPanel>()
        .add_systems(Update, apply_settings)
        .add_systems(OnEnter(AppState::Playing), close_on_play);
}
