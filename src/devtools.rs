//! Variabili d'ambiente di debug per provare il gioco senza giocarlo.
//!
//! - `PLANET_ONE_HOUR=21.5`: ora di gioco iniziale (vedi `daynight`).
//! - `PLANET_ONE_DATE=anno,mese,giorno`: data di gioco iniziale (es. `0,12,21`, solstizio d'inverno).
//! - `PLANET_ONE_WEATHER=sereno|nuvoloso|pioggia|temporale|nebbia|neve`: forza il meteo (anche in sviluppo).
//! - `PLANET_ONE_POS=x,y,z`: teletrasporta il player (dopo ~1 s).
//! - `PLANET_ONE_GIVE=1`: tutti gli oggetti in inventario (armi, scudo, frecce, 200 monete).
//! - `PLANET_ONE_DELAY_SCALE=0.1`: moltiplica i ritardi con cui il Viandante chiude porta/finestre.
//! - `PLANET_ONE_LOOK=yaw,pitch[,distanza[,1]]`: orientamento della camera in gradi
//!   (il quarto valore 1 attiva la prima persona).
//!
//! Se le variabili non sono impostate non succede nulla.

use crate::camera::OrbitCamera;
use crate::player::Player;
use bevy::prelude::*;

/// Sistema (Update): applica una sola volta posizione e orientamento di debug.
pub fn apply_debug_env(
    mut orbit: ResMut<OrbitCamera>,
    mut players: Query<&mut Transform, With<Player>>,
    mut inventory: ResMut<crate::inventory::Inventory>,
    mut frames: Local<u32>,
) {
    *frames += 1;
    if *frames != 60 {
        return;
    }
    // PLANET_ONE_GIVE=1: tutti gli oggetti in inventario (per provare armi e negozi).
    if std::env::var("PLANET_ONE_GIVE").is_ok_and(|v| v != "0") {
        use crate::inventory::ItemKind::*;
        for k in [
            ShortSword,
            LongSword,
            Axe,
            Shield,
            Bow,
            HealthPotion,
            Apple,
            Fish,
        ] {
            inventory.add(k, 1);
        }
        inventory.add(Arrow, 20);
        inventory.add(Coin, 200);
    }
    let parse = |name: &str| -> Option<Vec<f32>> {
        let v = std::env::var(name).ok()?;
        Some(v.split(',').filter_map(|x| x.trim().parse().ok()).collect())
    };
    if let Some(p) = parse("PLANET_ONE_LOOK")
        && p.len() >= 2
    {
        orbit.yaw = p[0].to_radians();
        orbit.pitch = p[1].to_radians();
        if let Some(d) = p.get(2) {
            orbit.distance = *d;
        }
        if p.get(3) == Some(&1.0) {
            orbit.first_person_lock = true;
            orbit.mode = crate::camera::CameraMode::FirstPerson;
        }
    }
    if let Some(p) = parse("PLANET_ONE_POS")
        && p.len() == 3
        && let Ok(mut t) = players.single_mut()
    {
        t.translation = Vec3::new(p[0], p[1], p[2]);
    }
}
