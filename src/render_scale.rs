//! Scala di rendering (ADR 0058): sotto il 100% la camera 3D disegna su un'immagine più piccola,
//! che una seconda camera stende sulla finestra con un filtro lineare.
//!
//! - Al 100% non esiste nulla di tutto questo: la camera 3D disegna sulla finestra come sempre.
//! - L'interfaccia (egui) va sulla camera finale, quindi resta a piena risoluzione. bevy_egui
//!   aggancia il contesto principale alla prima camera: qui lo si sposta, componenti e texture
//!   comprese, senza perdere lo stato (finestre aperte, scala, caratteri già caricati).
//! - L'immagine ha come fattore di scala quello della finestra per la scala di rendering: la sua
//!   misura logica è quella della finestra, così `world_to_viewport` e `viewport_to_world` danno
//!   le stesse coordinate di prima (targhette, fumetti, mirino del debug).
//! - Sulla mappa (M) la scala torna al 100% (l'immagine prende la misura della finestra).
//! - Senza finestra (`PLANET_ONE_HEADLESS`) la scala non si usa: la camera disegna già su
//!   un'immagine a piena risoluzione per le foto del pilota.

use crate::AppState;
use crate::audio::AudioSettings;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget, ShadowLodOrigin};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::sync_world::RenderEntity;
use bevy::render::{Extract, ExtractSchedule, RenderApp};
use bevy::window::PrimaryWindow;
use bevy_egui::input::{
    EguiContextImeState, EguiContextPointerPosition, EguiContextPointerTouchId,
};
use bevy_egui::render::EguiCameraView;
use bevy_egui::{
    EguiContext, EguiContextSettings, EguiFullOutput, EguiInput, EguiManagedTextures, EguiOutput,
    EguiRenderOutput, EguiSchedule, EguiZoomFactor, PrimaryEguiContext,
};

/// Livello di disegno riservato all'immagine stesa sulla finestra: la vede solo la camera finale.
const FRAME_LAYER: usize = 30;

/// Catena attiva sotto il 100%.
#[derive(Resource)]
struct Scaled {
    /// Camera finale (2D, sulla finestra) con l'interfaccia.
    view: Entity,
    /// Rettangolo con l'immagine della scena.
    frame: Entity,
    /// Misura dell'immagine in pixel.
    size: UVec2,
}

/// Camera finale.
#[derive(Component)]
struct ScaledView;

/// Misura dell'immagine per una finestra di `physical` pixel a `percent`%.
pub fn scaled_size(physical: UVec2, percent: u8) -> UVec2 {
    let f = f32::from(percent.clamp(1, 100)) / 100.0;
    UVec2::new(
        ((physical.x as f32 * f).round() as u32).max(1),
        ((physical.y as f32 * f).round() as u32).max(1),
    )
}

/// Componenti di bevy_egui che formano il contesto principale (vedi `EguiContext` e
/// `PrimaryEguiContext`): si spostano tutti insieme da una camera all'altra.
type EguiParts = (
    (
        PrimaryEguiContext,
        EguiContext,
        EguiSchedule,
        EguiContextSettings,
    ),
    (EguiInput, EguiZoomFactor, EguiContextPointerPosition),
    (EguiContextPointerTouchId, EguiContextImeState),
    (EguiFullOutput, EguiRenderOutput, EguiOutput),
);

/// Sposta il contesto egui da `from` a `to`, con le texture che egui vi ha già caricato
/// (i caratteri vengono mandati una volta sola: senza di loro il testo sparirebbe).
fn move_egui(world: &mut World, from: Entity, to: Entity) -> bool {
    let Some(parts) = world.entity_mut(from).take::<EguiParts>() else {
        warn!("[GRAFICA] contesto egui non trovato sulla camera: scala di rendering non applicata");
        return false;
    };
    world.entity_mut(to).insert(parts);
    if let Some(mut textures) = world.get_resource_mut::<EguiManagedTextures>() {
        let moved: Vec<_> = textures
            .keys()
            .filter(|(e, _)| *e == from)
            .copied()
            .collect();
        for key in moved {
            if let Some(t) = textures.remove(&key) {
                textures.insert((to, key.1), t);
            }
        }
    }
    true
}

fn new_frame_image(images: &mut Assets<Image>, size: UVec2) -> Handle<Image> {
    let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
    image.sampler = ImageSampler::linear();
    images.add(image)
}

/// Cosa deve fare la catena in questo fotogramma.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Plan {
    /// Disegno diretto sulla finestra (100%).
    Direct,
    /// Immagine di `size` pixel, fattore di scala della camera 3D, misura logica della finestra.
    Scaled {
        size: UVec2,
        scale_factor: f32,
        logical: Vec2,
    },
}

/// Sistema (First): decide la catena e, solo se cambia, la ricostruisce in un colpo solo.
fn apply_render_scale(
    mut commands: Commands,
    settings: Res<AudioSettings>,
    state: Res<State<AppState>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scaled: Option<Res<Scaled>>,
) {
    let percent = settings.gfx.render_scale;
    let plan = if percent >= 100 {
        Plan::Direct
    } else {
        let Ok(window) = windows.single() else {
            return;
        };
        let physical = window.physical_size();
        if physical.x == 0 || physical.y == 0 {
            // Finestra ridotta a icona: si lascia tutto com'è.
            return;
        }
        // La mappa resta a piena risoluzione.
        let percent = if *state.get() == AppState::Map {
            100
        } else {
            percent
        };
        let size = scaled_size(physical, percent);
        Plan::Scaled {
            size,
            scale_factor: window.scale_factor() * size.y as f32 / physical.y as f32,
            logical: window.size(),
        }
    };
    match (&plan, scaled.as_deref()) {
        (Plan::Direct, None) => {}
        (Plan::Scaled { size, .. }, Some(s)) if *size == s.size => {}
        _ => commands.queue(move |world: &mut World| rebuild(world, plan)),
    }
}

fn rebuild(world: &mut World, plan: Plan) {
    let mut cams = world.query_filtered::<Entity, (With<Camera3d>, Without<ScaledView>)>();
    let Some(cam3d) = cams.iter(world).next() else {
        return;
    };
    let current = world.remove_resource::<Scaled>();
    match (plan, current) {
        (Plan::Direct, Some(s)) => {
            // Ritorno al disegno diretto: tutto come prima della scala.
            move_egui(world, s.view, cam3d);
            world
                .entity_mut(cam3d)
                .insert(RenderTarget::default())
                .remove::<ShadowLodOrigin>();
            world.despawn(s.view);
            world.despawn(s.frame);
            info!("[GRAFICA] scala di rendering: 100% (disegno diretto)");
        }
        (Plan::Direct, None) => {}
        (
            Plan::Scaled {
                size,
                scale_factor,
                logical,
            },
            current,
        ) => {
            let image = new_frame_image(&mut world.resource_mut::<Assets<Image>>(), size);
            let (view, frame) = match current {
                Some(s) => {
                    // Cambio di misura (finestra, scala o mappa): nuova immagine, stessa catena.
                    if let Some(mut sprite) = world.get_mut::<Sprite>(s.frame) {
                        sprite.image = image.clone();
                        sprite.custom_size = Some(logical);
                    }
                    (s.view, s.frame)
                }
                None => {
                    // bevy_egui crea il contesto nel primo PreUpdate: si riprova al prossimo giro.
                    if world.get::<PrimaryEguiContext>(cam3d).is_none() {
                        return;
                    }
                    let view = world
                        .spawn((
                            Name::new("Camera finale (scala di rendering)"),
                            Camera2d,
                            Camera {
                                order: 1,
                                ..default()
                            },
                            // Solo una copia dell'immagine: niente tonemapping né dithering in più.
                            Tonemapping::None,
                            DebandDither::Disabled,
                            Msaa::Off,
                            RenderLayers::layer(FRAME_LAYER),
                            ScaledView,
                        ))
                        .id();
                    let frame = world
                        .spawn((
                            Name::new("Immagine della scena"),
                            Sprite {
                                image: image.clone(),
                                custom_size: Some(logical),
                                ..default()
                            },
                            RenderLayers::layer(FRAME_LAYER),
                        ))
                        .id();
                    if !move_egui(world, cam3d, view) {
                        world.despawn(view);
                        world.despawn(frame);
                        return;
                    }
                    (view, frame)
                }
            };
            world.entity_mut(cam3d).insert((
                RenderTarget::Image(ImageRenderTarget {
                    handle: image,
                    scale_factor,
                }),
                // Con la camera su un'immagine Bevy non sa da dove scegliere il dettaglio delle
                // ombre (come senza finestra): è la camera 3D.
                ShadowLodOrigin,
            ));
            info!(
                "[GRAFICA] scala di rendering: scena a {}x{} pixel",
                size.x, size.y
            );
            world.insert_resource(Scaled { view, frame, size });
        }
    }
}

/// Sistema del mondo di disegno: la camera che ha perso il contesto egui conserva il legame alla
/// vista di egui del fotogramma prima (bevy_egui lo toglie solo alle camere spente), e quella vista
/// non esiste più: senza toglierlo bevy_egui va in panic preparando le trasformazioni.
#[allow(clippy::type_complexity)]
fn drop_stale_egui_views(
    mut commands: Commands,
    cameras: Extract<Query<&RenderEntity, (With<Camera>, Without<EguiContext>)>>,
) {
    for render in &cameras {
        if let Ok(mut e) = commands.get_entity(render.id()) {
            e.remove::<EguiCameraView>();
        }
    }
}

pub fn plugin(app: &mut App) {
    // Senza finestra la camera disegna già su un'immagine a piena risoluzione (foto del pilota).
    if crate::devtools::headless() {
        return;
    }
    app.add_systems(First, apply_render_scale);
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render.add_systems(ExtractSchedule, drop_stale_egui_views);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_size_follows_the_percentage() {
        let full = UVec2::new(1920, 1080);
        assert_eq!(scaled_size(full, 100), full);
        assert_eq!(scaled_size(full, 75), UVec2::new(1440, 810));
        assert_eq!(scaled_size(full, 50), UVec2::new(960, 540));
        assert_eq!(scaled_size(UVec2::new(1, 1), 50), UVec2::new(1, 1));
    }
}
