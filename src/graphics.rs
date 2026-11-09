//! Impostazioni grafiche (menu Esc → "Impostazioni grafiche"): solo aspetto visivo, nessun effetto sul gioco.
//!
//! Le opzioni stanno in `AudioSettings.gfx` (salvate in `settings.txt`, chiavi `gfx_*`) e si applicano a
//! camera e sole quando cambiano o dopo un reset della scena.

use crate::audio::AudioSettings;
use crate::daynight::Sun;
use bevy::anti_alias::smaa::Smaa;
use bevy::light::{CascadeShadowConfig, CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy_egui::egui;

/// Qualità delle ombre del sole.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shadows {
    Off,
    Low,
    Medium,
    High,
}

impl Shadows {
    pub const ALL: [Shadows; 4] = [Shadows::High, Shadows::Medium, Shadows::Low, Shadows::Off];

    pub fn name(self) -> &'static str {
        match self {
            Shadows::Off => "Spente",
            Shadows::Low => "Basse",
            Shadows::Medium => "Medie",
            Shadows::High => "Alte",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Shadows::Off => "off",
            Shadows::Low => "low",
            Shadows::Medium => "medium",
            Shadows::High => "high",
        }
    }

    /// (cascate, lato della mappa in pixel, distanza massima in metri).
    pub fn params(self) -> (usize, usize, f32) {
        match self {
            Shadows::Off => (1, 512, 10.0),
            Shadows::Low => (2, 1024, 60.0),
            Shadows::Medium => (3, 2048, 100.0),
            Shadows::High => (4, 4096, 180.0),
        }
    }
}

/// Opzioni grafiche correnti.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GfxOptions {
    pub shadows: Shadows,
    pub lamp_shadows: bool,
    pub ssao: bool,
    pub bloom: bool,
    pub antialias: bool,
    /// Percentuale di particelle (pioggia, neve, foglie): 0, 50 o 100.
    pub particles: u8,
    /// Ferma le animazioni dei personaggi lontani.
    pub far_anim_cull: bool,
}

impl GfxOptions {
    pub const HIGH: Self = Self {
        shadows: Shadows::High,
        lamp_shadows: true,
        ssao: true,
        bloom: true,
        antialias: true,
        particles: 100,
        far_anim_cull: true,
    };
    pub const MEDIUM: Self = Self {
        shadows: Shadows::Medium,
        lamp_shadows: false,
        ssao: true,
        bloom: true,
        antialias: true,
        particles: 50,
        far_anim_cull: true,
    };
    pub const LOW: Self = Self {
        shadows: Shadows::Low,
        lamp_shadows: false,
        ssao: false,
        bloom: false,
        antialias: false,
        particles: 50,
        far_anim_cull: true,
    };

    /// Quota delle particelle attive (0..1).
    pub fn particle_scale(&self) -> f32 {
        self.particles as f32 / 100.0
    }

    /// Righe da scrivere in `settings.txt`.
    pub fn to_lines(self) -> String {
        format!(
            "gfx_shadows={}\ngfx_lamp_shadows={}\ngfx_ssao={}\ngfx_bloom={}\ngfx_antialias={}\ngfx_particles={}\ngfx_far_anim_cull={}\n",
            self.shadows.key(),
            u8::from(self.lamp_shadows),
            u8::from(self.ssao),
            u8::from(self.bloom),
            u8::from(self.antialias),
            self.particles,
            u8::from(self.far_anim_cull),
        )
    }

    /// Legge una riga di `settings.txt`; ignora ciò che non riconosce.
    pub fn read_line(&mut self, line: &str) {
        let Some((key, value)) = line.trim().split_once('=') else {
            return;
        };
        let value = value.trim();
        let on = value != "0";
        match key.trim() {
            "gfx_shadows" => {
                if let Some(s) = Shadows::ALL.into_iter().find(|s| s.key() == value) {
                    self.shadows = s;
                }
            }
            "gfx_lamp_shadows" => self.lamp_shadows = on,
            "gfx_ssao" => self.ssao = on,
            "gfx_bloom" => self.bloom = on,
            "gfx_antialias" => self.antialias = on,
            "gfx_particles" => {
                if let Ok(p) = value.parse::<u8>()
                    && [0, 50, 100].contains(&p)
                {
                    self.particles = p;
                }
            }
            "gfx_far_anim_cull" => self.far_anim_cull = on,
            _ => {}
        }
    }
}

impl Default for GfxOptions {
    fn default() -> Self {
        Self::HIGH
    }
}

/// La finestra delle impostazioni grafiche è aperta?
#[derive(Resource, Default)]
pub struct GfxPanelOpen(pub bool);

/// Sistema: applica le opzioni quando cambiano, o quando un reset ha ricreato il sole.
#[allow(clippy::type_complexity)]
pub fn apply_graphics(
    mut commands: Commands,
    settings: Res<AudioSettings>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    cameras: Query<Entity, With<Camera3d>>,
    mut suns: Query<(&mut DirectionalLight, &mut CascadeShadowConfig), With<Sun>>,
    fresh: Query<(), Added<Sun>>,
    mut last: Local<Option<GfxOptions>>,
) {
    let gfx = settings.gfx;
    if *last == Some(gfx) && fresh.is_empty() {
        return;
    }
    *last = Some(gfx);
    let (cascades, size, distance) = gfx.shadows.params();
    if shadow_map.size != size {
        shadow_map.size = size;
    }
    for (mut light, mut config) in &mut suns {
        light.shadow_maps_enabled = gfx.shadows != Shadows::Off;
        *config = CascadeShadowConfigBuilder {
            num_cascades: cascades,
            minimum_distance: 0.5,
            first_cascade_far_bound: (distance / 15.0).max(4.0),
            maximum_distance: distance,
            overlap_proportion: 0.2,
        }
        .build();
    }
    for cam in &cameras {
        let mut c = commands.entity(cam);
        if gfx.ssao {
            c.insert(ScreenSpaceAmbientOcclusion::default());
        } else {
            c.remove::<ScreenSpaceAmbientOcclusion>();
        }
        if gfx.bloom {
            c.insert(Bloom::NATURAL);
        } else {
            c.remove::<Bloom>();
        }
        if gfx.antialias {
            c.insert(Smaa::default());
        } else {
            c.remove::<Smaa>();
        }
    }
    info!("[GRAFICA] opzioni applicate: {gfx:?}");
}

/// Finestra del sotto-menu (chiamata dal menu Esc).
pub fn graphics_window(ctx: &egui::Context, audio: &mut AudioSettings, open: &mut bool) {
    let mut keep_open = *open;
    egui::Window::new("Impostazioni grafiche")
        .open(&mut keep_open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(260.0, 0.0))
        .show(ctx, |ui| {
            ui.label("Solo aspetto visivo: non cambiano il gioco.");
            ui.separator();
            let mut gfx = audio.gfx;
            ui.horizontal(|ui| {
                ui.label("Preset:");
                for (name, preset) in [
                    ("Alta", GfxOptions::HIGH),
                    ("Media", GfxOptions::MEDIUM),
                    ("Bassa", GfxOptions::LOW),
                ] {
                    if ui.selectable_label(gfx == preset, name).clicked() {
                        gfx = preset;
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Ombre:");
                for s in Shadows::ALL {
                    if ui.selectable_label(gfx.shadows == s, s.name()).clicked() {
                        gfx.shadows = s;
                    }
                }
            });
            ui.checkbox(&mut gfx.lamp_shadows, "Ombre dei lampioni");
            ui.checkbox(&mut gfx.ssao, "Occlusione ambientale");
            ui.checkbox(&mut gfx.bloom, "Bagliore delle luci");
            ui.checkbox(&mut gfx.antialias, "Antialiasing");
            ui.horizontal(|ui| {
                ui.label("Particelle (pioggia, neve, foglie):");
                for p in [100u8, 50, 0] {
                    let label = if p == 0 {
                        "Spente".to_string()
                    } else {
                        format!("{p}%")
                    };
                    if ui.selectable_label(gfx.particles == p, label).clicked() {
                        gfx.particles = p;
                    }
                }
            });
            if gfx != audio.gfx {
                audio.gfx = gfx;
                audio.dirty = true;
            }
        });
    *open = keep_open;
}

pub fn plugin(app: &mut App) {
    app.init_resource::<GfxPanelOpen>()
        .add_systems(Update, apply_graphics);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_get_lighter() {
        let side = |g: GfxOptions| g.shadows.params().1;
        assert!(side(GfxOptions::HIGH) > side(GfxOptions::MEDIUM));
        assert!(side(GfxOptions::MEDIUM) > side(GfxOptions::LOW));
        const { assert!(GfxOptions::HIGH.ssao && !GfxOptions::LOW.ssao) };
    }

    #[test]
    fn options_survive_the_settings_file() {
        let original = GfxOptions {
            shadows: Shadows::Off,
            lamp_shadows: false,
            ssao: false,
            bloom: true,
            antialias: false,
            particles: 0,
            far_anim_cull: false,
        };
        let mut read = GfxOptions::HIGH;
        for line in original.to_lines().lines() {
            read.read_line(line);
        }
        assert_eq!(read, original);
    }

    #[test]
    fn invalid_settings_lines_are_ignored() {
        let mut g = GfxOptions::HIGH;
        for line in [
            "gfx_shadows=boh",
            "gfx_particles=33",
            "altro=1",
            "senza uguale",
        ] {
            g.read_line(line);
        }
        assert_eq!(g, GfxOptions::HIGH);
    }
}
