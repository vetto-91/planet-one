//! Misure delle prestazioni per la riga `[PERF]` del log (una al minuto, anche giocando).
//!
//! Il 27-11-2026 su Windows gli FPS sono scesi da 122 a 18 in un'ora e mezza mentre entità, mesh,
//! materiali e luci restavano fermi; il log diceva che il contatore dei tick del mondo del render
//! aveva superato i 3,26 miliardi. Qui si misura ciò che allora mancava:
//! - fotogrammi, tempo medio e massimo del fotogramma nell'ultimo minuto;
//! - quanti tick avanzano per fotogramma il mondo principale e quello del render (un tick per ogni
//!   sistema eseguito: se crescono, qualcosa gira sempre più volte);
//! - quante viste disegna il render (camera e ombre).

use bevy::prelude::*;
use bevy::render::view::ExtractedView;
use bevy::render::{Render, RenderApp, RenderSystems};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Valori scritti dal mondo del render e letti dal principale.
#[derive(Resource, Clone, Default)]
pub struct RenderProbe(Arc<RenderCounters>);

#[derive(Default)]
pub struct RenderCounters {
    /// Tick del mondo del render alla fine dell'ultimo fotogramma.
    tick: AtomicU32,
    /// Fotogrammi disegnati dal render.
    frames: AtomicU64,
    /// Viste estratte nell'ultimo fotogramma (camere e viste delle ombre).
    views: AtomicU32,
    /// Il massimo di viste in un fotogramma dall'ultima lettura.
    views_max: AtomicU32,
}

/// Fotogrammi del mondo principale dall'ultima riga `[PERF]`.
#[derive(Resource, Default)]
pub struct FrameWindow {
    frames: u32,
    sum: f64,
    max: f64,
}

/// Valori del minuto precedente, per le differenze.
#[derive(Default)]
pub struct PerfMarks {
    main_tick: u32,
    render_tick: u32,
    render_frames: u64,
}

/// Riassunto di un minuto, già pronto per la riga `[PERF]`.
pub struct Sample {
    pub frames: u32,
    pub ms_avg: f64,
    pub ms_max: f64,
    pub main_ticks_per_frame: f64,
    pub render_ticks_per_frame: f64,
    pub views: u32,
    pub views_max: u32,
}

fn count_frames(time: Res<Time<Real>>, mut w: ResMut<FrameWindow>) {
    let ms = time.delta_secs_f64() * 1000.0;
    w.frames += 1;
    w.sum += ms;
    w.max = w.max.max(ms);
}

/// Nel mondo del render, a fine fotogramma: tick e viste.
fn probe_render(
    probe: Res<RenderProbe>,
    tick: bevy::ecs::system::SystemChangeTick,
    views: Query<(), With<ExtractedView>>,
) {
    let c = &probe.0;
    c.tick.store(tick.this_run().get(), Ordering::Relaxed);
    c.frames.fetch_add(1, Ordering::Relaxed);
    let n = views.iter().count() as u32;
    c.views.store(n, Ordering::Relaxed);
    c.views_max.fetch_max(n, Ordering::Relaxed);
}

/// Chiude il minuto: medie dei fotogrammi e tick per fotogramma dei due mondi.
pub fn take_sample(world: &mut World, marks: &mut PerfMarks) -> Sample {
    let main_tick = world.change_tick().get();
    let window = std::mem::take(&mut *world.resource_mut::<FrameWindow>());
    let probe = world.resource::<RenderProbe>().0.clone();
    let render_tick = probe.tick.load(Ordering::Relaxed);
    let render_frames = probe.frames.load(Ordering::Relaxed);
    let views_max = probe.views_max.swap(0, Ordering::Relaxed);
    let frames = window.frames.max(1);
    let rframes = (render_frames - marks.render_frames).max(1);
    let per = |now: u32, before: u32, n: f64| {
        if before == 0 {
            0.0
        } else {
            now.wrapping_sub(before) as f64 / n
        }
    };
    let sample = Sample {
        frames: window.frames,
        ms_avg: window.sum / frames as f64,
        ms_max: window.max,
        main_ticks_per_frame: per(main_tick, marks.main_tick, frames as f64),
        render_ticks_per_frame: per(render_tick, marks.render_tick, rframes as f64),
        views: probe.views.load(Ordering::Relaxed),
        views_max,
    };
    *marks = PerfMarks {
        main_tick,
        render_tick,
        render_frames,
    };
    sample
}

pub fn plugin(app: &mut App) {
    let probe = RenderProbe::default();
    app.insert_resource(probe.clone())
        .init_resource::<FrameWindow>()
        .add_systems(Last, count_frames);
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .insert_resource(probe)
            .add_systems(Render, probe_render.in_set(RenderSystems::Cleanup));
    }
}
