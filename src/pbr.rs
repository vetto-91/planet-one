//! Materiali PBR condivisi (ADR 0016): texture CC0 Poly Haven in `assets/textures/pbr/`.
//!
//! Ogni materiale ha colore, normal (convenzione OpenGL) e una mappa ARM (occlusione, ruvidità, metallo).
//! Le UV degli edifici sono in metri: `scale_m` dice quanti metri copre una ripetizione della texture.

use bevy::asset::RenderAssetUsages;
use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy::math::Affine2;
use bevy::prelude::*;
use std::collections::HashMap;

/// Materiale di una parte di edificio.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Mat {
    /// Solo colori di vertice (tende, dettagli, ecc.).
    Plain,
    Plaster,
    Planks,
    Stone,
    Tiles,
    Slate,
    Thatch,
    Glass,
    Cloth,
    Iron,
}

/// Tutti i materiali, nell'ordine in cui si generano le parti.
pub const MATS: [Mat; 10] = [
    Mat::Plain,
    Mat::Plaster,
    Mat::Planks,
    Mat::Stone,
    Mat::Tiles,
    Mat::Slate,
    Mat::Thatch,
    Mat::Glass,
    Mat::Cloth,
    Mat::Iron,
];

/// Materiali condivisi dagli edifici e dalle mura.
#[derive(Resource)]
pub struct PbrMaterials(HashMap<Mat, Handle<StandardMaterial>>);

impl PbrMaterials {
    pub fn get(&self, m: Mat) -> Handle<StandardMaterial> {
        self.0[&m].clone()
    }
}

/// Texture dei materiali a metà risoluzione (`assets/textures/pbr/half/`, ADR 0057).
static REDUCED_TEXTURES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Sceglie le texture ridotte: vale per quelle caricate da qui in poi (all'avvio).
pub fn set_reduced_textures(on: bool) {
    REDUCED_TEXTURES.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Percorso della texture: con le texture ridotte, la copia a metà risoluzione.
fn texture_path(path: String, reduced: bool) -> String {
    match path.strip_prefix("textures/pbr/") {
        Some(name) if reduced => format!("textures/pbr/half/{name}"),
        _ => path,
    }
}

/// Carica una texture ripetuta (`srgb` per il colore, lineare per normal e ARM).
pub fn load_tex(server: &AssetServer, path: String, srgb: bool) -> Handle<Image> {
    let path = texture_path(
        path,
        REDUCED_TEXTURES.load(std::sync::atomic::Ordering::Relaxed),
    );
    server
        .load_builder()
        .with_settings(move |s: &mut ImageLoaderSettings| {
            s.is_srgb = srgb;
            // Solo sulla GPU: nessuno rilegge queste texture, la copia in RAM era sprecata (ADR 0056).
            s.asset_usage = RenderAssetUsages::RENDER_WORLD;
            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                mipmap_filter: ImageFilterMode::Linear,
                anisotropy_clamp: 16,
                ..Default::default()
            });
        })
        .load(path)
}

/// Carica una texture con il campionatore predefinito, solo sulla GPU (ADR 0056).
pub fn load_gpu(server: &AssetServer, path: impl Into<String>) -> Handle<Image> {
    server
        .load_builder()
        .with_settings(|s: &mut ImageLoaderSettings| {
            s.asset_usage = RenderAssetUsages::RENDER_WORLD;
        })
        .load(path.into())
}

/// Mesh solo per la GPU: grandi mesh generate che nessuno rilegge (ADR 0056).
pub fn gpu_only(mut mesh: Mesh) -> Mesh {
    mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;
    mesh
}

/// Mesh di una primitiva con le tangenti, necessarie alle normal map.
pub fn with_tangents(mesh: impl Into<Mesh>) -> Mesh {
    let mut mesh = mesh.into();
    if mesh.generate_tangents().is_err() {
        warn!("[PBR] tangenti non generate per una primitiva");
    }
    mesh
}

/// Materiale PBR completo `name` (`pbr/<name>_diff.jpg`, `_nor.jpg`, `_arm.jpg`).
pub fn material(server: &AssetServer, name: &str, uv_scale: Vec2) -> StandardMaterial {
    let arm = load_tex(server, format!("textures/pbr/{name}_arm.jpg"), false);
    StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(load_tex(
            server,
            format!("textures/pbr/{name}_diff.jpg"),
            true,
        )),
        normal_map_texture: Some(load_tex(
            server,
            format!("textures/pbr/{name}_nor.jpg"),
            false,
        )),
        metallic_roughness_texture: Some(arm.clone()),
        occlusion_texture: Some(arm),
        perceptual_roughness: 1.0,
        metallic: 1.0,
        uv_transform: Affine2::from_scale(uv_scale),
        ..default()
    }
}

impl FromWorld for PbrMaterials {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>().clone();
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let mut map = HashMap::new();
        for (m, name, metres) in [
            (Mat::Plaster, "plaster", 2.0),
            (Mat::Planks, "planks", 1.8),
            (Mat::Stone, "stone", 2.5),
            (Mat::Tiles, "tiles", 2.4),
            (Mat::Slate, "slate", 3.0),
            (Mat::Thatch, "thatch", 2.2),
            (Mat::Cloth, "cloth", 1.3),
            (Mat::Iron, "iron", 1.0),
        ] {
            let scale = Vec2::splat(1.0 / metres);
            map.insert(m, materials.add(material(&server, name, scale)));
        }
        // Il ferro nasce ruggine chiara: scurito, resta ferro battuto.
        if let Some(iron) = map.get(&Mat::Iron).cloned()
            && let Some(mut m) = materials.get_mut(&iron)
        {
            m.base_color = Color::srgb(0.32, 0.31, 0.32);
        }
        map.insert(
            Mat::Plain,
            materials.add(StandardMaterial {
                perceptual_roughness: 0.9,
                ..default()
            }),
        );
        map.insert(
            Mat::Glass,
            materials.add(StandardMaterial {
                perceptual_roughness: 0.35,
                reflectance: 0.12,
                ..default()
            }),
        );
        Self(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduced_textures_come_from_the_half_folder() {
        let p = |s: &str, r| texture_path(s.to_string(), r);
        assert_eq!(
            p("textures/pbr/sand_diff.jpg", false),
            "textures/pbr/sand_diff.jpg"
        );
        assert_eq!(
            p("textures/pbr/sand_diff.jpg", true),
            "textures/pbr/half/sand_diff.jpg"
        );
        // Fuori dai materiali PBR non cambia nulla.
        assert_eq!(p("textures/capanna.png", true), "textures/capanna.png");
    }
}
