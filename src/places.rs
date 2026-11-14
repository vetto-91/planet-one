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
        (v(-6.6, CEIL, -24.6), v(1.0, CEIL + 0.6, -17.8)),
    ] {
        rock(b, int, min, max);
    }
    campfire(b, int, v(-3.0, FL, -20.2));
    rooms::chest(b, int, -4.8, -23.2, FL);
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
        (v(4.0, LO, -27.6), v(6.6, LOW, -27.0)),
        (v(0.4, LOW, -27.6), v(6.6, LOW + 0.6, -19.1)),
    ] {
        rock(b, int, min, max);
    }
    rooms::chest(b, int, 5.3, -26.2, FL);
    campfire(b, int, v(5.3, FL, -23.2));
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
            b.cuboid_c(
                Vec3::new(x, base - 0.1 + half.y, z),
                half * 2.0,
                if k.is_multiple_of(3) {
                    STONE
                } else {
                    STONE_DARK
                },
            );
        }
    }
    b.pyramid(Vec3::new(0.0, H + 2.2, -3.0), 5.4, 2.6, STONE_DARK);
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
            known.0.insert(key, p);
            info!("[LUOGHI] scoperto: {name}");
            crate::skills::train(crate::skills::Skill::Lore, 25.0);
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
    app.init_resource::<Known>()
        .init_resource::<FastTravel>()
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
