//! Luoghi da scoprire sull'isola (ADR 0029): campi, rovine, torre, mulino, grotta e santuario.
//!
//! Sono piazzamenti normali: la forma sta in [`build`], la posizione in `debug/placements.txt`
//! (`scripts/gen_luoghi.py`), i nomi sulla mappa in `assets/world/isola.txt`. I banditi ricompaiono
//! da soli e tornano anche caricando un salvataggio fatto prima che esistessero.

use crate::buildings::{B, Col, Interior, STONE, STONE_DARK, TIMBER};
use crate::debug_place::DebugPlace;
use crate::entities::{Enemy, PatrolState};
use crate::interiors as rooms;
use crate::pbr::Mat;
use crate::terrain::TerrainField;
use bevy::prelude::*;

const ASH: Col = [0.12, 0.11, 0.1, 1.0];
const LOG: Col = [0.3, 0.2, 0.12, 1.0];
/// Metri attorno al centro del campo in cui si muovono i banditi.
const BANDIT_WALK: f32 = 6.0;
/// Quanti sono i banditi per campo.
const BANDITS: usize = 3;

/// Tenda aperta sul lato +Z, con la cresta lungo Z.
fn tent(b: &mut B, c: Vec3, w: f32, l: f32, h: f32, col: Col) {
    let (x0, x1) = (c.x - w / 2.0, c.x + w / 2.0);
    let (z0, z1) = (c.z - l / 2.0, c.z + l / 2.0);
    let top = c.y + h;
    b.force = Some(Mat::Cloth);
    b.quad(
        Vec3::new(x0, c.y, z0),
        Vec3::new(x0, c.y, z1),
        Vec3::new(c.x, top, z1),
        Vec3::new(c.x, top, z0),
        Vec3::new(-1.0, 1.0, 0.0),
        col,
    );
    b.quad(
        Vec3::new(x1, c.y, z1),
        Vec3::new(x1, c.y, z0),
        Vec3::new(c.x, top, z0),
        Vec3::new(c.x, top, z1),
        Vec3::new(1.0, 1.0, 0.0),
        col,
    );
    b.tri(
        Vec3::new(x0, c.y, z0),
        Vec3::new(c.x, top, z0),
        Vec3::new(x1, c.y, z0),
        Vec3::NEG_Z,
        col,
    );
    b.force = None;
    b.beam(
        Vec3::new(c.x, top, z0),
        Vec3::new(c.x, top, z1),
        0.04,
        TIMBER,
    );
    for z in [z0, z1] {
        b.beam(Vec3::new(c.x, c.y, z), Vec3::new(c.x, top, z), 0.04, TIMBER);
    }
}

/// Giaciglio di stoffa a terra.
fn bedroll(b: &mut B, c: Vec3) {
    b.force = Some(Mat::Cloth);
    b.cuboid_c(
        c + Vec3::Y * 0.06,
        Vec3::new(0.75, 0.12, 1.8),
        [0.42, 0.34, 0.24, 1.0],
    );
    b.force = None;
}

/// Falò: cerchio di pietre, cenere e ciocchi; il fuoco lo accende `village_life`.
fn campfire(b: &mut B, int: &mut Interior, c: Vec3) {
    use std::f32::consts::TAU;
    for k in 0..9 {
        let a = k as f32 / 9.0 * TAU;
        b.cuboid_c(
            c + Vec3::new(a.cos() * 0.55, 0.08, a.sin() * 0.55),
            Vec3::new(0.2, 0.17, 0.2),
            STONE_DARK,
        );
    }
    b.cuboid_c(c + Vec3::Y * 0.02, Vec3::new(0.85, 0.04, 0.85), ASH);
    for k in 0..3 {
        let a = k as f32 / 3.0 * TAU;
        let foot = c + Vec3::new(a.cos() * 0.34, 0.04, a.sin() * 0.34);
        b.beam(foot, c + Vec3::Y * 0.42, 0.045, LOG);
    }
    int.fires.push(c + Vec3::Y * 0.42);
}

/// Campo del ladro: una tenda, un giaciglio, il falò e il suo baule.
fn thief_camp(b: &mut B, int: &mut Interior) -> Vec<(Vec3, Vec3)> {
    tent(
        b,
        Vec3::new(-1.4, 0.0, -0.4),
        2.2,
        2.8,
        1.55,
        [0.36, 0.3, 0.22, 1.0],
    );
    bedroll(b, Vec3::new(-1.4, 0.0, -0.5));
    campfire(b, int, Vec3::new(1.2, 0.0, 1.0));
    rooms::chest(b, int, 1.7, -1.3, 0.0);
    rooms::crate_box(b, int, 0.4, -1.5, 0.0, 0.55);
    vec![(Vec3::new(-1.4, 0.6, -0.4), Vec3::new(2.0, 1.2, 2.6))]
}

/// Accampamento dei banditi: tende, falò, palizzata a tratti, provviste.
fn bandit_camp(b: &mut B, int: &mut Interior) -> Vec<(Vec3, Vec3)> {
    let cloth: Col = [0.33, 0.16, 0.12, 1.0];
    tent(b, Vec3::new(-3.2, 0.0, -1.8), 2.4, 3.0, 1.7, cloth);
    tent(b, Vec3::new(3.0, 0.0, -2.0), 2.4, 3.0, 1.7, cloth);
    bedroll(b, Vec3::new(-3.2, 0.0, -2.0));
    bedroll(b, Vec3::new(3.0, 0.0, -2.2));
    campfire(b, int, Vec3::ZERO);
    rooms::barrel(b, int, -1.0, -3.6, 0.0);
    rooms::crate_box(b, int, 0.5, -3.7, 0.0, 0.7);
    rooms::chest(b, int, 1.7, -3.7, 0.0);
    let mut colliders = vec![
        (Vec3::new(-3.2, 0.6, -1.8), Vec3::new(2.2, 1.2, 2.8)),
        (Vec3::new(3.0, 0.6, -2.0), Vec3::new(2.2, 1.2, 2.8)),
    ];
    // Pali appuntiti sul retro, con varchi.
    for k in 0..16 {
        if k % 6 == 5 {
            continue;
        }
        let x = -6.0 + k as f32 * 0.8;
        let h = 1.5 + (k % 3) as f32 * 0.25;
        let foot = Vec3::new(x, 0.0, -5.2 - (k % 2) as f32 * 0.1);
        b.beam(foot, foot + Vec3::new(0.0, h, 0.12), 0.08, LOG);
        colliders.push((foot + Vec3::Y * h / 2.0, Vec3::new(0.2, h, 0.2)));
    }
    colliders
}

/// Muri spezzati e colonne di un antico tempio, con un baule in mezzo.
fn ruins(b: &mut B, int: &mut Interior) -> Vec<(Vec3, Vec3)> {
    let mut colliders = Vec::new();
    let mut wall = |b: &mut B, min: Vec3, max: Vec3| {
        b.cuboid(min, max, STONE);
        colliders.push(((min + max) / 2.0, max - min));
    };
    b.cuboid(
        Vec3::new(-6.0, -0.2, -5.0),
        Vec3::new(6.0, 0.08, 5.0),
        STONE_DARK,
    );
    wall(b, Vec3::new(-6.0, 0.0, -5.0), Vec3::new(-1.5, 3.1, -4.4));
    wall(b, Vec3::new(-1.5, 0.0, -5.0), Vec3::new(1.2, 1.5, -4.4));
    wall(b, Vec3::new(3.0, 0.0, -5.0), Vec3::new(6.0, 2.2, -4.4));
    wall(b, Vec3::new(-6.0, 0.0, -4.4), Vec3::new(-5.4, 2.4, 0.5));
    wall(b, Vec3::new(5.4, 0.0, -4.4), Vec3::new(6.0, 1.2, -1.2));
    for (x, z, h) in [
        (-3.4, 3.4, 3.6),
        (-1.2, 3.4, 1.4),
        (1.2, 3.4, 3.2),
        (3.4, 3.4, 2.1),
        (-3.4, 0.4, 2.6),
        (3.4, 0.4, 3.5),
    ] {
        wall(
            b,
            Vec3::new(x - 0.32, 0.0, z - 0.32),
            Vec3::new(x + 0.32, h, z + 0.32),
        );
    }
    // Rocchi di colonna caduti.
    for (x, z) in [(-1.0, 1.4), (2.0, 2.2), (0.2, 4.4)] {
        wall(
            b,
            Vec3::new(x - 0.6, 0.0, z - 0.3),
            Vec3::new(x + 0.6, 0.6, z + 0.3),
        );
    }
    rooms::chest(b, int, 0.0, -3.4, 0.08);
    colliders
}

/// Torre in rovina: scala interna fino alla piattaforma, parapetto spezzato e un baule in cima.
fn ruined_tower(b: &mut B, int: &mut Interior) -> f32 {
    const HW: f32 = 2.3;
    const INNER: f32 = 1.75;
    const FLOOR: f32 = 0.12;
    const TOP: f32 = 8.0;
    let door = Some((0.55, FLOOR + 2.1));
    crate::buildings::tower_shell(b, int, HW, INNER, 0.0, TOP, STONE_DARK, door);
    let rect = (-INNER, INNER, -INNER, INNER);
    rooms::slab(b, int, rect.0, rect.1, rect.2, rect.3, FLOOR);
    let holes = rooms::square_helix(b, int, rect, FLOOR, TOP, 0.85);
    rooms::slab_with_holes(b, int, (-HW, HW, -HW, HW), &holes, TOP);
    // Merli a tratti: il crollo ha portato via un lato.
    for (x0, z0, x1, z1, h) in [
        (-HW, -HW, HW, -HW + 0.4, 1.1),
        (-HW, -HW, -HW + 0.4, 0.2, 1.1),
        (HW - 0.4, -HW, HW, -0.9, 0.7),
        (-HW, 1.1, -HW + 0.4, HW, 0.5),
    ] {
        let (min, max) = (Vec3::new(x0, TOP, z0), Vec3::new(x1, TOP + h, z1));
        b.cuboid(min, max, STONE_DARK);
        int.walls.push(((min + max) / 2.0, max - min));
    }
    rooms::chest(b, int, 1.2, 1.5, TOP);
    int.lights.clear();
    TOP
}

/// Semicampo locale della grotta (larghezza, profondità): ingresso a +Z, fondo a -Z.
pub const CAVE_HALF: Vec2 = Vec2::new(5.0, 8.8);

/// Imbocco delle gallerie (minimo e massimo locali x, z): dove il terreno passerebbe in mezzo
/// alla discesa non c'è (`terrain::TerrainHole`); più in là la collina è sopra il soffitto.
pub const CAVE_HOLE: (Vec2, Vec2) = (Vec2::new(-3.9, -13.0), Vec2::new(-1.1, -8.0));

/// Numero pseudocasuale stabile in 0..1.
fn cave_rnd(i: u32) -> f32 {
    ((i.wrapping_mul(2_654_435_761) >> 8) & 0xffff) as f32 / 65535.0
}

/// Roccia piena (visibile e solida).
fn rock(b: &mut B, int: &mut Interior, a: Vec3, c: Vec3) {
    let (min, max) = (a.min(c), a.max(c));
    b.cuboid(min, max, STONE_DARK);
    int.walls.push(((min + max) / 2.0, max - min));
}

/// Pavimento di pietra su cui si cammina.
fn cave_floor(b: &mut B, int: &mut Interior, a: Vec3, c: Vec3) {
    let (min, max) = (a.min(c), a.max(c));
    b.cuboid(min, max, STONE);
    int.ground.push(((min + max) / 2.0, max - min));
}

/// Le gallerie sotto la collina, a sud della camera: una discesa, la sala del tesoro, un laghetto
/// e un cunicolo stretto che gira ad angolo e finisce in una nicchia con un forziere.
/// Quote locali: pavimento delle gallerie -1,2 (mondo ≈0,9, sopra il mare), soffitto 2,5 (0,95 nel cunicolo).
fn cave_depths(b: &mut B, int: &mut Interior) {
    const FL: f32 = -1.2; // pavimento delle gallerie
    const LO: f32 = -1.6; // base delle pareti
    const CEIL: f32 = 2.5;
    const LOW: f32 = 0.95; // soffitto del cunicolo
    let v = Vec3::new;
    // Discesa: sul varco del fondo e poi una rampa di 4 m (≈17°) fino al pavimento delle gallerie.
    cave_floor(b, int, v(-3.6, -0.2, -8.8), v(-1.4, 0.06, -8.0));
    rooms::ramp(int, v(-2.5, 0.06, -8.0), v(-2.5, FL, -12.0), 2.2);
    let slope = (0.06 - FL) / 4.0;
    for i in 0..4 {
        let z0 = -8.0 - i as f32;
        let top = 0.06 - slope * (i as f32 + 0.5) - 0.04;
        b.cuboid(
            v(-3.6, LO, z0 - 1.0),
            v(-1.4, top, z0),
            if i % 2 == 0 { STONE } else { STONE_DARK },
        );
    }
    cave_floor(b, int, v(-3.6, LO, -18.4), v(-1.4, FL, -12.0));
    // Pareti e soffitto della discesa (la parete ovest ha l'apertura verso il laghetto).
    for (min, max) in [
        (v(-4.2, LO, -13.6), v(-3.6, CEIL, -8.8)),
        (v(-4.2, LO, -18.4), v(-3.6, CEIL, -16.4)),
        (v(-1.4, LO, -18.4), v(-0.8, CEIL, -8.8)),
        (v(-4.2, CEIL, -18.4), v(-0.8, CEIL + 0.6, -8.8)),
    ] {
        rock(b, int, min, max);
    }
    // Laghetto: sala a ovest della discesa; l'acqua è bassa e si attraversa.
    // Il pavimento passa anche sotto il varco nella parete (x da -4,2 a -3,6).
    cave_floor(b, int, v(-8.4, LO, -17.6), v(-3.6, FL, -12.4));
    for (min, max) in [
        (v(-8.4, LO, -12.4), v(-3.6, CEIL, -13.6)),
        (v(-8.4, LO, -17.6), v(-7.8, CEIL, -12.4)),
        (v(-8.4, LO, -18.2), v(-3.6, CEIL, -17.0)),
        (v(-8.4, CEIL, -18.2), v(-3.6, CEIL + 0.6, -12.4)),
    ] {
        rock(b, int, min, max);
    }
    b.force = Some(Mat::Plain);
    b.cuboid_c(
        v(-6.2, FL + 0.05, -15.2),
        v(3.2, 0.1, 3.0),
        [0.08, 0.22, 0.32, 1.0],
    );
    b.force = None;
    for (x, z, h) in [
        (-7.2, -13.4, 1.1),
        (-5.6, -16.6, 0.8),
        (-7.0, -16.2, 1.5),
        (-4.9, -13.6, 0.6),
    ] {
        b.cuboid_c(v(x, FL + h / 2.0, z), v(0.3, h, 0.3), STONE_DARK);
    }
    // Sala del tesoro in fondo alla discesa.
    cave_floor(b, int, v(-6.0, LO, -24.0), v(0.4, FL, -18.4));
    for (min, max) in [
        (v(-6.6, LO, -17.8), v(-3.6, CEIL, -18.4)),
        (v(-1.4, LO, -17.8), v(1.0, CEIL, -18.4)),
        (v(-6.6, LO, -24.6), v(-6.0, CEIL, -17.8)),
        (v(-6.6, LO, -24.6), v(1.0, CEIL, -24.0)),
        // Parete est con il varco del cunicolo (z da -21,2 a -19,7).
        (v(0.4, LO, -19.7), v(1.0, CEIL, -17.8)),
        (v(0.4, LO, -24.6), v(1.0, CEIL, -21.2)),
        // Sopra il varco, dal soffitto basso del cunicolo a quello della sala: senza, restava una
        // finestra da cui si vedeva l'accampamento dei ladri (segnalazione 2026-11-19_21-49-13).
        (v(0.4, LOW, -21.2), v(1.0, CEIL, -19.7)),
        (v(-6.6, CEIL, -24.6), v(1.0, CEIL + 0.6, -17.8)),
    ] {
        rock(b, int, min, max);
    }
    campfire(b, int, v(-3.0, FL, -20.2));
    rooms::chest(b, int, -4.8, -23.2, FL);
    // Il forziere della nicchia: stava davanti alla Porta Sommersa e chiudeva il passaggio
    // (segnalazione 2026-11-19_22-10-52).
    rooms::chest(b, int, -2.8, -23.4, FL);
    rooms::barrel(b, int, -0.9, -23.2, FL);
    rooms::crate_box(b, int, -5.3, -19.2, FL, 0.7);
    // Cunicolo stretto (soffitto basso): va a est e gira a sud fino a una nicchia.
    cave_floor(b, int, v(0.4, LO, -21.2), v(6.0, FL, -19.7));
    cave_floor(b, int, v(4.6, LO, -27.0), v(6.0, FL, -21.2));
    for (min, max) in [
        (v(0.4, LO, -19.1), v(6.6, LOW, -19.7)),
        (v(0.4, LO, -21.8), v(4.6, LOW, -21.2)),
        (v(6.0, LO, -27.6), v(6.6, LOW, -19.1)),
        (v(4.0, LO, -27.6), v(4.6, LOW, -21.8)),
        // Fondo della nicchia: il vano della Porta Sommersa (x da 4,9 a 5,7, alto 1,95).
        (v(4.0, LO, -27.6), v(SUNKEN_X0, LOW, -27.0)),
        (v(SUNKEN_X1, LO, -27.6), v(6.6, LOW, -27.0)),
        (v(SUNKEN_X0, FL + SUNKEN_H, -27.6), v(SUNKEN_X1, LOW, -27.0)),
        (v(0.4, LOW, -27.6), v(6.6, LOW + 0.6, -19.1)),
    ] {
        rock(b, int, min, max);
    }
    campfire(b, int, v(5.3, FL, -23.2));
    sunken_passage(b, int);
}

/// Vano della Porta Sommersa in fondo alla nicchia (coordinate locali della grotta).
const SUNKEN_X0: f32 = 4.9;
const SUNKEN_X1: f32 = 5.7;
const SUNKEN_H: f32 = 1.95;
/// Faccia interna della parete di fondo della nicchia.
const SUNKEN_Z: f32 = -27.0;
/// In fondo ai gradini oltre la Porta Sommersa (coordinate della grotta): lì comincia la via sotto il mare.
pub const SUNKEN_DEEP: Vec3 = Vec3::new(5.3, -1.95, -31.0);

/// Oltre la Porta Sommersa: un breve corridoio e gradini che spariscono in un'acqua ferma, la «via
/// sotto il mare» (ADR 0048): in fondo ai gradini comincia la traversata (`crossing`, ADR 0054). Una cornice di rame a spirali segna il vano dal lato della nicchia.
fn sunken_passage(b: &mut B, int: &mut Interior) {
    const FL: f32 = -1.2;
    const LO: f32 = -2.2;
    const TOP: f32 = 0.95;
    let v = Vec3::new;
    cave_floor(b, int, v(4.6, LO, -29.0), v(6.0, FL, -27.6));
    // Gradini che scendono nell'acqua: la via sotto il mare comincia qui (`crossing`, ADR 0054).
    for (i, z) in [-29.0f32, -29.6, -30.2].into_iter().enumerate() {
        let top = FL - 0.25 * (i as f32 + 1.0);
        cave_floor(b, int, v(4.6, LO, z - 0.6), v(6.0, top, z));
    }
    cave_floor(b, int, v(4.6, LO, -31.8), v(6.0, FL - 0.75, -30.8));
    b.force = Some(Mat::Plain);
    b.cuboid_c(
        v(5.3, FL - 0.3, -30.4),
        v(1.4, 0.04, 2.8),
        [0.05, 0.16, 0.24, 1.0],
    );
    b.force = None;
    for (min, max) in [
        (v(4.0, LO, -32.4), v(4.6, TOP, -27.6)),
        (v(6.0, LO, -32.4), v(6.6, TOP, -27.6)),
        (v(4.0, LO, -32.4), v(6.6, TOP, -31.8)),
        (v(4.0, TOP, -32.4), v(6.6, TOP + 0.6, -27.6)),
    ] {
        rock(b, int, min, max);
    }
    // Cornice di rame: stipiti, architrave e due spirali ai lati.
    const COPPER: Col = [0.42, 0.2, 0.09, 1.0];
    let z = SUNKEN_Z + 0.03;
    b.force = Some(Mat::Plain);
    for x in [SUNKEN_X0 - 0.06, SUNKEN_X1 + 0.06] {
        b.cuboid_c(v(x, FL + SUNKEN_H / 2.0, z), v(0.1, SUNKEN_H, 0.06), COPPER);
    }
    b.cuboid_c(
        v((SUNKEN_X0 + SUNKEN_X1) / 2.0, FL + SUNKEN_H + 0.05, z),
        v(SUNKEN_X1 - SUNKEN_X0 + 0.22, 0.1, 0.06),
        COPPER,
    );
    for (cx, s) in [(SUNKEN_X0 - 0.35, 1.0f32), (SUNKEN_X1 + 0.35, -1.0)] {
        for k in 0..7 {
            let a = k as f32 * 0.9;
            let r = 0.04 + k as f32 * 0.025;
            b.cuboid_c(
                v(cx + s * a.cos() * r, FL + 1.3 + a.sin() * r, z),
                v(0.05, 0.05, 0.04),
                COPPER,
            );
        }
    }
    b.force = None;
}

/// La Camera di Risonanza sotto il pozzo della piazza (QS-12, ADR 0052): il pozzetto con la scala
/// a chiocciola, accanto al pozzo (minimo e massimo locali x, z), dove il terreno non c'è.
pub const WELL_HOLE: (Vec2, Vec2) = (Vec2::new(1.6, 1.6), Vec2::new(4.0, 4.0));
/// La canna del pozzo (semilato interno): dal parapetto scende vuota fino alla vasca della camera.
pub const WELL_SHAFT: f32 = 0.8;
/// Buco nel terreno dentro il parapetto: senza, la canna finiva sul prato a 45 cm dal bordo.
pub const WELL_SHAFT_HOLE: (Vec2, Vec2) = (
    Vec2::new(-WELL_SHAFT, -WELL_SHAFT),
    Vec2::new(WELL_SHAFT, WELL_SHAFT),
);
/// Quota (locale del pozzo) dell'acqua: quella della vasca nella Camera di Risonanza, 4 m sotto il
/// bordo (segnalazioni 2026-11-18_22-36-48 e 23-57-19: il pozzo sembrava una pozzanghera).
pub const WELL_WATER_Y: f32 = RES_FLOOR + 0.34;
/// Pavimento e soffitto della camera (quote locali del pozzo).
const RES_FLOOR: f32 = -3.4;
const RES_CEIL: f32 = -0.6;
const COPPER_COL: Col = [0.42, 0.2, 0.09, 1.0];

/// La camera sotto il pozzo: una stanza di pietra con la colonna del pozzo che scende in una vasca,
/// il cavo di rame della Luce Viva che corre verso est (QS-13) e la scala a chiocciola nel pozzetto.
pub fn resonance_chamber(b: &mut B, int: &mut Interior) {
    let v = Vec3::new;
    let (x0, x1, z0, z1) = (-3.0, 4.0, -3.0, 4.0);
    let (hx0, hz0) = (WELL_HOLE.0.x, WELL_HOLE.0.y);
    cave_floor(b, int, v(x0, RES_FLOOR - 0.3, z0), v(x1, RES_FLOOR, z1));
    // Muri e soffitto (il soffitto ha il vano del pozzetto).
    for (min, max) in [
        (
            v(x0 - 0.5, RES_FLOOR - 0.3, z0 - 0.5),
            v(x0, RES_CEIL, z1 + 0.5),
        ),
        (
            v(x1, RES_FLOOR - 0.3, z0 - 0.5),
            v(x1 + 0.5, RES_CEIL, z1 + 0.5),
        ),
        (v(x0, RES_FLOOR - 0.3, z0 - 0.5), v(x1, RES_CEIL, z0)),
        (v(x0, RES_FLOOR - 0.3, z1), v(x1, RES_CEIL, z1 + 0.5)),
        // Il soffitto lascia aperta la canna del pozzo (x e z da -0,8 a 0,8).
        (v(x0 - 0.5, RES_CEIL, z0 - 0.5), v(-WELL_SHAFT, -0.05, hz0)),
        (v(WELL_SHAFT, RES_CEIL, z0 - 0.5), v(x1 + 0.5, -0.05, hz0)),
        (
            v(-WELL_SHAFT, RES_CEIL, z0 - 0.5),
            v(WELL_SHAFT, -0.05, -WELL_SHAFT),
        ),
        (
            v(-WELL_SHAFT, RES_CEIL, WELL_SHAFT),
            v(WELL_SHAFT, -0.05, hz0),
        ),
        (v(x0 - 0.5, RES_CEIL, hz0), v(hx0, -0.05, z1 + 0.5)),
    ] {
        b.cuboid(min, max, STONE);
        int.walls.push(((min + max) / 2.0, max - min));
    }
    // Il pozzetto: pareti sotto il bordo del buco nel terreno su tutti e quattro i lati, poi la
    // chiocciola fino al pavimento. Le pareti salgono a cordolo un poco sopra il selciato: il
    // terreno attorno sta più in alto della base del pozzo e tra bordo e parete si vedeva il cielo
    // (segnalazione 2026-11-18_23-52-52). Il collider resta a filo, niente gradino all'uscita.
    const CURB: f32 = 0.35;
    // Il cordolo sporge 0,6 m fuori dal buco: copre dall'alto il bordo del terreno tagliato.
    const RIM_OUT: f32 = 0.6;
    for (min, max) in [
        (
            v(x1, RES_CEIL, hz0 - RIM_OUT),
            v(x1 + RIM_OUT, CURB, z1 + RIM_OUT),
        ),
        (v(hx0 - RIM_OUT, RES_CEIL, z1), v(x1, CURB, z1 + RIM_OUT)),
        (v(hx0 - RIM_OUT, RES_CEIL, hz0 - RIM_OUT), v(hx0, CURB, z1)),
        (v(hx0, RES_CEIL, hz0 - RIM_OUT), v(x1, CURB, hz0)),
    ] {
        b.cuboid(min, max, STONE_DARK);
        let solid = Vec3::new(max.x, max.y.min(0.0), max.z);
        int.walls.push(((min + solid) / 2.0, solid - min));
    }
    rooms::square_helix(b, int, (hx0, x1, hz0, z1), RES_FLOOR, 0.06, 0.8);
    // La colonna del pozzo scende fino a una vasca d'acqua ferma: cava, è la canna del pozzo, e
    // dall'alto si vede l'acqua della vasca giù in fondo.
    let (o, i) = (1.0, WELL_SHAFT);
    for (min, max) in [
        (v(-o, RES_FLOOR + 0.3, -o), v(o, -0.05, -i)),
        (v(-o, RES_FLOOR + 0.3, i), v(o, -0.05, o)),
        (v(-o, RES_FLOOR + 0.3, -i), v(-i, -0.05, i)),
        (v(i, RES_FLOOR + 0.3, -i), v(o, -0.05, i)),
    ] {
        rock(b, int, min, max);
    }
    b.cuboid(
        v(-1.8, RES_FLOOR, -1.8),
        v(1.8, RES_FLOOR + 0.35, 1.8),
        STONE_DARK,
    );
    int.walls
        .push((v(0.0, RES_FLOOR + 0.17, 0.0), v(3.6, 0.35, 3.6)));
    b.force = Some(Mat::Plain);
    b.cuboid_c(
        v(0.0, RES_FLOOR + 0.3, 0.0),
        v(3.3, 0.04, 3.3),
        [0.05, 0.18, 0.26, 1.0],
    );
    // Il cavo di rame: esce dalla colonna e corre verso est, dentro il muro.
    b.cuboid_c(
        v((1.0 + x1) / 2.0, -1.7, 0.0),
        v(x1 - 1.0, 0.12, 0.12),
        COPPER_COL,
    );
    // Segni di rame sui muri: spirali come quelle della Porta Sommersa.
    for (x, z) in [(x0 + 0.03, 0.5), (x0 + 0.03, -1.8), (-1.5, z0 + 0.03)] {
        for k in 0..7 {
            let a = k as f32 * 0.9;
            let r = 0.05 + k as f32 * 0.03;
            let along_x = z == z0 + 0.03;
            let p = if along_x {
                v(x + a.cos() * r, -1.8 + a.sin() * r, z)
            } else {
                v(x, -1.8 + a.sin() * r, z + a.cos() * r)
            };
            b.cuboid_c(p, v(0.06, 0.06, 0.06), COPPER_COL);
        }
    }
    b.force = None;
}

/// Dove si tocca l'acqua della vasca nella Camera di Risonanza (dal centro del pozzo, in piano).
const BASIN_REACH: f32 = 2.8;

/// Il bagliore del cavo di rame quando la camera risuona: si spegne da solo.
#[derive(Component)]
pub struct ResonancePulse {
    left: f32,
}

/// Sistema: nella Camera di Risonanza, `E` vicino alla vasca fa risuonare la stanza (QS-12): un
/// accordo, il cavo di rame che si accende verso est e, la prima volta, la lamina di rame con la
/// rotta del cavo verso nord (`risonanza`).
#[allow(clippy::too_many_arguments)]
pub fn resonance_basin(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    dialog: Res<crate::entities::DialogState>,
    karma: Res<crate::karma::Karma>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    wells: Query<&GlobalTransform, With<WellMouth>>,
    mut nearest: ResMut<crate::entities::NearestInteract>,
    mut session: ResMut<crate::save::Session>,
) {
    if dialog.open || crate::region::current() != crate::region::Region::Isola {
        return;
    }
    let (Ok(player), Some(well)) = (players.single(), wells.iter().next()) else {
        return;
    };
    let p = player.translation();
    let w = well.translation();
    let below = p.y < w.y + RES_FLOOR + 2.2;
    if !below || p.xz().distance(w.xz()) > BASIN_REACH {
        return;
    }
    let hint = "[E] Tocca l'acqua della vasca".to_string();
    nearest.prompt = Some(match nearest.prompt.take() {
        Some(prompt) => format!("{prompt} · {hint}"),
        None => hint,
    });
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    crate::sfx::play(crate::sfx::Sfx::Ring);
    // Il cavo verso est si accende per qualche secondo, dalla colonna al muro.
    let rot = well.rotation();
    for x in [1.6f32, 2.6, 3.6] {
        commands.spawn((
            crate::scene::SceneNode,
            ResonancePulse { left: 3.0 },
            PointLight {
                color: Color::srgb(1.0, 0.62, 0.3),
                intensity: 40_000.0,
                range: 4.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(w + rot * Vec3::new(x, -1.55, 0.0)),
        ));
    }
    if karma.unlocks.contains("risonanza") {
        session.notify("La stanza risuona ancora: il cavo si accende verso est");
        return;
    }
    crate::karma::flag("risonanza");
    crate::inventory::grant(
        crate::inventory::ItemKind::Relic(crate::inventory::Relic::ResonancePlate),
        1,
    );
    crate::skills::train(crate::skills::Skill::Lore, 30.0);
    session.notify(
        "L'acqua trema, la stanza canta. Nella vasca, una lamina di rame incisa: il cavo, e una linea verso nord",
    );
    info!("[MONDO] la Camera di Risonanza risuona");
}

/// Sistema: il bagliore del cavo si spegne in pochi secondi.
pub fn resonance_pulse(
    mut commands: Commands,
    time: Res<Time>,
    mut pulses: Query<(Entity, &mut ResonancePulse, &mut PointLight)>,
) {
    for (e, mut pulse, mut light) in &mut pulses {
        pulse.left -= time.delta_secs();
        light.intensity = 40_000.0 * (pulse.left / 3.0).max(0.0);
        if pulse.left <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

/// Segna il pozzo della piazza: sotto c'è la Camera di Risonanza.
#[derive(Component)]
pub struct WellMouth;

/// La grata sul pozzetto: chiusa finché la Croce delle Quattro Ronde non scopre la camera.
#[derive(Component)]
pub struct WellGrate;

/// Sistema: la grata sul pozzetto accanto al pozzo, e la luce dell'acqua nella camera. Si apre (la
/// grata si appoggia al pozzo) quando la camera è scoperta (`camera-risonanza`, QS-12).
#[allow(clippy::too_many_arguments)]
pub fn well_grate(
    mut commands: Commands,
    wells: Query<(Entity, &Children), With<WellMouth>>,
    mut grates: Query<(Entity, &mut Transform, Has<avian3d::prelude::Collider>), With<WellGrate>>,
    missions: Res<crate::quests::missions::Missions>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut iron: Local<Option<Handle<StandardMaterial>>>,
) {
    let open = missions.unlocked("camera-risonanza");
    let size = WELL_HOLE.1 - WELL_HOLE.0;
    let center = (WELL_HOLE.0 + WELL_HOLE.1) / 2.0;
    // Chiusa, appoggiata sul cordolo del pozzetto: a livello del selciato attorno (segnalazione
    // 2026-11-18_23-52-52), non più giù nel buco.
    let closed_tf = Transform::from_xyz(center.x, 0.37, center.y);
    // Aperta: alzata e appoggiata al pozzo, di taglio.
    let open_tf =
        Transform::from_xyz(1.3, 1.1, center.y).with_rotation(Quat::from_rotation_z(1.25));
    for (well, children) in &wells {
        if children.iter().any(|c| grates.contains(c)) {
            continue;
        }
        let iron = iron
            .get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.12, 0.12, 0.13),
                    metallic: 0.8,
                    perceptual_roughness: 0.6,
                    ..default()
                })
            })
            .clone();
        commands.entity(well).with_children(|c| {
            let mut grate = c.spawn((
                WellGrate,
                Mesh3d(meshes.add(Cuboid::new(size.x, 0.05, 0.08))),
                MeshMaterial3d(iron.clone()),
                if open { open_tf } else { closed_tf },
                Visibility::default(),
            ));
            if !open {
                grate.insert(avian3d::prelude::Collider::cuboid(size.x, 0.1, size.y));
            }
            grate.with_children(|g| {
                // Le sbarre: sette di traverso, più la cornice.
                for k in 0..7 {
                    let z = -size.y / 2.0 + (k as f32 + 0.5) * size.y / 7.0;
                    g.spawn((
                        Mesh3d(meshes.add(Cuboid::new(size.x, 0.05, 0.06))),
                        MeshMaterial3d(iron.clone()),
                        Transform::from_xyz(0.0, 0.0, z),
                    ));
                }
                for x in [-size.x / 2.0, size.x / 2.0] {
                    g.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.08, 0.06, size.y))),
                        MeshMaterial3d(iron.clone()),
                        Transform::from_xyz(x, 0.0, 0.0),
                    ));
                }
            });
            // La luce dell'acqua, giù nella camera.
            c.spawn((
                PointLight {
                    color: Color::srgb(0.4, 0.7, 0.85),
                    intensity: 25_000.0,
                    range: 9.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, RES_FLOOR + 1.2, 2.0),
            ));
        });
        info!(
            "[MONDO] grata del pozzo {}",
            if open { "aperta" } else { "chiusa" }
        );
    }
    for (e, mut tf, solid) in &mut grates {
        if open && solid {
            commands.entity(e).remove::<avian3d::prelude::Collider>();
            *tf = open_tf;
            crate::sfx::play(crate::sfx::Sfx::Grind);
            info!("[MONDO] la grata del pozzo si apre: si scende nella Camera di Risonanza");
        }
    }
}

/// Segna la grotta piazzata: lì sotto c'è la Porta Sommersa.
#[derive(Component)]
pub struct CaveMouth;

/// La collina sopra le gallerie della grotta è già stata riempita di roccia.
#[derive(Component)]
pub struct CaveFilled;

/// Le gallerie sotto la collina (coordinate locali della grotta): x, z minimi e massimi e quota del
/// tetto (faccia di sopra). Discesa, laghetto e sala del tesoro hanno il tetto a `CEIL + 0,6`;
/// cunicolo e corridoio della Porta Sommersa a `LOW + 0,6`. Vedi [`cave_depths`].
const CAVE_GALLERIES: [(f32, f32, f32, f32, f32); 5] = [
    (-4.2, -0.8, -18.4, -8.8, 3.1),
    (-8.4, -3.6, -18.2, -12.4, 3.1),
    (-6.6, 1.0, -24.6, -17.8, 3.1),
    (0.4, 6.6, -27.6, -19.1, 1.55),
    (4.0, 6.6, -32.4, -27.6, 1.55),
];
/// Riquadro (locale) riempito: tutte le gallerie, dal fondo della camera in giù.
const CAVE_FILL: (Vec2, Vec2) = (Vec2::new(-8.4, -32.4), Vec2::new(6.6, -8.8));
/// Lato delle colonne di roccia.
const CAVE_FILL_CELL: f32 = 1.0;
/// Quota (locale) da cui partono le colonne fuori dalle gallerie: la base delle loro pareti.
const CAVE_FILL_LOW: f32 = -1.6;

/// Colonne di roccia sopra le gallerie, fino al terreno: `(min, max)` locali per ogni cella in cui
/// il terreno sta più in alto del tetto. Il terreno è un heightfield e la collina sopra le gallerie
/// arriva a 6-8,5 m mentre i tetti stanno a 3,6-5,1 m: nello spazio in mezzo si camminava sotto la
/// superficie, si vedeva fuori da sottoterra e si usciva dalla mappa (segnalazioni del
/// 2026-11-19: 21-49-13, 21-52-43, 21-53-53). `ground(x, z)` è la quota del terreno nel punto
/// locale, già riportata in coordinate locali.
pub fn cave_fill_columns(ground: impl Fn(f32, f32) -> f32) -> Vec<(Vec3, Vec3)> {
    let (lo, hi) = CAVE_FILL;
    let nx = ((hi.x - lo.x) / CAVE_FILL_CELL).ceil() as i32;
    let nz = ((hi.y - lo.y) / CAVE_FILL_CELL).ceil() as i32;
    let mut out = Vec::new();
    for iz in 0..nz {
        for ix in 0..nx {
            let x0 = lo.x + ix as f32 * CAVE_FILL_CELL;
            let z0 = lo.y + iz as f32 * CAVE_FILL_CELL;
            let (x1, z1) = (
                (x0 + CAVE_FILL_CELL).min(hi.x),
                (z0 + CAVE_FILL_CELL).min(hi.y),
            );
            // Sopra una galleria si parte dal tetto più alto che tocca la cella, così la colonna
            // non entra mai in una stanza; altrove dalla base delle pareti.
            let bottom = CAVE_GALLERIES
                .iter()
                .filter(|(gx0, gx1, gz0, gz1, _)| x0 < *gx1 && x1 > *gx0 && z0 < *gz1 && z1 > *gz0)
                .map(|g| g.4)
                .reduce(f32::max)
                .unwrap_or(CAVE_FILL_LOW);
            // La quota più alta della cella: la colonna chiude anche il lato in salita.
            let top = [
                (x0, z0),
                (x1, z0),
                (x0, z1),
                (x1, z1),
                ((x0 + x1) / 2.0, (z0 + z1) / 2.0),
            ]
            .into_iter()
            .map(|(x, z)| ground(x, z))
            .fold(f32::MIN, f32::max);
            if top > bottom + 0.05 {
                out.push((Vec3::new(x0, bottom, z0), Vec3::new(x1, top, z1)));
            }
        }
    }
    out
}

/// Sistema: quando la grotta è appoggiata al terreno, riempie di roccia la collina sopra le
/// gallerie ([`cave_fill_columns`]): collider e, per i punti in cui spunta, la pietra.
#[allow(clippy::type_complexity)]
pub fn fill_cave_hill(
    mut commands: Commands,
    field: Option<Res<TerrainField>>,
    caves: Query<
        (Entity, &Transform),
        (
            With<CaveMouth>,
            Without<CaveFilled>,
            Without<crate::props::SnapToGround>,
        ),
    >,
    pbr: Option<Res<crate::pbr::PbrMaterials>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let (Some(field), Some(pbr)) = (field, pbr) else {
        return;
    };
    for (cave, tf) in &caves {
        let columns = cave_fill_columns(|x, z| {
            let w = tf.transform_point(Vec3::new(x, 0.0, z));
            field.height_at(w.x, w.z) - tf.translation.y
        });
        let mut b = B::default();
        for (min, max) in &columns {
            b.cuboid(*min, *max, STONE_DARK);
        }
        info!(
            "[MONDO] collina della grotta riempita: {} colonne di roccia",
            columns.len()
        );
        commands.entity(cave).insert(CaveFilled).with_children(|c| {
            for (m, mesh) in b.finish_parts() {
                c.spawn((
                    Mesh3d(meshes.add(crate::pbr::gpu_only(mesh))),
                    MeshMaterial3d(pbr.get(m)),
                ));
            }
            for (min, max) in columns {
                c.spawn((
                    avian3d::prelude::Collider::cuboid(max.x - min.x, max.y - min.y, max.z - min.z),
                    Transform::from_translation((min + max) / 2.0),
                    crate::camera::BlocksCamera,
                ));
            }
        });
    }
}

/// La lastra della Porta Sommersa: chiusa finché le stele non risuonano, poi scende nel pavimento.
#[derive(Component)]
pub struct SunkenDoor {
    /// 0 = chiusa, 1 = del tutto aperta.
    pub open: f32,
}

/// La porta si apre quando le stele del laghetto risuonano (o è già stata trovata aperta).
pub fn sunken_door_unlocked(karma: &crate::karma::Karma) -> bool {
    karma.unlocks.contains("stele-risonanti") || karma.unlocks.contains("porta-sommersa")
}

/// Sistema: mette la lastra nel vano di ogni grotta e la fa scendere quando le stele cantano.
#[allow(clippy::too_many_arguments)]
pub fn sunken_door(
    mut commands: Commands,
    caves_built: Query<(Entity, &Children), With<CaveMouth>>,
    mut doors: Query<(Entity, &mut SunkenDoor, &mut Transform)>,
    karma: Res<crate::karma::Karma>,
    pbr: Option<Res<crate::pbr::PbrMaterials>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut copper: Local<Option<Handle<StandardMaterial>>>,
) {
    let unlocked = sunken_door_unlocked(&karma);
    let closed_at = Vec3::new(
        (SUNKEN_X0 + SUNKEN_X1) / 2.0,
        -1.2 + SUNKEN_H / 2.0,
        SUNKEN_Z - 0.3,
    );
    for (cave, children) in &caves_built {
        if children.iter().any(|c| doors.contains(c)) {
            continue;
        }
        let Some(pbr) = &pbr else {
            return;
        };
        let copper = copper
            .get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.66, 0.38, 0.2),
                    metallic: 1.0,
                    perceptual_roughness: 0.38,
                    ..default()
                })
            })
            .clone();
        let open = if unlocked { 1.0 } else { 0.0 };
        let w = SUNKEN_X1 - SUNKEN_X0;
        commands.entity(cave).with_children(|c| {
            let mut slab = c.spawn((
                SunkenDoor { open },
                Mesh3d(meshes.add(Cuboid::new(w, SUNKEN_H, 0.22))),
                MeshMaterial3d(pbr.get(Mat::Stone)),
                Transform::from_translation(closed_at - Vec3::Y * SUNKEN_H * open),
                Visibility::default(),
            ));
            if !unlocked {
                slab.insert(avian3d::prelude::Collider::cuboid(w, SUNKEN_H, 0.22));
            }
            slab.with_children(|s| {
                // Fasce di rame e un disco al centro, sul lato della nicchia.
                for y in [-0.7f32, 0.0, 0.7] {
                    s.spawn((
                        Mesh3d(meshes.add(Cuboid::new(w - 0.04, 0.06, 0.03))),
                        MeshMaterial3d(copper.clone()),
                        Transform::from_xyz(0.0, y, 0.12),
                    ));
                }
                s.spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.2, 0.03))),
                    MeshMaterial3d(copper.clone()),
                    Transform::from_xyz(0.0, 0.35, 0.125)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                ));
            });
            // Il bagliore dell'acqua oltre la porta.
            c.spawn((
                PointLight {
                    color: Color::srgb(0.35, 0.75, 0.85),
                    intensity: 0.0,
                    range: 6.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                SunkenGlow,
                Transform::from_xyz(5.3, -0.6, -30.0),
            ));
        });
        info!(
            "[MONDO] Porta Sommersa {}",
            if unlocked { "aperta" } else { "chiusa" }
        );
    }
    let dt = time.delta_secs();
    for (e, mut door, mut tf) in &mut doors {
        if unlocked && door.open < 1.0 {
            if door.open == 0.0 {
                commands.entity(e).remove::<avian3d::prelude::Collider>();
                crate::sfx::play(crate::sfx::Sfx::Grind);
                info!("[MONDO] la Porta Sommersa si apre");
            }
            door.open = (door.open + dt / 1.6).min(1.0);
            tf.translation = closed_at - Vec3::Y * SUNKEN_H * door.open;
        }
    }
}

/// Luce dell'acqua oltre la Porta Sommersa: si accende con la porta aperta.
#[derive(Component)]
pub struct SunkenGlow;

/// Sistema: il bagliore oltre la porta segue l'apertura della lastra.
pub fn sunken_glow(doors: Query<&SunkenDoor>, mut glows: Query<&mut PointLight, With<SunkenGlow>>) {
    let open = doors.iter().map(|d| d.open).fold(0.0f32, f32::max);
    for mut l in &mut glows {
        l.intensity = 30_000.0 * open;
    }
}

/// Grotta: ingresso ad arco, corridoio di 6 m e una camera ampia, tutto in un cumulo di massi
/// irregolari; dentro è buio, rischiarato solo da un falò.
fn cave(b: &mut B, int: &mut Interior) {
    const H: f32 = 3.0; // soffitto della camera
    const CH: f32 = 2.5; // soffitto del corridoio
    const Z_IN: f32 = 8.8; // facciata d'ingresso
    const Z_BACK: f32 = -8.8; // fondo
    const Z_MID: f32 = 2.0; // dove il corridoio entra nella camera
    let mut wall = |b: &mut B, min: Vec3, max: Vec3| {
        b.cuboid(min, max, STONE_DARK);
        int.walls.push(((min + max) / 2.0, max - min));
    };
    for (min, max) in [
        // Fondo e fianchi della camera.
        // Il fondo ha un varco (x da -3,6 a -1,4) verso le gallerie.
        (
            Vec3::new(-4.8, 0.0, Z_BACK),
            Vec3::new(-3.6, H, Z_BACK + 0.8),
        ),
        (
            Vec3::new(-1.4, 0.0, Z_BACK),
            Vec3::new(4.8, H, Z_BACK + 0.8),
        ),
        (
            Vec3::new(-3.6, 2.5, Z_BACK),
            Vec3::new(-1.4, H, Z_BACK + 0.8),
        ),
        (
            Vec3::new(-4.8, 0.0, Z_BACK + 0.8),
            Vec3::new(-4.0, H, Z_MID),
        ),
        (Vec3::new(4.0, 0.0, Z_BACK + 0.8), Vec3::new(4.8, H, Z_MID)),
        // Parete fra camera e corridoio, con il varco.
        (Vec3::new(-4.8, 0.0, Z_MID), Vec3::new(-1.2, H, Z_MID + 0.8)),
        (Vec3::new(1.2, 0.0, Z_MID), Vec3::new(4.8, H, Z_MID + 0.8)),
        // Fianchi del corridoio.
        (Vec3::new(-2.0, 0.0, Z_MID + 0.8), Vec3::new(-1.2, CH, Z_IN)),
        (Vec3::new(1.2, 0.0, Z_MID + 0.8), Vec3::new(2.0, CH, Z_IN)),
        // Soffitti.
        (
            Vec3::new(-4.8, H, Z_BACK),
            Vec3::new(4.8, H + 0.8, Z_MID + 0.8),
        ),
        (
            Vec3::new(-2.0, CH, Z_MID + 0.8),
            Vec3::new(2.0, CH + 0.7, Z_IN),
        ),
        (Vec3::new(-1.2, CH, Z_MID), Vec3::new(1.2, H, Z_MID + 0.8)),
    ] {
        wall(b, min, max);
    }
    for (min, max) in [
        (
            Vec3::new(-4.0, -0.2, Z_BACK + 0.8),
            Vec3::new(4.0, 0.06, Z_MID),
        ),
        (
            Vec3::new(-1.2, -0.2, Z_MID),
            Vec3::new(1.2, 0.06, Z_IN + 0.4),
        ),
    ] {
        b.cuboid(min, max, STONE);
        int.ground.push(((min + max) / 2.0, max - min));
    }
    // Rampa di pietra fuori dall'arco: il terreno davanti scende ripido verso il mare. La rampa
    // (≈19°) è ciò su cui si cammina; sotto, un terrapieno a gradoni la sostiene.
    let (ramp_top, ramp_end, ramp_drop) = (Z_IN + 0.4, Z_IN + 6.8, 2.16);
    rooms::ramp(
        int,
        Vec3::new(0.0, 0.06, ramp_top),
        Vec3::new(0.0, 0.06 - ramp_drop, ramp_end),
        2.8,
    );
    let slope = ramp_drop / (ramp_end - ramp_top);
    for i in 0..6 {
        let z0 = ramp_top + i as f32 * (ramp_end - ramp_top) / 6.0;
        let z1 = z0 + (ramp_end - ramp_top) / 6.0;
        let top = 0.06 - slope * (z0 - ramp_top + 0.2) - 0.25;
        b.cuboid(
            Vec3::new(-1.5, -3.0, z0),
            Vec3::new(1.5, top, z1),
            if i % 2 == 0 { STONE } else { STONE_DARK },
        );
    }
    // Ingresso ad arco a gradoni: stipiti e due ordini di conci sopra il vano.
    for s in [-1.0f32, 1.0] {
        let (a, c) = (1.2 * s, 2.35 * s);
        b.cuboid(
            Vec3::new(a.min(c), 0.0, Z_IN - 0.2),
            Vec3::new(a.max(c), CH + 0.2, Z_IN + 0.5),
            STONE_DARK,
        );
        b.cuboid(
            Vec3::new(s * 1.0 - 0.45, CH, Z_IN - 0.2),
            Vec3::new(s * 1.0 + 0.45, CH + 0.5, Z_IN + 0.6),
            STONE,
        );
    }
    b.cuboid(
        Vec3::new(-0.8, CH + 0.4, Z_IN - 0.2),
        Vec3::new(0.8, CH + 0.95, Z_IN + 0.7),
        STONE,
    );
    // Il cumulo di massi sopra e attorno: blocchi di misura e quota variabili.
    let mut k = 0u32;
    for ix in -2..=2i32 {
        for iz in -4..=4i32 {
            k += 1;
            let (fx, fz) = (ix as f32, iz as f32);
            let x = fx * 2.1 + (cave_rnd(k) - 0.5) * 0.9;
            let z = fz * 2.0 + (cave_rnd(k + 91) - 0.5) * 0.9;
            // Più alto al centro, più basso ai bordi e verso l'ingresso.
            let dome = 1.0 - (x.abs() / 6.2).powi(2) - ((z + 1.0).abs() / 11.0).powi(2);
            if dome <= 0.05 {
                continue;
            }
            let base = if z > Z_MID + 0.8 { CH + 0.7 } else { H + 0.8 };
            let hgt = 0.5 + dome * 2.4 * (0.6 + 0.8 * cave_rnd(k + 17));
            let half = Vec3::new(
                1.0 + cave_rnd(k + 3) * 0.7,
                hgt / 2.0,
                1.0 + cave_rnd(k + 5) * 0.7,
            );
            let center = Vec3::new(x, base - 0.1 + half.y, z);
            b.cuboid_c(
                center,
                half * 2.0,
                if k.is_multiple_of(3) {
                    STONE
                } else {
                    STONE_DARK
                },
            );
            // Solidi anche loro: si cammina sul cumulo, non ci si affonda dentro
            // (segnalazione 2026-11-19_22-08-49).
            int.walls.push((center, half * 2.0));
        }
    }
    let (peak_base, peak_half, peak_h) = (Vec3::new(0.0, H + 2.2, -3.0), 5.4, 2.6);
    b.pyramid(peak_base, peak_half, peak_h, STONE_DARK);
    // La cima a piramide, solida a gradoni (quattro strati sempre più stretti).
    const LAYERS: usize = 4;
    for i in 0..LAYERS {
        let layer_h = peak_h / LAYERS as f32;
        let half = peak_half * (1.0 - (i as f32 + 0.5) / LAYERS as f32);
        int.walls.push((
            peak_base + Vec3::Y * (layer_h * (i as f32 + 0.5)),
            Vec3::new(half * 2.0, layer_h, half * 2.0),
        ));
    }
    cave_depths(b, int);
    campfire(b, int, Vec3::new(-1.6, 0.06, -3.0));
    bedroll(b, Vec3::new(-3.0, 0.06, -4.4));
    rooms::chest(b, int, 2.6, -5.0, 0.06);
    rooms::barrel(b, int, 3.4, -3.0, 0.06);
    rooms::crate_box(b, int, 1.4, -6.0, 0.06, 0.7);
}

/// Santuario: basamento, altare, quattro colonne e un tetto a piramide.
fn shrine(b: &mut B, _int: &mut Interior) -> Vec<(Vec3, Vec3)> {
    b.cuboid(Vec3::new(-2.4, -0.2, -2.4), Vec3::new(2.4, 0.2, 2.4), STONE);
    let mut colliders = vec![(Vec3::new(0.0, 0.55, -0.6), Vec3::new(1.2, 0.9, 0.7))];
    b.cuboid_c(
        Vec3::new(0.0, 0.55, -0.6),
        Vec3::new(1.2, 0.7, 0.7),
        STONE_DARK,
    );
    b.cuboid_c(
        Vec3::new(0.0, 0.95, -0.6),
        Vec3::new(1.4, 0.12, 0.85),
        STONE,
    );
    b.cuboid_c(
        Vec3::new(0.0, 1.1, -0.6),
        Vec3::new(0.34, 0.2, 0.34),
        [0.78, 0.62, 0.22, 1.0],
    );
    for (x, z) in [(-1.9, -1.9), (1.9, -1.9), (-1.9, 1.9), (1.9, 1.9)] {
        b.cuboid_c(Vec3::new(x, 1.6, z), Vec3::new(0.34, 2.8, 0.34), STONE);
        colliders.push((Vec3::new(x, 1.6, z), Vec3::new(0.4, 2.8, 0.4)));
    }
    b.cuboid_c(
        Vec3::new(0.0, 3.08, 0.0),
        Vec3::new(4.6, 0.2, 4.6),
        STONE_DARK,
    );
    b.pyramid(
        Vec3::new(0.0, 3.18, 0.0),
        2.5,
        1.6,
        crate::buildings::ROOFS[1],
    );
    colliders
}

/// Pale del mulino attorno all'origine, nel piano XY (girano attorno a Z).
pub fn sails() -> Vec<(Mat, Mesh)> {
    use std::f32::consts::TAU;
    const LEN: f32 = 2.9;
    let mut b = B::default();
    let cloth: Col = [0.84, 0.8, 0.68, 1.0];
    for k in 0..4 {
        let a = k as f32 / 4.0 * TAU;
        let dir = Vec3::new(a.cos(), a.sin(), 0.0);
        let side = Vec3::new(-dir.y, dir.x, 0.0);
        b.beam(Vec3::ZERO, dir * LEN, 0.05, TIMBER);
        for s in [0.75f32, 1.5, 2.25, 3.0] {
            b.beam(dir * s, dir * s + side * 0.72, 0.02, TIMBER);
        }
        b.beam(
            dir * 0.75 + side * 0.72,
            dir * LEN + side * 0.72,
            0.02,
            TIMBER,
        );
        // Tinta unita: con la trama del tessuto le pale da lontano fanno l'effetto a quadri.
        b.force = Some(Mat::Plain);
        let z = Vec3::Z * 0.01;
        b.quad(
            dir * 0.75 + z,
            dir * LEN + z,
            dir * LEN + side * 0.7 + z,
            dir * 0.75 + side * 0.7 + z,
            Vec3::Z,
            cloth,
        );
        b.force = None;
    }
    b.cuboid_c(Vec3::ZERO, Vec3::new(0.32, 0.32, 0.5), TIMBER);
    b.finish_parts()
}

/// È uno dei luoghi fatti da questo modulo (senza la casa del mulino)?
pub fn is_place(kind: &str) -> bool {
    matches!(
        kind,
        "campo-ladro" | "banditi" | "rovine" | "torre-rovina" | "grotta" | "santuario"
    )
}

/// Costruisce il luogo: larghezza, profondità e collider come in `buildings::model`.
pub fn build(kind: &str, b: &mut B, int: &mut Interior) -> (f32, f32, Vec<(Vec3, Vec3)>) {
    match kind {
        "campo-ladro" => (5.0, 5.0, thief_camp(b, int)),
        "banditi" => (12.0, 11.0, bandit_camp(b, int)),
        "rovine" => (12.0, 10.0, ruins(b, int)),
        "torre-rovina" => {
            ruined_tower(b, int);
            (4.6, 4.6, Vec::new())
        }
        "grotta" => {
            cave(b, int);
            (CAVE_HALF.x * 2.0, CAVE_HALF.y * 2.0, Vec::new())
        }
        _ => (4.8, 4.8, shrine(b, int)),
    }
}

/// Dove camminano i banditi di un campo con centro `c`.
pub fn bandit_routes(c: Vec2) -> Vec<(Vec2, Vec2)> {
    (0..BANDITS)
        .map(|i| {
            let a = i as f32 * 2.1 + 0.4;
            let dir = Vec2::from_angle(a) * BANDIT_WALK;
            (c + dir, c + dir.perp() * 0.7 + Vec2::new(0.0, 1.5))
        })
        .collect()
}

/// Il percorso che parte da `first` appartiene a un campo di banditi?
pub fn is_camp_patrol(dbg: &DebugPlace, first: Vec2) -> bool {
    dbg.items
        .iter()
        .filter(|p| p.kind == "banditi")
        .flat_map(|p| p.points.iter())
        .any(|c| {
            bandit_routes(*c)
                .iter()
                .any(|(a, _)| a.distance(first) < 1.5)
        })
}

/// Notte di gioco di una data: cambia a mezzogiorno, così tutta la notte conta come una.
pub fn night_index(clock: &crate::daynight::GameClock) -> i64 {
    let day = i64::from(clock.date.year) * 400 + i64::from(clock.date.day_of_year());
    if clock.hour < 12.0 { day - 1 } else { day }
}

/// Una data notte i banditi tornano? Casuale, ma uguale per tutti nella stessa notte.
pub fn bandits_return(center: Vec2, night: i64) -> bool {
    crate::world_props::hash01(center, night as f32 * 1.37 + 5.0) < 0.5
}

/// Sistema: i banditi uccisi non tornano subito: una notte a caso, con la stessa probabilità ogni
/// notte, riappaiono dove mancano. All'avvio dell'app il campo è già popolato.
#[allow(clippy::too_many_arguments)]
/// Marcatore: i campi dei banditi di questa scena sono già stati popolati.
#[derive(Component)]
struct BanditsSeeded;

#[allow(clippy::too_many_arguments)]
fn keep_bandits(
    mut commands: Commands,
    server: Res<AssetServer>,
    time: Res<Time>,
    clock: Res<crate::daynight::GameClock>,
    dbg: Res<DebugPlace>,
    field: Option<Res<TerrainField>>,
    enemies: Query<&PatrolState, With<Enemy>>,
    seeded: Query<(), With<BanditsSeeded>>,
    missions: Res<crate::quests::missions::Missions>,
    mut timer: Local<f32>,
    mut last_night: Local<Option<i64>>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 2.0;
    let Some(field) = field else {
        return;
    };
    let night_hours = clock.hour >= 21.5 || clock.hour < 4.5;
    // Popolamento iniziale una volta per scena (il marcatore sparisce col reset della scena):
    // prima valeva «i primi 40 s», e chi uccideva un bandito subito lo vedeva ricomparire.
    let start = seeded.is_empty();
    let night = night_index(&clock);
    if !start && (!night_hours || *last_night == Some(night)) {
        return;
    }
    let camps = dbg
        .items
        .iter()
        .filter(|p| p.kind == "banditi")
        .flat_map(|p| p.points.iter().copied());
    let mut spawned = false;
    // Dopo il Rito del Nome di Don Anselmo (qs04) i banditi trovano pace e non tornano più.
    let at_peace = missions.unlocked("rito-banditi");
    for center in camps {
        if at_peace || (!start && !bandits_return(center, night)) {
            continue;
        }
        for (a, b) in bandit_routes(center) {
            let known = |w: &[Vec3]| w.first().is_some_and(|p| p.xz().distance(a) < 1.0);
            if enemies.iter().any(|e| known(&e.waypoints)) {
                continue;
            }
            let at = |p: Vec2| Vec3::new(p.x, field.height_at(p.x, p.y) + 1.0, p.y);
            crate::entities::spawn_enemy(&mut commands, &server, at(a), at(b));
            spawned = true;
            info!(
                "[LUOGHI] bandito al campo ({:.0},{:.0})",
                center.x, center.y
            );
        }
    }
    if spawned && !start {
        *last_night = Some(night);
    }
    if start {
        commands.spawn((BanditsSeeded, crate::scene::SceneNode));
    }
}

/// Distanza (m) a cui un luogo si considera scoperto.
const DISCOVER_RANGE: f32 = 16.0;

/// Luoghi scoperti dal giocatore (ADR 0037): chiave «regione|nome» e il punto in cui stava
/// quando li ha scoperti, che è dove arriva il viaggio veloce (di sicuro si può stare lì).
#[derive(Resource, Default)]
pub struct Known(pub std::collections::BTreeMap<String, Vec3>);

impl Known {
    fn key(name: &str) -> String {
        format!("{}|{name}", crate::region::current().key())
    }

    /// Punto d'arrivo del luogo `name` nella regione corrente, se è stato scoperto.
    pub fn arrival(&self, name: &str) -> Option<Vec3> {
        self.0.get(&Self::key(name)).copied()
    }

    /// Un abitante ti ha parlato di questo luogo: sulla mappa compare con un «?» (ADR 0046).
    /// Le voci stanno nella stessa tabella, con la chiave «?regione|nome».
    pub fn rumored(&self, name: &str) -> bool {
        self.0.contains_key(&format!("?{}", Self::key(name)))
    }

    /// Annota un luogo sentito nominare; `false` se era già noto o già annotato.
    pub fn rumor(&mut self, name: &str) -> bool {
        if self.arrival(name).is_some() || self.rumored(name) {
            return false;
        }
        self.0.insert(format!("?{}", Self::key(name)), Vec3::ZERO);
        true
    }

    /// Il luogo scoperto della regione corrente il cui nome contiene `wanted` (minuscolo).
    pub fn find(&self, wanted: &str) -> Option<(String, Vec3)> {
        let prefix = format!("{}|", crate::region::current().key());
        self.0
            .iter()
            .filter_map(|(k, v)| Some((k.strip_prefix(&prefix)?.to_string(), *v)))
            .find(|(name, _)| name.to_lowercase().contains(wanted))
    }
}

/// Richiesta di viaggio veloce verso un luogo scoperto (mappa o shell): nome e punto d'arrivo.
#[derive(Resource, Default)]
pub struct FastTravel(pub Option<(String, Vec3)>);

/// I luoghi della regione corrente che si possono scoprire: quelli di `isola.txt` e il paese.
pub fn all_places(village: &crate::village::VillageInfo) -> Vec<(String, Vec2)> {
    let mut places = crate::water::load_spec_cached().places.clone();
    if village.poly.len() >= 3 {
        places.push((village.spec.name.clone(), village.spec.plaza));
    }
    places
}

/// Chi parla di quale luogo: alla prima chiacchierata l'abitante lo annota sulla tua mappa.
pub const RUMORS: [(&str, &str); 9] = [
    ("Viandante", "Campo del ladro"),
    ("Gino", "Torre antica"),
    ("Rosa", "Torre antica"),
    ("Ida", "Santuario"),
    ("Silvia", "Campo dei banditi"),
    ("Nina", "Grotta"),
    ("Ferruccio", "Rovine antiche"),
    ("Pietro", "Mulino"),
    ("Beppe", "Fattoria"),
];

/// Stato di esplorazione di un luogo scoperto: mobili aperti, mobili in tutto, nemici vivi.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Explored {
    pub opened: u32,
    pub furniture: u32,
    pub enemies: u32,
    /// Eventi del luogo non ancora vissuti (la scintilla sulla torre, l'alba sui gradoni…).
    pub events: u32,
}

impl Explored {
    /// Esplorato del tutto: niente nemici, ogni mobile aperto e ogni evento del luogo vissuto.
    pub fn complete(self) -> bool {
        self.enemies == 0 && self.opened >= self.furniture && self.events == 0
    }

    pub fn label(self) -> &'static str {
        if self.complete() {
            "esplorato"
        } else {
            "in parte"
        }
    }
}

/// Stato di esplorazione dei luoghi scoperti della regione (ricalcolato, non salvato).
#[derive(Resource, Default, Debug)]
pub struct Exploration(pub std::collections::BTreeMap<String, Explored>);

/// Raggio (m) entro cui mobili e nemici appartengono a un luogo.
const PLACE_REACH: f32 = 18.0;

/// Sistema: un abitante nomina un luogo che non conosci ancora e lo annota sulla mappa.
pub fn hear_rumors(
    dialog: Res<crate::entities::DialogState>,
    mut known: ResMut<Known>,
    mut session: ResMut<crate::save::Session>,
    mut was_open: Local<bool>,
) {
    let opened = dialog.open && !*was_open;
    *was_open = dialog.open;
    if !opened || crate::region::current() != crate::region::Region::Isola {
        return;
    }
    for (who, place) in RUMORS {
        if who == dialog.speaker && known.rumor(place) {
            info!("[LUOGHI] {who} parla di: {place}");
            session.notify(format!("Nuovo luogo annotato: {place}"));
        }
    }
}

/// Sistema: quanto è esplorato ogni luogo scoperto (mobili aperti e nemici rimasti).
#[allow(clippy::too_many_arguments)]
pub fn update_exploration(
    time: Res<Time>,
    mut timer: Local<f32>,
    village: Res<crate::village::VillageInfo>,
    known: Res<Known>,
    containers: Res<crate::containers::Containers>,
    furniture: Query<(&GlobalTransform, &crate::containers::Container)>,
    enemies: Query<(&GlobalTransform, &crate::combat::EnemyHealth), With<Enemy>>,
    missions: Res<crate::quests::missions::Missions>,
    mut exploration: ResMut<Exploration>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 1.0;
    exploration.0.clear();
    for (name, at) in all_places(&village) {
        if known.arrival(&name).is_none() || name == village.spec.name {
            continue;
        }
        let near = |t: &GlobalTransform| t.translation().xz().distance(at) <= PLACE_REACH;
        let mut e = Explored::default();
        for (t, c) in &furniture {
            if near(t) {
                e.furniture += 1;
                e.opened += u32::from(containers.was_opened(&c.key));
            }
        }
        e.enemies = enemies
            .iter()
            .filter(|(t, hp)| hp.current > 0.0 && near(t))
            .count() as u32;
        if missions.enabled {
            e.events = missions.events_near(at, PLACE_REACH) as u32;
        }
        exploration.0.insert(name, e);
    }
}

/// Sistema: avvicinandosi a un luogo lo si scopre (nome sulla mappa e viaggio veloce).
pub fn discover_places(
    time: Res<Time>,
    mut timer: Local<f32>,
    village: Res<crate::village::VillageInfo>,
    mut known: ResMut<Known>,
    mut session: ResMut<crate::save::Session>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 0.5;
    let Ok(player) = players.single() else {
        return;
    };
    let p = player.translation();
    for (name, at) in all_places(&village) {
        let key = Known::key(&name);
        if p.xz().distance(at) <= DISCOVER_RANGE && !known.0.contains_key(&key) {
            known.0.remove(&format!("?{key}"));
            known.0.insert(key, p);
            info!("[LUOGHI] scoperto: {name}");
            crate::skills::train(crate::skills::Skill::Lore, 25.0);
            crate::karma::praise("esplorazione", 2);
            // Nelle prove automatiche (nessuna partita attiva) niente avvisi a schermo.
            if session.active {
                session.notify(format!("Luogo scoperto: {name}"));
            }
        }
    }
}

/// Sistema: esegue il viaggio veloce. Non si parte con un nemico alle calcagna.
#[allow(clippy::type_complexity)]
pub fn fast_travel(
    mut travel: ResMut<FastTravel>,
    mut session: ResMut<crate::save::Session>,
    state: Res<State<crate::AppState>>,
    mut next: ResMut<NextState<crate::AppState>>,
    minds: Query<(&GlobalTransform, &crate::entities::EnemyMind), With<Enemy>>,
    mut players: Query<
        (&mut Transform, &mut avian3d::prelude::LinearVelocity),
        With<crate::player::Player>,
    >,
) {
    let Some((name, to)) = travel.0.take() else {
        return;
    };
    let Ok((mut t, mut v)) = players.single_mut() else {
        return;
    };
    let here = t.translation;
    if minds
        .iter()
        .any(|(g, m)| m.chase && g.translation().distance(here) < 40.0)
    {
        session.notify("Non puoi viaggiare con un nemico alle calcagna");
        info!("[LUOGHI] viaggio veloce rifiutato: nemico vicino");
        return;
    }
    t.translation = to + Vec3::Y * 0.3;
    v.0 = Vec3::ZERO;
    if *state.get() == crate::AppState::Map {
        next.set(crate::AppState::Playing);
    }
    info!("[LUOGHI] viaggio veloce: {name}");
    session.notify(name);
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            (sunken_door, sunken_glow).chain(),
            well_grate,
            fill_cave_hill,
        ),
    );
    app.add_systems(
        Update,
        (
            resonance_basin.after(crate::entities::interact),
            resonance_pulse,
        )
            .run_if(in_state(crate::AppState::Playing)),
    );
    app.init_resource::<Known>()
        .init_resource::<FastTravel>()
        .init_resource::<Exploration>()
        .add_systems(Update, (hear_rumors, update_exploration))
        .add_systems(
            Update,
            (
                keep_bandits.run_if(in_state(crate::AppState::Playing)),
                discover_places.run_if(in_state(crate::AppState::Playing)),
                fast_travel,
            ),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hill_over_the_galleries_is_filled_without_entering_them() {
        // Collina come quella misurata sopra le gallerie: 6-8,5 m nel mondo, 4-6,5 m dalla base.
        let columns = cave_fill_columns(|_, z| if z < -12.0 { 6.0 } else { 0.9 });
        assert!(!columns.is_empty());
        let mut b = B::default();
        let mut int = Interior::default();
        cave(&mut b, &mut int);
        // Nessuna colonna tocca l'aria delle gallerie: un punto a mezza altezza di ogni stanza
        // (pavimento -1,2) resta fuori da tutte.
        for (x0, x1, z0, z1, top) in CAVE_GALLERIES {
            let room_air = Vec3::new((x0 + x1) / 2.0, (-1.2 + top - 0.6) / 2.0, (z0 + z1) / 2.0);
            for (min, max) in &columns {
                let inside = room_air.cmpgt(*min).all() && room_air.cmplt(*max).all();
                assert!(
                    !inside,
                    "colonna {min} {max} dentro la galleria a {room_air}"
                );
            }
        }
        // Sopra le gallerie la roccia arriva al terreno: niente spazio in cui stare in piedi.
        let over_descent = Vec3::new(-2.5, 5.0, -15.0);
        assert!(
            columns
                .iter()
                .any(|(min, max)| over_descent.cmpgt(*min).all() && over_descent.cmplt(*max).all()),
            "sopra la discesa resta un vuoto"
        );
    }

    #[test]
    fn no_rock_where_the_ground_is_below_the_roofs() {
        assert!(cave_fill_columns(|_, _| -2.0).is_empty());
    }

    fn top_of(int: &Interior) -> f32 {
        int.ground
            .iter()
            .map(|(c, s)| c.y + s.y / 2.0)
            .fold(f32::MIN, f32::max)
    }

    #[test]
    fn tower_can_be_climbed_to_the_top() {
        let mut b = B::default();
        let mut int = Interior::default();
        let top = ruined_tower(&mut b, &mut int);
        assert!((top_of(&int) - top).abs() < 0.05);
        assert_eq!(int.containers.len(), 1);
    }

    #[test]
    fn every_place_has_something_to_find_or_a_fire() {
        for kind in ["campo-ladro", "banditi", "rovine", "grotta"] {
            let mut b = B::default();
            let mut int = Interior::default();
            build(kind, &mut b, &mut int);
            assert!(!int.containers.is_empty(), "{kind}");
        }
    }

    #[test]
    fn bandits_keep_their_own_route() {
        let r = bandit_routes(Vec2::ZERO);
        assert_eq!(r.len(), BANDITS);
        assert!(r[0].0.distance(r[1].0) > 3.0);
    }
}
