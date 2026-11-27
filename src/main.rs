//! Planet One — RPG 3D open source, cel-shaded.
//!
//! Fase 2.1: character controller cinematico (Avian MoveAndSlide),
//! terreno 100 m, debug wireframe collider (F3).

// In release su Windows niente finestra di console dietro il gioco.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod ai;
mod ambience;
mod animals;
mod animation;
mod archery;
mod audio;
mod autopilot;
mod awakening;
mod bells;
mod bugreport;
mod buildings;
mod camera;
mod character;
mod combat;
mod companions;
mod containers;
mod crossing;
mod cutscene;
mod daynight;
mod debug_place;
mod devtools;
mod diagnosis;
mod diving;
mod entities;
mod equipment;
mod figure;
mod graphics;
mod house;
mod hud;
mod hut;
mod interiors;
mod inventory;
mod inventory_ui;
mod island_events;
mod island_life;
mod karma;
mod lanterns;
mod localtime;
mod magic;
mod main_menu;
mod map;
mod menu;
mod nature;
mod par;
mod pbr;
mod perf;
mod places;
mod player;
mod props;
mod quests;
mod region;
mod render_scale;
mod save;
mod scene;
mod seasons;
mod settings_ui;
mod sfx;
mod shell;
mod shop;
mod side_scenes;
mod skills;
mod sky;
mod tailing;
mod terrain;
mod thievery;
mod trees;
mod ui_style;
mod version;
mod village;
mod village_life;
mod villagers;
mod vitals;
mod voice;
#[cfg(test)]
mod walk_test;
mod water;
mod weather;
mod world_props;

use avian3d::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::gizmos::config::GizmoConfigStore;
use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use quests::denaro_rubato as quest;

/// Se true, mostra wireframe globale delle mesh (diagnostica).
/// Default false per non coprire la scena in produzione.
const DEBUG_WIREFRAME: bool = false;

/// Stati principali dell'applicazione.
///
/// - `Playing`: gameplay attivo (movimento, camera, fisica).
/// - `Paused`: menu Esc aperto; i sistemi di gameplay sono sospesi.
/// - `Resetting`: transitorio: in un singolo frame despawn e ricostruzione
///   della scena da zero; evita che sistemi di gameplay tentino di scrivere
///   su entità che stanno per essere distrutte.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Menu principale, all'avvio.
    #[default]
    MainMenu,
    /// Attesa della scena dopo una nuova partita o un caricamento.
    Loading,
    /// Gameplay attivo.
    Playing,
    /// Menu aperto (Esc): gameplay sospeso.
    Paused,
    /// Diario delle quest aperto (J): gameplay sospeso, cursore libero.
    Journal,
    /// Finestra "Aspetta" aperta (T): gameplay sospeso, cursore libero.
    Wait,
    /// Mappa aperta (M): vista dall'alto, gameplay sospeso, cursore libero.
    Map,
    /// Chat con la IA locale dopo un dialogo: gameplay sospeso, cursore libero.
    Chat,
    /// Inventario aperto (I): gioco sospeso, cursore libero.
    Inventory,
    /// Pannello del personaggio (C): gioco sospeso, cursore libero.
    Character,
    /// Mobile aperto (casse, armadi…): gioco sospeso, cursore libero.
    Container,
    /// Scasso di una serratura (gioco dei perni): gioco sospeso.
    Lockpick,
    /// Pannelli del risveglio sulla spiaggia (età, indole, nome): gioco sospeso.
    Awakening,
    /// Shell dei trucchi aperta (\\): gioco sospeso, cursore libero.
    Shell,
    /// Negozio aperto (acquisto con le monete): gameplay sospeso, cursore libero.
    Shop,
    /// Nota di una segnalazione (F12) da scrivere: gioco sospeso, cursore libero.
    BugNote,
    /// Reset della scena in corso (un frame, non interattivo).
    Resetting,
}

fn main() -> AppExit {
    // Lanciato come guardiano dell'Ollama del gioco: fa solo quello (ADR 0063).
    if ai::server::run_guard() {
        return AppExit::Success;
    }
    // `PLANET_ONE_HEADLESS=1`: niente finestra (né winit), la camera disegna in un'immagine sulla
    // GPU e il gioco gira a 60 Hz; serve al pilota automatico quando lo schermo non è disponibile.
    let headless = devtools::headless();
    let defaults = DefaultPlugins
        .set(bevy::audio::AudioPlugin {
            default_spatial_scale: bevy::audio::SpatialScale::new(audio::SPATIAL_SCALE),
            ..default()
        })
        .set(bevy::log::LogPlugin {
            fmt_layer: |_| {
                Some(Box::new(
                    bevy::log::tracing_subscriber::fmt::layer()
                        .with_timer(localtime::LogClock)
                        // Copia delle ultime righe per F12 «Segnala bug».
                        .with_writer(bugreport::tee),
                ))
            },
            ..default()
        })
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Planet One".into(),
                // `PLANET_ONE_SIZE=1280x720`: finestra fissa per screenshot ripetibili.
                resolution: devtools::window_size().into(),
                // Avvio a schermo intero di default; F11 torna a finestra.
                // Le prove automatiche nascono già a finestra (e con la dimensione chiesta).
                mode: if devtools::skip_menu() {
                    bevy::window::WindowMode::Windowed
                } else {
                    bevy::window::WindowMode::Fullscreen(
                        bevy::window::MonitorSelection::Current,
                        bevy::window::VideoModeSelection::Current,
                    )
                },
                ..default()
            }),
            // In Bevy 0.20 Esc non chiude più la finestra di default:
            // lo lasciamo gestire solo al menu di gioco.
            ..default()
        });
    let defaults = if headless {
        defaults
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                close_when_requested: false,
                ..default()
            })
            // Senza finestra non arrivano tastiera né mouse; niente nemmeno dai gamepad:
            // in prova comanda solo il pilota.
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::gilrs::GilrsPlugin>()
            .add(bevy::app::ScheduleRunnerPlugin::run_loop(
                devtools::headless_frame(),
            ))
    } else {
        defaults
    };
    let exit = App::new()
        .add_plugins((
            defaults,
            PhysicsPlugins::default(),
            // Debug fisica Avian (collider wireframe): disabilitato di
            // default, attivabile con F3. Utile per confrontare mesh e collider.
            PhysicsDebugPlugin,
            EguiPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        // Stato applicativo + risorse.
        .insert_state(if devtools::skip_menu() {
            AppState::Playing
        } else {
            AppState::MainMenu
        })
        // Wireframe delle mesh disattivato di default (era diagnostico);
        // riattivabile con DEBUG_WIREFRAME.
        .insert_resource(WireframeConfig {
            global: DEBUG_WIREFRAME,
            ..default()
        })
        .insert_resource(ClearColor(Color::srgb(0.02, 0.03, 0.05)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.9, 0.92, 0.98),
            brightness: 60.0,
            ..Default::default()
        })
        .init_resource::<camera::OrbitCamera>()
        .init_resource::<menu::ShowFps>()
        .init_resource::<menu::HudHidden>()
        .init_resource::<quests::Tracked>()
        .init_resource::<entities::DialogState>()
        .init_resource::<entities::NearestInteract>()
        .init_resource::<inventory::Inventory>()
        .init_resource::<inventory_ui::InventoryUi>()
        .init_resource::<quest::QuestState>()
        .init_resource::<quests::QuestTexts>()
        .init_resource::<quests::Barks>()
        .init_resource::<quests::missions::Missions>()
        .init_resource::<menu::JournalTab>()
        .add_plugins(map::plugin)
        .add_plugins(ai::plugin)
        .add_plugins(village::plugin)
        .add_plugins(villagers::plugin)
        .add_plugins(shell::plugin)
        .add_plugins(autopilot::plugin)
        .add_plugins(perf::plugin)
        .add_plugins(diagnosis::plugin)
        .init_resource::<bugreport::BugReport>()
        .add_systems(Update, (bugreport::report_bug, bugreport::move_shots))
        .add_plugins(containers::plugin)
        .add_plugins(character::plugin)
        .add_plugins(skills::plugin)
        .add_plugins(vitals::plugin)
        .add_plugins(thievery::plugin)
        .add_plugins(karma::plugin)
        .add_plugins(magic::plugin)
        .add_plugins(house::plugin)
        .add_plugins(archery::plugin)
        .add_plugins(island_life::plugin)
        .add_plugins(lanterns::plugin)
        .add_plugins(side_scenes::plugin)
        .add_plugins(island_events::plugin)
        .add_plugins(crossing::plugin)
        .add_plugins(diving::plugin)
        .add_systems(Update, hut::copper_lid)
        .add_systems(Update, voice::bark_voice.before(voice::drive_voice))
        .add_plugins(awakening::plugin)
        .add_plugins(cutscene::plugin)
        .add_plugins(companions::plugin)
        .add_plugins(tailing::plugin)
        .add_plugins(sfx::plugin)
        .add_plugins(water::plugin)
        .add_plugins(nature::plugin)
        .add_plugins(save::plugin)
        .add_plugins(settings_ui::plugin)
        .add_plugins(main_menu::plugin)
        .add_plugins(shop::plugin)
        .add_plugins(bells::plugin)
        .add_plugins(ambience::plugin)
        .add_plugins(terrain::plugin)
        .init_resource::<camera::FlyCamera>()
        .add_systems(
            PreUpdate,
            (debug_place::shortcuts, camera::fly_keys)
                .chain()
                .after(bevy::input::InputSystems)
                .after(autopilot::run_autopilot),
        )
        .add_plugins(animals::plugin)
        .add_plugins(places::plugin)
        .add_plugins(village_life::plugin)
        .add_plugins(seasons::plugin)
        .add_plugins(weather::plugin)
        .add_plugins(figure::plugin)
        .add_plugins(animation::plugin)
        .add_plugins(graphics::plugin)
        .add_plugins(render_scale::plugin)
        .init_resource::<debug_place::DebugPlace>()
        .init_resource::<world_props::Homes>()
        .init_resource::<hud::Occluded>()
        .init_resource::<audio::AudioSettings>()
        .init_resource::<voice::VoiceQueue>()
        .init_resource::<audio::MusicDuck>()
        .add_message::<hut::PropToggle>()
        .init_resource::<daynight::GameClock>()
        .init_resource::<pbr::PbrMaterials>()
        .insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 })
        .init_resource::<daynight::WaitUi>()
        .init_resource::<combat::ContactDamageCd>()
        .init_resource::<combat::SwingTimer>()
        .init_resource::<inventory::Equipment>()
        // Asset del terreno: heightmap PNG + handle del mesh generato.
        .add_systems(
            Startup,
            (
                scene::spawn_camera,
                scene::spawn_world_startup,
                daynight::spawn_sky_discs,
                audio::start_music,
                terrain::setup_assets,
            ),
        )
        // All'avvio il wireframe dei collider è DISATTIVO; F3 lo attiva
        // (utile in sviluppo per confrontare mesh e collider del terreno).
        .add_systems(Startup, disable_physics_debug_default)
        // Prove automatiche sempre a finestra; altrimenti vale l'impostazione salvata.
        .add_systems(Startup, |mut audio: ResMut<audio::AudioSettings>| {
            if devtools::skip_menu() || std::env::var_os("PLANET_ONE_WINDOWED").is_some() {
                audio.fullscreen = false;
            }
        })
        // Ricostruzione del terreno quando la heightmap è disponibile o cambia.
        .add_systems(
            Update,
            (
                terrain::update_terrain,
                sky::apply_sky_texture,
                daynight::update_sky,
                devtools::apply_debug_env,
                hut::handle_prop_toggle,
                hut::update_prop_prompts,
                hut::keeper_routine,
                hut::animate_hinges,
                hut::animate_sliders,
                hut::apply_house_light,
                hut::stove_light,
                hut::emit_smoke,
                hut::update_smoke,
                props::snap_to_ground,
                audio::update_music_duck,
                audio::apply_music_volume,
                audio::save_settings,
                voice::drive_voice,
                voice::dialog_voice,
                daynight::tick_clock.run_if(in_state(AppState::Playing)),
            ),
        )
        // Fisica e movimento del player in FixedUpdate (delta stabile):
        // ground-check, input/gravita'/salto, MoveAndSlide, facing in un'unica
        // pipeline fixed: salto e contatto restano coerenti anche in corsa.
        .add_systems(
            FixedUpdate,
            (
                player::update_grounded,
                player::movement,
                player::run_move_and_slide,
                player::apply_fall_damage,
                player::facing,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        // Nemico in FixedUpdate: stesso pipeline del player per MoveAndSlide.
        .add_systems(
            FixedUpdate,
            entities::enemy_patrol.run_if(in_state(AppState::Playing)),
        )
        .add_systems(Update, entities::smooth_enemies)
        // Camera in Update (dopo il FixedUpdate): segue il pg con delta reale.
        .add_systems(
            Update,
            (
                camera::rmb_raw,
                camera::orbit,
                camera::mode_input,
                camera::follow,
                camera::tag_player_head,
                camera::player_visibility,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        // Interazione (E) e avanzamento dialogo in Update, solo in Playing.
        .add_systems(
            Update,
            (
                quest::quest_on_dialog_close,
                quest::quest_on_thief_death,
                quest::collect_quest_coins,
                quest::update_thief_hint,
                quest::place_quest_thief,
                quest::tick_title,
                quest::thief_notice_player,
                quests::tick_barks,
                entities::dialog_walk_away.before(entities::interact),
                // advance_dialog PRIMA di interact: la E che apre il dialogo
                // non deve anche avanzarlo nello stesso frame.
                entities::advance_dialog.before(entities::interact),
                entities::interact,
                hud::update_occlusion,
                inventory::collect_pickups,
                inventory::use_hotbar_items,
                inventory::drop_requested_items,
                combat::player_attack,
                combat::contact_damage,
                combat::player_death.after(combat::contact_damage),
                combat::handle_enemy_death,
                combat::enemy_respawn_tick,
            )
                .run_if(in_state(AppState::Playing)),
        )
        // Menu e overlay FPS: il disegno della UI egui DEVE girare nella
        // schedule EguiPrimaryContextPass (in egui 0.36 i font sono
        // disponibili solo durante il pass).
        .add_systems(
            EguiPrimaryContextPass,
            (
                ui_style::apply_ui_scale,
                inventory_ui::load_item_icons,
                menu::ui,
                debug_place::debug_ui,
                inventory_ui::inventory_window.run_if(in_state(AppState::Inventory)),
                shell::shell_panel.run_if(in_state(AppState::Shell)),
                bugreport::bug_note_window.run_if(in_state(AppState::BugNote)),
                containers::container_window.run_if(in_state(AppState::Container)),
                thievery::lockpick_window.run_if(in_state(AppState::Lockpick)),
                awakening::awakening_window.run_if(in_state(AppState::Awakening)),
                character::character_window.run_if(in_state(AppState::Character)),
                main_menu::main_menu_ui.run_if(|s: Res<State<AppState>>| {
                    matches!(s.get(), AppState::MainMenu | AppState::Paused)
                }),
                main_menu::loading_ui.run_if(in_state(AppState::Loading)),
                settings_ui::settings_window,
                save::draw_notice,
                cutscene::draw_cutscene,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                quests::missions::track_missions,
                quest::thief_parley.after(entities::interact),
                thievery::pickpocket.after(quest::thief_parley),
                thievery::pick_enemy_pockets.after(thievery::pickpocket),
                karma::give_gift.after(thievery::pickpocket),
                karma::meet_and_confide.after(entities::interact),
                karma::judge_quest,
                quest::thief_choice,
            )
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(Update, debug_place::sync_markers)
        .add_systems(Update, inventory::apply_grants)
        .init_resource::<player::Dash>()
        .add_systems(
            Update,
            player::trigger_dash.run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            Update,
            (
                hut::update_los_blockers,
                terrain::terrain_debug_gizmos,
                combat::update_arrows.run_if(in_state(AppState::Playing)),
                combat::enemy_ai.run_if(in_state(AppState::Playing)),
                inventory::attach_pickup_icons,
                inventory::face_pickup_icons,
                combat::update_enemy_arrows
                    .before(combat::player_death)
                    .run_if(in_state(AppState::Playing)),
                equipment::update_held_items,
                equipment::stow_weapon_while_running,
                equipment::first_person_weapon.after(equipment::update_held_items),
            ),
        )
        .add_systems(
            Update,
            equipment::carried_lantern.run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            Update,
            (
                world_props::sync_world_props,
                world_props::update_street_lamps,
                world_props::update_lighthouse,
                world_props::update_interior_lights,
                world_props::interior_light_shadows,
            ),
        )
        // Toggle Esc: bidirezionale (Playing ↔ Paused), esclude Resetting.
        // (dopo advance_dialog e prima di interact: vedi `DialogState::block_interact`)
        .add_systems(
            Update,
            menu::toggle
                .after(entities::advance_dialog)
                .before(entities::interact),
        )
        .add_systems(Update, menu::toggle_hud.run_if(in_state(AppState::Playing)))
        // Menu e pannelli fermano il tempo del mondo (ADR 0059).
        .add_systems(Update, menu::pause_world.run_if(state_changed::<AppState>))
        // F3: toggle debug wireframe dei collider (PhysicsDebugPlugin).
        .add_systems(
            Update,
            (
                toggle_physics_debug.run_if(in_state(AppState::Playing)),
                toggle_fullscreen,
                apply_cursor_for_mode,
                warp_cursor_to_center,
                apply_cursor_warp,
                log_input_events,
            ),
        )
        .init_resource::<map::SavedSea>()
        .add_systems(OnEnter(AppState::Map), (map::enter_map, map::sea_on_map))
        .add_systems(OnExit(AppState::Map), (map::exit_map, map::sea_off_map))
        .add_systems(
            Update,
            (map::map_zoom, map::map_lighting.after(daynight::update_sky))
                .run_if(in_state(AppState::Map)),
        )
        .add_systems(OnEnter(AppState::Resetting), scene::reset_scene)
        // Il codice d'uscita conta per il pilota automatico (1 = verifiche fallite).
        .run();
    // L'Ollama avviato dal gioco si chiude con lui (ADR 0063).
    ai::server::stop();
    exit
}

/// Disattiva di default il wireframe dei collider all'avvio.
/// F3 lo riattiva a runtime per confrontare mesh e collider.
fn disable_physics_debug_default(mut store: ResMut<GizmoConfigStore>) {
    store
        .config_mut::<avian3d::prelude::PhysicsGizmos>()
        .0
        .enabled = false;
}

/// Applica le opzioni del cursore in base alla modalità finestra:
/// - fullscreen (F11): cursore nascosto e bloccato (grab) → tutti gli input restano nel gioco.
/// - finestra: cursore visibile e libero → può uscire e interagire col sistema.
fn apply_cursor_for_mode(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut cursor_opts: Query<&mut bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>,
    app_state: Res<State<AppState>>,
    debug: Res<debug_place::DebugPlace>,
    keys: Res<ButtonInput<KeyCode>>,
    mut last: Local<(Option<bevy::window::WindowMode>, bool)>,
) {
    use bevy::window::{CursorGrabMode, WindowMode};
    let Ok(window) = windows.single() else {
        return;
    };
    let fullscreen = !matches!(window.mode, WindowMode::Windowed);
    // A schermo intero catturiamo SOLO in Playing: in Paused (menu) il cursore
    // deve tornare visibile e libero per cliccare i bottoni.
    // Con il debug F4 il mirino sceglie i punti; Ctrl tenuto libera il cursore per il pannello.
    let capture = fullscreen
        && *app_state.get() == AppState::Playing
        && !debug_place::cursor_free(debug.on, &keys);
    if *last == (Some(window.mode), capture) {
        return;
    }
    *last = (Some(window.mode), capture);
    let Ok(mut opts) = cursor_opts.single_mut() else {
        return;
    };
    if capture {
        opts.visible = false;
        // Windows non supporta Locked: Confined (il warp al centro mantiene la rotazione
        // infinita). Su macOS Locked funziona; su X11 ricade su Confined.
        opts.grab_mode = if cfg!(target_os = "windows") {
            CursorGrabMode::Confined
        } else {
            CursorGrabMode::Locked
        };
        info!("[CURSOR] cattura ON (fullscreen+playing): nascosto+bloccato");
    } else {
        opts.visible = true;
        opts.grab_mode = CursorGrabMode::None;
        if fullscreen {
            info!("[CURSOR] fullscreen ma menu aperto: cursore libero");
        } else {
            info!("[CURSOR] finestra: cursore visibile e libero");
        }
    }
}

/// In fullscreen + Playing, riporta il cursore al centro della finestra ogni
/// frame: su X11 Confined blocca ai bordi, ma il delta del movimento continua
/// ad arrivare, così la rotazione yaw resta infinita (360°).
fn warp_cursor_to_center(
    mut commands: Commands,
    windows: Query<(Entity, &Window), With<bevy::window::PrimaryWindow>>,
    app_state: Res<State<AppState>>,
    debug: Res<debug_place::DebugPlace>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok((entity, window)) = windows.single() else {
        return;
    };
    let fullscreen = !matches!(window.mode, bevy::window::WindowMode::Windowed);
    if !fullscreen
        || *app_state.get() != AppState::Playing
        || debug_place::cursor_free(debug.on, &keys)
    {
        return;
    }
    let size = window.resolution.size();
    // set_cursor_position e' un metodo su &mut Window: usiamo un EntityCommand
    // via Commands per ottenere il borrow mutabile in modo sicuro.
    commands
        .entity(entity)
        .insert(CursorWarpRequest(size / 2.0));
}

/// Componente temporaneo: posizione a cui warpare il cursore (centro finestra).
#[derive(Component)]
struct CursorWarpRequest(Vec2);

/// Applica il warp del cursore richiesto nel frame precedente.
fn apply_cursor_warp(
    mut commands: Commands,
    mut windows: Query<(&mut Window, Entity), With<bevy::window::PrimaryWindow>>,
    requests: Query<(Entity, &CursorWarpRequest)>,
) {
    for (req_entity, req) in &requests {
        if let Ok((mut window, win_entity)) = windows.get_mut(req_entity) {
            window.set_cursor_position(Some(req.0));
            let _ = win_entity;
        }
        commands.entity(req_entity).remove::<CursorWarpRequest>();
    }
}

/// Logga TUTTI gli eventi di input (tastiera, mouse, wheel, finestra) con
/// prefisso [INPUT]/[EVENT] per debug via grep. Attivo sempre in dev.
fn log_input_events(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut mouse_btn: MessageReader<bevy::input::mouse::MouseButtonInput>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut cursor_moved: MessageReader<bevy::window::CursorMoved>,
) {
    for ev in keys.read() {
        info!("[INPUT] key {:?} {:?}", ev.key_code, ev.state);
    }
    for ev in mouse_btn.read() {
        info!("[INPUT] mouse btn {:?} {:?}", ev.button, ev.state);
    }
    // Movimenti del mouse e del cursore: decine per frame, a livello debug per non intasare il
    // log (tasti, click e rotella restano [INPUT] come prima; ADR 0056).
    for ev in mouse_motion.read() {
        debug!("[INPUT] mouse motion delta={:?}", ev.delta);
    }
    for ev in wheel.read() {
        info!("[INPUT] wheel {:?}", ev.unit);
    }
    for ev in cursor_moved.read() {
        debug!(
            "[EVENT] cursor moved to {:?} delta={:?}",
            ev.position, ev.delta
        );
    }
}

/// F11: toggle fullscreen (usa il sistema operativo per catturare input).
fn toggle_fullscreen(keys: Res<ButtonInput<KeyCode>>, mut audio: ResMut<audio::AudioSettings>) {
    if keys.just_pressed(KeyCode::F11) {
        audio.fullscreen = !audio.fullscreen;
        audio.dirty = true;
        info!("fullscreen: {}", audio.fullscreen);
    }
}

/// Toggle con F3 del rendering debug dei collider (wireframe arancione).
/// Utile in fase di sviluppo per confrontare il collider fisico con la mesh.
fn toggle_physics_debug(
    keys: Res<ButtonInput<KeyCode>>,
    cheats: Res<shell::Cheats>,
    mut store: ResMut<GizmoConfigStore>,
) {
    if cheats.debug_keys && keys.just_pressed(KeyCode::F3) {
        let cfg = store.config_mut::<avian3d::prelude::PhysicsGizmos>();
        cfg.0.enabled = !cfg.0.enabled;
        info!("physics debug wireframe: {}", cfg.0.enabled);
    }
}
