//! Comando `diagnosi` della shell: spegne per qualche secondo una parte del gioco alla volta e
//! scrive nel log quanto costa, in righe `[DIAGNOSI]`. Serve quando gli FPS calano su un PC che non
//! è quello di sviluppo: dopo, un F12 porta con sé queste righe.
//!
//! Le opzioni grafiche si cambiano solo in memoria (`AudioSettings` senza `dirty`), quindi non
//! finiscono in `settings.txt`; tutto torna com'era alla fine di ogni prova.

use crate::AppState;
use crate::audio::AudioSettings;
use bevy::prelude::*;

/// Secondi di assestamento dopo ogni cambio (shader, mesh che spariscono).
const SETTLE: f32 = 1.0;
/// Secondi di misura per ogni prova.
const MEASURE: f32 = 3.0;

/// Le parti del gioco che si spengono una alla volta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Shadows,
    PointLights,
    InteriorLights,
    Rain,
    Vegetation,
    Animations,
    Water,
    Ssao,
    Bloom,
    Antialias,
    HalfScale,
}

impl Part {
    pub const ALL: [Part; 11] = [
        Part::Shadows,
        Part::PointLights,
        Part::InteriorLights,
        Part::Rain,
        Part::Vegetation,
        Part::Animations,
        Part::Water,
        Part::Ssao,
        Part::Bloom,
        Part::Antialias,
        Part::HalfScale,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Part::Shadows => "ombre del sole",
            Part::PointLights => "luci puntiformi (lampioni, torce, lanterne)",
            Part::InteriorLights => "luci negli edifici",
            Part::Rain => "pioggia, neve e particelle",
            Part::Vegetation => "vegetazione (erba, fiori, cespugli, alberi a riquadri)",
            Part::Animations => "animazioni dei personaggi",
            Part::Water => "onde e schiuma",
            Part::Ssao => "SSAO",
            Part::Bloom => "bloom",
            Part::Antialias => "antialiasing",
            Part::HalfScale => {
                "scala di rendering al 50% (se aiuta molto, il limite è la scheda video)"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// In attesa del gioco (la shell va chiusa perché il mondo giri).
    Waiting,
    Baseline,
    Settle(usize),
    Measure(usize),
    Restore(usize),
}

/// Ciò che è stato spento, per rimetterlo com'era.
#[derive(Default)]
struct Saved {
    gfx: Option<crate::graphics::GfxOptions>,
    hidden: Vec<(Entity, Visibility)>,
    paused: Vec<Entity>,
}

#[derive(Resource, Default)]
pub struct Diagnosis {
    phase: Option<Phase>,
    timer: f32,
    sum: f64,
    frames: u32,
    baseline: f64,
    saved: Saved,
}

impl Diagnosis {
    /// Avvia la diagnosi: parte appena si torna a giocare.
    pub fn start(&mut self) {
        *self = Diagnosis {
            phase: Some(Phase::Waiting),
            ..Default::default()
        };
    }
}

fn ms_to_fps(ms: f64) -> f64 {
    if ms > 0.0 { 1000.0 / ms } else { 0.0 }
}

/// Riga del resoconto per una prova: guadagno in percentuale del tempo del fotogramma.
pub fn report_line(part: Part, base_ms: f64, ms: f64) -> String {
    let gain = if base_ms > 0.0 {
        (base_ms - ms) / base_ms * 100.0
    } else {
        0.0
    };
    format!(
        "[DIAGNOSI] senza {}: {ms:.1} ms ({:.0} FPS), {gain:+.0}% di tempo risparmiato",
        part.label(),
        ms_to_fps(ms)
    )
}

fn hide<F: bevy::ecs::query::QueryFilter>(world: &mut World, saved: &mut Saved) {
    let mut q = world.query_filtered::<(Entity, &mut Visibility), F>();
    for (e, mut v) in q.iter_mut(world) {
        saved.hidden.push((e, *v));
        *v = Visibility::Hidden;
    }
}

fn apply(world: &mut World, part: Part, saved: &mut Saved) {
    saved.gfx = Some(world.resource::<AudioSettings>().gfx);
    match part {
        Part::PointLights => return hide::<With<PointLight>>(world, saved),
        Part::Vegetation => return hide::<With<crate::nature::NatureChunk>>(world, saved),
        Part::Animations => {
            let mut q = world.query::<(Entity, &mut AnimationPlayer)>();
            for (e, mut p) in q.iter_mut(world) {
                if !p.all_paused() {
                    p.pause_all();
                    saved.paused.push(e);
                }
            }
            return;
        }
        _ => {}
    }
    let mut settings = world.resource_mut::<AudioSettings>();
    let gfx = &mut settings.gfx;
    match part {
        Part::Shadows => gfx.shadows = crate::graphics::Shadows::Off,
        Part::InteriorLights => gfx.interior_lights = false,
        Part::Water => gfx.waves = false,
        Part::Ssao => gfx.ssao = false,
        Part::Bloom => gfx.bloom = false,
        Part::Antialias => gfx.antialias = false,
        Part::HalfScale => gfx.render_scale = 50,
        // Le gocce le muove `weather` a ogni fotogramma: si spengono dalla quota di particelle.
        Part::Rain => gfx.particles = 0,
        Part::PointLights | Part::Vegetation | Part::Animations => {}
    }
}

fn restore(world: &mut World, saved: &mut Saved) {
    if let Some(gfx) = saved.gfx.take() {
        world.resource_mut::<AudioSettings>().gfx = gfx;
    }
    for (e, v) in saved.hidden.drain(..) {
        if let Ok(mut ent) = world.get_entity_mut(e)
            && let Some(mut vis) = ent.get_mut::<Visibility>()
        {
            *vis = v;
        }
    }
    for e in saved.paused.drain(..) {
        if let Ok(mut ent) = world.get_entity_mut(e)
            && let Some(mut p) = ent.get_mut::<AnimationPlayer>()
        {
            p.resume_all();
        }
    }
}

/// Sistema esclusivo: porta avanti la diagnosi un fotogramma alla volta.
pub fn run_diagnosis(world: &mut World) {
    let Some(phase) = world.resource::<Diagnosis>().phase else {
        return;
    };
    let playing = *world.resource::<State<AppState>>().get() == AppState::Playing;
    let dt = world.resource::<Time<Real>>().delta_secs();
    let mut d = std::mem::take(&mut *world.resource_mut::<Diagnosis>());
    d.timer += dt;
    let measuring = matches!(phase, Phase::Baseline | Phase::Measure(_));
    if measuring && d.timer > 0.0 {
        d.sum += f64::from(dt) * 1000.0;
        d.frames += 1;
    }
    let mean = |d: &Diagnosis| d.sum / f64::from(d.frames.max(1));
    let next = match phase {
        Phase::Waiting if playing => {
            info!(
                "[DIAGNOSI] inizio: {} prove da {MEASURE:.0} s; non muovere il personaggio",
                Part::ALL.len()
            );
            Some(Phase::Baseline)
        }
        Phase::Waiting => None,
        Phase::Baseline if d.timer >= MEASURE => {
            d.baseline = mean(&d);
            info!(
                "[DIAGNOSI] tutto acceso: {:.1} ms ({:.0} FPS)",
                d.baseline,
                ms_to_fps(d.baseline)
            );
            apply(world, Part::ALL[0], &mut d.saved);
            Some(Phase::Settle(0))
        }
        Phase::Settle(i) if d.timer >= SETTLE => Some(Phase::Measure(i)),
        Phase::Measure(i) if d.timer >= MEASURE => {
            info!("{}", report_line(Part::ALL[i], d.baseline, mean(&d)));
            restore(world, &mut d.saved);
            Some(Phase::Restore(i))
        }
        Phase::Restore(i) if d.timer >= SETTLE => {
            if let Some(&part) = Part::ALL.get(i + 1) {
                apply(world, part, &mut d.saved);
                Some(Phase::Settle(i + 1))
            } else {
                info!("[DIAGNOSI] fine: tutto è tornato com'era. F12 per mandare il resoconto");
                d.phase = None;
                *world.resource_mut::<Diagnosis>() = d;
                return;
            }
        }
        _ => None,
    };
    if let Some(p) = next {
        d.phase = Some(p);
        d.timer = -0.0;
        d.sum = 0.0;
        d.frames = 0;
    }
    *world.resource_mut::<Diagnosis>() = d;
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Diagnosis>()
        .add_systems(Last, run_diagnosis);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_says_how_much_time_a_part_costs() {
        let line = report_line(Part::Rain, 40.0, 30.0);
        assert!(line.contains("pioggia"), "{line}");
        assert!(line.contains("+25%"), "{line}");
        assert!(line.contains("33 FPS"), "{line}");
    }
}
