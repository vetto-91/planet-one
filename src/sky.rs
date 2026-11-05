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
use bevy::light::Skybox;
use bevy::prelude::*;
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
        // NOTA: Escolhemos Rgba8Unorm (lineare) Deliberadamente em vez de Srgb:
        // a cubemap é amostrada diretamente pelo shader do skybox SEM passar
        // pela conversão sRGB do tonemapping, então guardar os dados em sRGB
        // causaria um "double gamma" que deixa o céu acinzentado.
        // Com Unorm os valores 46..232 viram diretamente o gradiente azul.
        TextureFormat::Rgba8Unorm,
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

/// Handle della panoramica del cielo (PNG), caricata dall'asset server.
#[derive(Resource)]
pub struct SkyTexture(pub Handle<Image>);

/// Quante volte la panoramica si ripete attorno ai 360° (evita nuvole stirate).
const SKY_REPEAT: f32 = 2.0;
/// Lato delle facce della cubemap generata dalla panoramica.
const SKY_FACE_SIZE: u32 = 512;

/// Campiona bilinearmente (RGB, byte grezzi) la panoramica a `(u, v)` in 0..1;
/// `u` è periodico, `v` è bloccato ai bordi.
fn sample_bilinear(data: &[u8], w: u32, h: u32, bpp: usize, u: f32, v: f32) -> [f32; 3] {
    let x = u.rem_euclid(1.0) * w as f32 - 0.5;
    let y = (v.clamp(0.0, 1.0) * h as f32 - 0.5).clamp(0.0, (h - 1) as f32);
    let x0 = x.floor();
    let fx = x - x0;
    let y0 = y.floor() as u32;
    let y1 = (y0 + 1).min(h - 1);
    let fy = y - y0 as f32;
    let xi = |xx: f32| (xx.rem_euclid(w as f32)) as u32;
    let (xa, xb) = (xi(x0), xi(x0 + 1.0));
    let px = |xx: u32, yy: u32, c: usize| data[(yy * w + xx) as usize * bpp + c] as f32;
    let mut out = [0.0; 3];
    for (c, o) in out.iter_mut().enumerate() {
        let top = px(xa, y0, c) * (1.0 - fx) + px(xb, y0, c) * fx;
        let bot = px(xa, y1, c) * (1.0 - fx) + px(xb, y1, c) * fx;
        *o = top * (1.0 - fy) + bot * fy;
    }
    out
}

/// Converte la panoramica (zenit in alto, orizzonte in basso) in una cubemap.
/// Direzioni per faccia secondo la convenzione standard wgpu (v verso il basso).
/// Sotto l'orizzonte si ripete la riga più bassa. `None` se il formato non è RGB(A) 8 bit.
pub fn equirect_to_cubemap(src: &Image) -> Option<Image> {
    let data = src.data.as_ref()?;
    let (w, h) = (src.width(), src.height());
    let bpp = data.len() / (w * h) as usize;
    if bpp < 3 || w == 0 || h == 0 {
        return None;
    }
    let n = SKY_FACE_SIZE;
    let mut out = Vec::with_capacity((n * n * 6 * 4) as usize);
    for face in 0..6 {
        for y in 0..n {
            let v = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            for x in 0..n {
                let u = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
                let d = match face {
                    0 => Vec3::new(1.0, -v, -u),
                    1 => Vec3::new(-1.0, -v, u),
                    2 => Vec3::new(u, 1.0, v),
                    3 => Vec3::new(u, -1.0, -v),
                    4 => Vec3::new(u, -v, 1.0),
                    _ => Vec3::new(-u, -v, -1.0),
                }
                .normalize();
                let azimuth = d.x.atan2(-d.z) / std::f32::consts::TAU;
                let elevation = d.y.max(0.0).asin() / std::f32::consts::FRAC_PI_2;
                let c = sample_bilinear(data, w, h, bpp, azimuth * SKY_REPEAT, 1.0 - elevation);
                out.extend_from_slice(&[c[0] as u8, c[1] as u8, c[2] as u8, 255]);
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: n,
            height: n,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        out,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..Default::default()
    });
    Some(image)
}

/// Sistema: appena la panoramica è caricata la converte in cubemap e la
/// assegna allo `Skybox` della camera (il gradiente resta come segnaposto).
pub fn apply_sky_texture(
    sky: Option<Res<SkyTexture>>,
    mut images: ResMut<Assets<Image>>,
    mut skyboxes: Query<&mut Skybox>,
    mut done: Local<bool>,
) {
    let Some(sky) = sky else { return };
    if *done {
        return;
    }
    let Some(src) = images.get(&sky.0) else {
        return;
    };
    *done = true;
    let Some(cube) = equirect_to_cubemap(src) else {
        warn!("[SKY] formato della panoramica non supportato: resta il gradiente");
        return;
    };
    let handle = images.add(cube);
    for mut skybox in &mut skyboxes {
        skybox.image = Some(handle.clone());
    }
    info!("[SKY] cubemap generata dalla panoramica");
}
