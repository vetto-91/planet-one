//! Scene raccontate: una dissolvenza al nero con poche righe, poi il viaggio (ADR 0048).
//!
//! Serve alla partenza dal Prologo: per la via del mare (la barca di Gaspare) o per la via delle
//! profondità (la Porta Sommersa sotto la grotta). Le righe cambiano con ciò che si è fatto.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::region::Region;

/// Secondi per ogni riga, e di dissolvenza all'inizio.
const LINE_SECS: f32 = 3.2;
const FADE_SECS: f32 = 1.2;

/// Scena in corso e avvisi in coda.
#[derive(Resource, Default, Debug)]
pub struct Cutscene {
    pub lines: Vec<String>,
    pub elapsed: f32,
    pub travel: Option<Region>,
    /// Avvisi da mostrare uno dopo l'altro (doni della Sera delle Lanterne…).
    notices: Vec<String>,
    /// Il dialogo era aperto al frame prima (serve a `missions::track_missions`).
    pub was_open: bool,
}

impl Cutscene {
    pub fn active(&self) -> bool {
        !self.lines.is_empty()
    }

    pub fn queue_notice(&mut self, text: String) {
        self.notices.push(text);
    }

    /// La partenza dall'isola: `how` è `mare` o `profonda`.
    pub fn start_travel(&mut self, to: Region, how: &str, karma: &crate::karma::Karma) {
        self.lines = departure_lines(how, karma.unlocks.contains("gaspare-timoniere"));
        self.elapsed = 0.0;
        self.travel = Some(to);
        info!("[VIAGGIO] scena della partenza ({how})");
    }

    fn duration(&self) -> f32 {
        FADE_SECS + self.lines.len() as f32 * LINE_SECS
    }
}

/// Le righe della partenza.
pub fn departure_lines(how: &str, helmsman: bool) -> Vec<String> {
    let mut lines: Vec<&str> = if how == "profonda" {
        vec![
            "Scendi nel cunicolo con la Sfera di luce davanti a te. La lastra di pietra e rame è aperta.",
            "Oltre la soglia, una via lastricata corre sotto il mare. Lungo le pareti, i cavi della Luce Viva.",
            "Cammini per ore seguendo il filo di rame, tra colonne e segni che nessuno legge più.",
            "Poi i gradini salgono, e dall'alto cade una luce grigia di nebbia.",
        ]
    } else {
        let mut v = vec![
            "Prima dell'alba la barca scivola fuori dal porticciolo. Le lanterne di Fontevivo si fanno piccole.",
        ];
        v.push(if helmsman {
            "Gaspare tiene il timone: conosce le correnti dello Stretto, e i tenoni di bronzo non temono gli scogli."
        } else {
            "Gaspare ti lascia la barca e la sua benedizione: le correnti dello Stretto le devi leggere tu."
        });
        v.extend([
            "Il vortice brontola a ovest, i pinnacoli neri tirano a est. La stella ferma resta alta sulla prua.",
            "Poi il mare si placa. Davanti, nella nebbia, una costa.",
        ]);
        v
    };
    lines.push("Il continente del Nord.");
    lines.into_iter().map(str::to_string).collect()
}

/// Sistema: avanza la scena e, alla fine, parte il viaggio.
pub fn run_cutscene(
    time: Res<Time>,
    mut cutscene: ResMut<Cutscene>,
    mut travel: ResMut<crate::save::TravelRequest>,
    mut session: ResMut<crate::save::Session>,
    mut notice_timer: Local<f32>,
) {
    *notice_timer -= time.delta_secs();
    if *notice_timer <= 0.0 && !cutscene.notices.is_empty() {
        let text = cutscene.notices.remove(0);
        session.notify(text);
        *notice_timer = 3.2;
    }
    if !cutscene.active() {
        return;
    }
    cutscene.elapsed += time.delta_secs();
    if cutscene.elapsed >= cutscene.duration() {
        cutscene.lines.clear();
        if let Some(to) = cutscene.travel.take() {
            travel.0 = Some(to);
        }
    }
}

/// Sistema (egui): il nero e le righe della scena.
pub fn draw_cutscene(mut contexts: EguiContexts, cutscene: Res<Cutscene>) {
    if !cutscene.active() {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let size = ctx.viewport_rect().size();
    let fade = (cutscene.elapsed / FADE_SECS).clamp(0.0, 1.0);
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("cutscene"),
    ));
    painter.rect_filled(
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size),
        egui::CornerRadius::ZERO,
        egui::Color32::from_black_alpha((fade * 250.0) as u8),
    );
    let t = cutscene.elapsed - FADE_SECS;
    if t < 0.0 {
        return;
    }
    let k = (t / LINE_SECS) as usize;
    let Some(line) = cutscene.lines.get(k) else {
        return;
    };
    let local = t - k as f32 * LINE_SECS;
    let alpha = (local / 0.6).min((LINE_SECS - local) / 0.6).clamp(0.0, 1.0);
    let galley = painter.layout(
        line.clone(),
        egui::FontId::proportional(26.0),
        crate::ui_style::TEXT.gamma_multiply(alpha),
        (size.x - 160.0).min(1100.0),
    );
    let at = egui::pos2(
        size.x / 2.0 - galley.size().x / 2.0,
        size.y / 2.0 - galley.size().y / 2.0,
    );
    painter.galley(at, galley, crate::ui_style::TEXT);
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Cutscene>()
        .add_systems(Update, run_cutscene);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_departure_tells_the_way_and_the_helmsman() {
        let sea = departure_lines("mare", true);
        assert!(sea.iter().any(|l| l.contains("Gaspare tiene il timone")));
        assert!(
            !departure_lines("mare", false)
                .iter()
                .any(|l| l.contains("tiene il timone"))
        );
        let deep = departure_lines("profonda", true);
        assert!(deep.iter().any(|l| l.contains("sotto il mare")));
        assert_eq!(sea.last(), deep.last());
    }
}
