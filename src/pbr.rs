//! Materiali PBR condivisi (ADR 0016): texture CC0 Poly Haven in `assets/textures/pbr/`.
//!
//! Ogni materiale ha colore, normal (convenzione OpenGL) e una mappa ARM (occlusione, ruvidità, metallo).
//! Le UV degli edifici sono in metri: `scale_m` dice quanti metri copre una ripetizione della texture.

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
}

/// Tutti i materiali, nell'ordine in cui si generano le parti.
pub const MATS: [Mat; 8] = [
    Mat::Plain,
    Mat::Plaster,
    Mat::Planks,
    Mat::Stone,
    Mat::Tiles,
    Mat::Slate,
    Mat::Thatch,
    Mat::Glass,
];

/// Materiali condivisi dagli edifici e dalle mura.
#[derive(Resource)]
pub struct PbrMaterials(HashMap<Mat, Handle<StandardMaterial>>);

impl PbrMaterials {
    pub fn get(&self, m: Mat) -> Handle<StandardMaterial> {
        self.0[&m].clone()
    }
}

/// Carica una texture ripetuta (`srgb` per il colore, lineare per normal e ARM).
pub fn load_tex(server: &AssetServer, path: String, srgb: bool) -> Handle<Image> {
    server
        .load_builder()
        .with_settings(move |s: &mut ImageLoaderSettings| {
            s.is_srgb = srgb;
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
        ] {
            let scale = Vec2::splat(1.0 / metres);
            map.insert(m, materials.add(material(&server, name, scale)));
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
