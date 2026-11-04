//! Cielo procedurale: cubemap generata a runtime con un gradiente.
//!
//! Il cielo non usa asset esterni (nessuna HDRI da scaricare): costruiamo
//! direttamente una texture cubemap con sei facce, colorata in base alla
//! direzione del raggio, per ottenere un cielo stile cel-shading senza
//! dipendenze aggiuntive né asset binari.
//!
//! Nota sulla geometria: una cubemap è composta da 6 facce quadrate; qui le
//! generiamo con `depth_or_array_layers = 6` e marchiamo la texture come
//! `TextureViewDimension::Cube`, il formato richiesto da `Skybox`.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

/// Larghezza/altezza di ogni faccia della cubemap (128 px è sufficiente per
/// un gradiente e mantiene il costo in memoria trascurabile).
const FACE_SIZE: u32 = 128;

/// Costruisce una cubemap RGB con gradiente orizzonte/zenit calcolato per
/// pixel a partire dalla direzione del raggio, senza giunture visibili.
pub fn gradient_skybox() -> Image {
    let layers = 6;
    let bytes_per_pixel = 4;

    // 6 facce di FACE_SIZE x FACE_SIZE pixel, RGBA 8 bit.
    let mut data = Vec::with_capacity((FACE_SIZE * FACE_SIZE * layers * bytes_per_pixel) as usize);

    // Direzioni delle sei facce: +X, -X, +Y, -Y, +Z, -Z (ordine standard
    // cubemap di wgpu/Bevy).
    let face_dirs: [(f32, f32, f32); 6] = [
        (1.0, 0.0, 0.0),
        (-1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0),
        (0.0, -1.0, 0.0),
        (0.0, 0.0, 1.0),
        (0.0, 0.0, -1.0),
    ];

    // Vettori “destra” e “su” per ogni faccia, usati per interpolare la
    // direzione del raggio dentro la faccia.
    let face_rights: [(f32, f32, f32); 6] = [
        (0.0, 0.0, -1.0),
        (0.0, 0.0, 1.0),
        (1.0, 0.0, 0.0),
        (1.0, 0.0, 0.0),
        (1.0, 0.0, 0.0),
        (-1.0, 0.0, 0.0),
    ];
    let face_ups: [(f32, f32, f32); 6] = [
        (0.0, 1.0, 0.0),
        (0.0, 1.0, 0.0),
        (0.0, 0.0, -1.0),
        (0.0, 0.0, 1.0),
        (0.0, 1.0, 0.0),
        (0.0, 1.0, 0.0),
    ];

    // Colori base del cielo: zenit azzurro profondo, orizzonte caldo chiaro.
    let zenith = (46, 120, 190);
    let horizon = (232, 236, 242);
    let ground = (60, 70, 78);

    // Per ogni faccia, per ogni pixel, calcoliamo la direzione del raggio
    // (in spazio mondo) e campioniamo un gradiente in base a `dir.y`.
    for face in 0..layers {
        let (fx, fy, fz) = face_dirs[face as usize];
        let (rx, ry, rz) = face_rights[face as usize];
        let (ux, uy, uz) = face_ups[face as usize];

        for y in 0..FACE_SIZE {
            // `v` va da -1 a 1 lungo l'asse “up” della faccia.
            let v = (y as f32 / (FACE_SIZE - 1) as f32) * 2.0 - 1.0;
            for x in 0..FACE_SIZE {
                // `u` va da -1 a 1 lungo l'asse “right” della faccia.
                let u = (x as f32 / (FACE_SIZE - 1) as f32) * 2.0 - 1.0;

                // Direzione approssimata: centro della faccia + offset.
                let dx = fx + rx * u + ux * v;
                let dy = fy + ry * u + uy * v;
                let dz = fz + rz * u + uz * v;
                let len = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-6);
                let dir_y = dy / len;

                // Gradiente: orizzonte caldo, zenit azzurro, sotto orizzonte
                // un terreno scuro per evitare riflessi innaturali.
                let (r, g, b) = if dir_y > 0.0 {
                    let t = dir_y.min(1.0);
                    let t2 = t * t;
                    (
                        horizon.0 as f32 * (1.0 - t2) + zenith.0 as f32 * t2,
                        horizon.1 as f32 * (1.0 - t2) + zenith.1 as f32 * t2,
                        horizon.2 as f32 * (1.0 - t2) + zenith.2 as f32 * t2,
                    )
                } else {
                    let t = (-dir_y).min(1.0);
                    (
                        horizon.0 as f32 * (1.0 - t) + ground.0 as f32 * t,
                        horizon.1 as f32 * (1.0 - t) + ground.1 as f32 * t,
                        horizon.2 as f32 * (1.0 - t) + ground.2 as f32 * t,
                    )
                };

                data.push(r as u8);
                data.push(g as u8);
                data.push(b as u8);
                data.push(255);
            }
        }
    }

    let size = Extent3d {
        width: FACE_SIZE,
        height: FACE_SIZE,
        depth_or_array_layers: layers,
    };

    let mut image = Image::new(
        size,
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    // Una cubemap richiede una vista `Cube`; senza questa marcatura il
    // render pipeline ignorerebbe la texture.
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..Default::default()
    });

    image
}
