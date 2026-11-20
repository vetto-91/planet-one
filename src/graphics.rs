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
use bevy::window::{PresentMode, PrimaryWindow};
use bevy_egui::egui;
use std::time::{Duration, Instant};

/// Qualità delle ombre del sole.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shadows {
    Off,
    /// Per le schede integrate (ADR 0057): una cascata piccola e corta.
    Minimal,
    Low,
    Medium,
    High,
}

impl Shadows {
    pub const ALL: [Shadows; 5] = [
        Shadows::High,
        Shadows::Medium,
        Shadows::Low,
        Shadows::Minimal,
        Shadows::Off,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Shadows::Off => "Spente",
            Shadows::Minimal => "Minime",
            Shadows::Low => "Basse",
            Shadows::Medium => "Medie",
            Shadows::High => "Alte",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Shadows::Off => "off",
            Shadows::Minimal => "minimal",
            Shadows::Low => "low",
            Shadows::Medium => "medium",
            Shadows::High => "high",
        }
    }

    /// (cascate, lato della mappa in pixel, distanza massima in metri).
    pub fn params(self) -> (usize, usize, f32) {
        match self {
            Shadows::Off => (1, 512, 10.0),
            Shadows::Minimal => (1, 1024, 40.0),
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
    /// Luci negli edifici (solo in quello in cui si trova il giocatore).
    pub interior_lights: bool,
    pub ssao: bool,
    pub bloom: bool,
    pub antialias: bool,
    /// Percentuale di particelle (pioggia, neve, foglie): 0, 50 o 100.
    pub particles: u8,
    /// Ferma le animazioni dei personaggi lontani.
    pub far_anim_cull: bool,
    /// Onde, corrente e schiuma sull'acqua.
    pub waves: bool,
    /// Percentuale di vegetazione: 100, 70 o 40.
    pub vegetation: u8,
    /// Ombre delle luci piccole: stufa, lume, luci interne e lanterne delle guardie (ADR 0057).
    pub small_shadows: bool,
    /// Distanza di erba, fiori e cespugli in metri (0 = normale, 42 m come sempre).
    pub grass_distance: u16,
    /// Texture dei materiali a metà risoluzione (vale al riavvio).
    pub reduced_textures: bool,
    /// Scala di rendering della scena in percentuale: 100, 75 o 50 (ADR 0058).
    pub render_scale: u8,
}

impl GfxOptions {
    pub const HIGH: Self = Self {
        shadows: Shadows::High,
        lamp_shadows: true,
        interior_lights: true,
        ssao: true,
        bloom: true,
        antialias: true,
        particles: 100,
        far_anim_cull: true,
        waves: true,
        vegetation: 100,
        small_shadows: true,
        grass_distance: 0,
        reduced_textures: false,
        render_scale: 100,
    };
    pub const MEDIUM: Self = Self {
        shadows: Shadows::Medium,
        lamp_shadows: false,
        interior_lights: true,
        ssao: true,
        bloom: true,
        antialias: true,
        particles: 50,
        far_anim_cull: true,
        waves: true,
        vegetation: 70,
        small_shadows: true,
        grass_distance: 0,
        reduced_textures: false,
        render_scale: 100,
    };
    pub const LOW: Self = Self {
        shadows: Shadows::Low,
        lamp_shadows: false,
        interior_lights: false,
        ssao: false,
        bloom: false,
        antialias: false,
        particles: 50,
        far_anim_cull: true,
        waves: false,
        vegetation: 40,
        small_shadows: true,
        grass_distance: 0,
        reduced_textures: false,
        render_scale: 100,
    };

    /// Schede integrate e senza GPU (ADR 0057): grafica leggera, stesso gioco.
    pub const INTEGRATED: Self = Self {
        shadows: Shadows::Minimal,
        lamp_shadows: false,
        interior_lights: false,
        ssao: false,
        bloom: false,
        antialias: false,
        particles: 50,
        far_anim_cull: true,
        waves: false,
        vegetation: 40,
        small_shadows: false,
        grass_distance: 30,
        reduced_textures: true,
        render_scale: 75,
    };

    /// Distanze proposte per erba, fiori e cespugli (0 = normale).
    pub const GRASS_DISTANCES: [u16; 2] = [0, 30];

    /// Scale di rendering proposte, in percentuale.
    pub const RENDER_SCALES: [u8; 3] = [100, 75, 50];

    /// Quota delle particelle attive (0..1).
    pub fn particle_scale(&self) -> f32 {
        self.particles as f32 / 100.0
    }

    /// Righe da scrivere in `settings.txt`.
    pub fn to_lines(self) -> String {
        format!(
            "gfx_shadows={}\ngfx_lamp_shadows={}\ngfx_interior_lights={}\ngfx_ssao={}\ngfx_bloom={}\ngfx_antialias={}\ngfx_particles={}\ngfx_far_anim_cull={}\ngfx_waves={}\ngfx_vegetation={}\ngfx_small_shadows={}\ngfx_grass_distance={}\ngfx_reduced_textures={}\ngfx_render_scale={}\n",
            self.shadows.key(),
            u8::from(self.lamp_shadows),
            u8::from(self.interior_lights),
            u8::from(self.ssao),
            u8::from(self.bloom),
            u8::from(self.antialias),
            self.particles,
            u8::from(self.far_anim_cull),
            u8::from(self.waves),
            self.vegetation,
            u8::from(self.small_shadows),
            self.grass_distance,
            u8::from(self.reduced_textures),
            self.render_scale,
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
            "gfx_interior_lights" => self.interior_lights = on,
            "gfx_ssao" => self.ssao = on,
            "gfx_bloom" => self.bloom = on,
            "gfx_antialias" => self.antialias = on,
            "gfx_vegetation" => {
                if let Ok(p) = value.parse::<u8>()
                    && [40, 70, 100].contains(&p)
                {
                    self.vegetation = p;
                }
            }
            "gfx_particles" => {
                if let Ok(p) = value.parse::<u8>()
                    && [0, 50, 100].contains(&p)
                {
                    self.particles = p;
                }
            }
            "gfx_far_anim_cull" => self.far_anim_cull = on,
            "gfx_waves" => self.waves = on,
            "gfx_small_shadows" => self.small_shadows = on,
            "gfx_grass_distance" => {
                if let Ok(d) = value.parse::<u16>()
                    && Self::GRASS_DISTANCES.contains(&d)
                {
                    self.grass_distance = d;
                }
            }
            "gfx_reduced_textures" => self.reduced_textures = on,
            "gfx_render_scale" => {
                if let Ok(p) = value.parse::<u8>()
                    && Self::RENDER_SCALES.contains(&p)
                {
                    self.render_scale = p;
                }
            }
            _ => {}
        }
    }
}

impl Default for GfxOptions {
    fn default() -> Self {
        Self::HIGH
    }
}

/// Limiti di FPS proposti (0 = illimitati).
pub const FPS_LIMITS: [u16; 8] = [0, 30, 60, 90, 120, 144, 165, 240];

/// Frequenza di aggiornamento: fuori dai preset di qualità.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct FpsOptions {
    pub vsync: bool,
    /// Limite in FPS (0 = illimitati).
    pub limit: u16,
}

impl FpsOptions {
    pub fn to_lines(self) -> String {
        format!(
            "fps_vsync={}\nfps_limit={}\n",
            u8::from(self.vsync),
            self.limit
        )
    }

    pub fn read_line(&mut self, line: &str) {
        let Some((key, value)) = line.trim().split_once('=') else {
            return;
        };
        match key.trim() {
            "fps_vsync" => self.vsync = value.trim() != "0",
            "fps_limit" => {
                if let Ok(v) = value.trim().parse::<u16>()
                    && FPS_LIMITS.contains(&v)
                {
                    self.limit = v;
                }
            }
            _ => {}
        }
    }
}

/// Durata di un fotogramma al limite dato (`None` = illimitati).
pub fn frame_period(limit: u16) -> Option<Duration> {
    (limit > 0).then(|| Duration::from_secs_f64(1.0 / limit as f64))
}

/// Prossimo istante di fine fotogramma: a passi regolari, così la media resta esatta; dopo un
/// fotogramma molto lento si riparte da adesso, senza inseguire il ritardo.
pub fn next_deadline(previous: Instant, now: Instant, period: Duration) -> Instant {
    let next = previous + period;
    if now > next + period { now } else { next }
}

/// Sistema: applica il VSync quando cambia.
pub fn apply_present_mode(
    settings: Res<AudioSettings>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mode = if settings.fps.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
    if let Ok(mut w) = windows.single_mut()
        && w.present_mode != mode
    {
        w.present_mode = mode;
        info!("[GRAFICA] VSync: {}", settings.fps.vsync);
    }
}

/// Sistema (fine del fotogramma): attende quanto serve a rispettare il limite di FPS.
pub fn limit_frame_rate(settings: Res<AudioSettings>, mut deadline: Local<Option<Instant>>) {
    let Some(period) = frame_period(settings.fps.limit) else {
        *deadline = None;
        return;
    };
    let now = Instant::now();
    let target = next_deadline(deadline.unwrap_or(now), now, period);
    // Sleep per la parte lunga, attesa attiva per gli ultimi 2 ms: il limite resta preciso.
    let margin = Duration::from_millis(2);
    let wait = target.saturating_duration_since(now);
    if wait > margin {
        std::thread::sleep(wait - margin);
    }
    while Instant::now() < target {
        std::hint::spin_loop();
    }
    *deadline = Some(target);
}

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

/// Schede che vogliono la grafica leggera: integrate nella CPU (Intel, AMD) o senza GPU vera
/// (rendering software). `kind` è il nome del tipo di scheda di wgpu (ADR 0057).
pub fn wants_light_preset(kind: &str) -> bool {
    matches!(kind, "IntegratedGpu" | "Cpu")
}

/// Sistema (prima dello startup, prima di caricare le texture): su una scheda integrata, se la
/// grafica non è mai stata scelta, propone il preset «Integrata». Una scelta salvata non si tocca.
pub fn pick_preset_for_adapter(
    info: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    mut settings: ResMut<AudioSettings>,
) {
    if let Some(info) = info {
        let kind = format!("{:?}", info.device_type);
        let choice = if settings.gfx_chosen {
            "grafica già scelta"
        } else if wants_light_preset(&kind) {
            settings.gfx = GfxOptions::INTEGRATED;
            "preset Integrata"
        } else {
            "preset Alta"
        };
        info!("[GRAFICA] scheda: {} ({kind}) → {choice}", info.name);
    }
    crate::pbr::set_reduced_textures(settings.gfx.reduced_textures);
}

/// Scheda Video: solo aspetto visivo.
pub fn graphics_ui(ui: &mut egui::Ui, audio: &mut AudioSettings) {
    ui.label("Solo aspetto visivo: non cambiano il gioco.");
    ui.separator();
    let mut gfx = audio.gfx;
    ui.horizontal(|ui| {
        ui.label("Preset:");
        for (name, preset) in [
            ("Alta", GfxOptions::HIGH),
            ("Media", GfxOptions::MEDIUM),
            ("Bassa", GfxOptions::LOW),
            ("Integrata", GfxOptions::INTEGRATED),
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
    ui.checkbox(&mut gfx.small_shadows, "Ombre di stufa, lumi e lanterne");
    ui.checkbox(&mut gfx.interior_lights, "Luci negli edifici");
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
    ui.checkbox(&mut gfx.far_anim_cull, "Ferma le animazioni lontane");
    ui.checkbox(&mut gfx.waves, "Onde e schiuma sull'acqua");
    ui.horizontal(|ui| {
        ui.label("Densità vegetazione:");
        for p in [100u8, 70, 40] {
            if ui
                .selectable_label(gfx.vegetation == p, format!("{p}%"))
                .clicked()
            {
                gfx.vegetation = p;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Erba, fiori e cespugli:");
        for d in GfxOptions::GRASS_DISTANCES {
            let label = if d == 0 {
                "Normali".to_string()
            } else {
                format!("Vicini ({d} m)")
            };
            if ui
                .selectable_label(gfx.grass_distance == d, label)
                .clicked()
            {
                gfx.grass_distance = d;
            }
        }
    });
    ui.checkbox(
        &mut gfx.reduced_textures,
        "Texture ridotte (al prossimo avvio)",
    );
    ui.horizontal(|ui| {
        ui.label("Risoluzione della scena:");
        for p in GfxOptions::RENDER_SCALES {
            if ui
                .selectable_label(gfx.render_scale == p, format!("{p}%"))
                .clicked()
            {
                gfx.render_scale = p;
            }
        }
    })
    .response
    .on_hover_text("Sotto il 100% la scena si disegna più piccola e si ingrandisce: interfaccia e mappa restano nitide.");
    ui.separator();
    let mut fps = audio.fps;
    ui.checkbox(&mut fps.vsync, "VSync (FPS legati al monitor)");
    ui.horizontal_wrapped(|ui| {
        ui.label("Limite FPS:");
        for limit in FPS_LIMITS {
            let label = if limit == 0 {
                "Illimitati".to_string()
            } else {
                limit.to_string()
            };
            if ui.selectable_label(fps.limit == limit, label).clicked() {
                fps.limit = limit;
            }
        }
    });
    if fps != audio.fps {
        audio.fps = fps;
        audio.dirty = true;
    }
    if gfx != audio.gfx {
        audio.gfx = gfx;
        audio.dirty = true;
    }
}

/// Una luce puntiforme che può fare ombra: chi la gestisce (lampioni, luci degli edifici, stufa,
/// lanterne) dice solo se la *vorrebbe*; [`budget_point_shadows`] la accende davvero solo alle più
/// vicine. Ogni ombra di una luce puntiforme è un cubo di sei facce: con 50-60 luci che la
/// volevano tutte, di sera la VRAM saliva di circa 1,4 GB e gli FPS crollavano (segnalazione
/// 2026-11-19_22-17-36, copione `zz-lunga-sessione`).
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct ShadowWanted(pub bool);

impl ShadowWanted {
    /// Scrive solo se cambia, per non segnare la componente come cambiata a ogni fotogramma.
    pub fn set(&mut self, on: bool) {
        if self.0 != on {
            self.0 = on;
        }
    }
}

/// Quante luci puntiformi fanno ombra insieme, secondo la qualità delle ombre.
pub fn point_shadow_budget(shadows: Shadows) -> usize {
    match shadows {
        Shadows::Off => 0,
        Shadows::Minimal => 1,
        Shadows::Low => 2,
        Shadows::Medium => 3,
        Shadows::High => 4,
    }
}

/// Oltre questa distanza dalla camera una luce puntiforme non fa ombra.
const POINT_SHADOW_RANGE: f32 = 30.0;

/// Sceglie le luci che fanno ombra: tra quelle che la vogliono, accese e visibili entro
/// [`POINT_SHADOW_RANGE`], solo le [`point_shadow_budget`] più vicine alla camera. Ricalcola
/// quattro volte al secondo, così le ombre non si accendono e spengono a ogni passo.
pub fn budget_point_shadows(
    time: Res<Time>,
    settings: Res<AudioSettings>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut lights: Query<(
        Entity,
        &mut PointLight,
        &ShadowWanted,
        &GlobalTransform,
        &InheritedVisibility,
    )>,
    mut timer: Local<f32>,
    mut picks: Local<Vec<(f32, Entity)>>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 && !settings.is_changed() {
        return;
    }
    *timer = 0.25;
    let Some(cam) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    picks.clear();
    picks.extend(lights.iter().filter_map(|(e, l, want, t, vis)| {
        let d = t.translation().distance(cam);
        (want.0 && vis.get() && l.intensity > 1.0 && d <= POINT_SHADOW_RANGE).then_some((d, e))
    }));
    picks.sort_by(|a, b| a.0.total_cmp(&b.0));
    picks.truncate(point_shadow_budget(settings.gfx.shadows));
    for (e, mut l, ..) in &mut lights {
        let on = picks.iter().any(|(_, p)| *p == e);
        if l.shadow_maps_enabled != on {
            l.shadow_maps_enabled = on;
        }
    }
}

/// Aggira un difetto di Bevy 0.20.1: `extract_lights` considera solo le luci cambiate in questo
/// fotogramma e toglie le liste di visibilità delle ombre a tutte le altre, mentre le viste delle
/// facce esistono ancora e vanno in panic («Failed to get point light visible entities»).
/// Segnare ogni luce con ombre come cambiata a ogni fotogramma evita il panic.
fn keep_shadow_lights_extracted(mut lights: Query<&mut PointLight>) {
    for mut light in &mut lights {
        if light.shadow_maps_enabled {
            light.set_changed();
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(PreStartup, pick_preset_for_adapter)
        .add_systems(
            Update,
            (
                apply_graphics,
                apply_present_mode,
                budget_point_shadows,
                keep_shadow_lights_extracted.after(budget_point_shadows),
            ),
        )
        .add_systems(Last, limit_frame_rate);
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
            interior_lights: false,
            ssao: false,
            bloom: true,
            antialias: false,
            particles: 0,
            far_anim_cull: false,
            waves: false,
            vegetation: 40,
            small_shadows: false,
            grass_distance: 30,
            reduced_textures: true,
            render_scale: 50,
        };
        let mut read = GfxOptions::HIGH;
        for line in original.to_lines().lines() {
            read.read_line(line);
        }
        assert_eq!(read, original);
    }

    #[test]
    fn integrated_preset_survives_the_settings_file() {
        let mut read = GfxOptions::HIGH;
        for line in GfxOptions::INTEGRATED.to_lines().lines() {
            read.read_line(line);
        }
        assert_eq!(read, GfxOptions::INTEGRATED);
    }

    #[test]
    fn old_settings_files_keep_todays_graphics() {
        // Un file scritto prima dell'ADR 0057 non ha le chiavi nuove: restano i valori di oggi.
        for preset in [GfxOptions::HIGH, GfxOptions::MEDIUM, GfxOptions::LOW] {
            assert!(preset.small_shadows);
            assert_eq!(preset.grass_distance, 0);
            assert!(!preset.reduced_textures);
            assert_eq!(preset.render_scale, 100);
        }
        assert_eq!(GfxOptions::default(), GfxOptions::HIGH);
    }

    #[test]
    fn only_integrated_and_software_adapters_get_the_light_preset() {
        assert!(wants_light_preset("IntegratedGpu"));
        assert!(wants_light_preset("Cpu"));
        for kind in ["DiscreteGpu", "VirtualGpu", "Other"] {
            assert!(!wants_light_preset(kind), "{kind}");
        }
    }

    #[test]
    fn integrated_preset_is_the_lightest() {
        let g = GfxOptions::INTEGRATED;
        let low = GfxOptions::LOW;
        assert!(g.shadows.params().0 < low.shadows.params().0);
        assert!(g.shadows.params().2 < low.shadows.params().2);
        assert!(!g.ssao && !g.bloom && !g.antialias && !g.small_shadows);
        assert!(g.vegetation <= low.vegetation && g.particles <= low.particles);
    }

    #[test]
    fn invalid_settings_lines_are_ignored() {
        let mut g = GfxOptions::HIGH;
        for line in [
            "gfx_shadows=boh",
            "gfx_particles=33",
            "gfx_grass_distance=7",
            "gfx_render_scale=60",
            "altro=1",
            "senza uguale",
        ] {
            g.read_line(line);
        }
        assert_eq!(g, GfxOptions::HIGH);
    }

    #[test]
    fn deadlines_keep_a_regular_pace_and_skip_long_stalls() {
        assert_eq!(frame_period(0), None);
        let period = frame_period(100).unwrap();
        let start = Instant::now();
        let early = start + Duration::from_millis(4);
        assert_eq!(next_deadline(start, early, period), start + period);
        let stalled = start + Duration::from_millis(50);
        assert_eq!(next_deadline(start, stalled, period), stalled);
    }

    #[test]
    fn fps_options_survive_the_settings_file() {
        let original = FpsOptions {
            vsync: true,
            limit: 144,
        };
        let mut read = FpsOptions::default();
        for line in original.to_lines().lines() {
            read.read_line(line);
        }
        assert_eq!(read, original);
        read.read_line("fps_limit=77");
        assert_eq!(read.limit, 144);
    }
}
