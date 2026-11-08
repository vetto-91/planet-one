//! Personaggi a parallelepipedo con texture a "sviluppo" (net).
//!
//! Le skin (`assets/textures/{player,ladro,viandante}.png`) sono sviluppi di un
//! box: colonna 1 = fronte (viso in alto), colonna 0 = lato +X, colonna 2 =
//! lato -X, colonna 3 = retro, riquadro sopra la colonna 1 = cima. Il Cuboid di
//! Bevy mappa tutta la texture su ogni faccia, quindi qui si costruisce un mesh
//! con un rettangolo UV per faccia. Il fronte guarda verso -Z (come `facing`).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Dimensioni (px) delle skin ritagliate: tutte uguali.
const TEX_W: f32 = 945.0;
const TEX_H: f32 = 910.0;

/// Rettangolo UV in pixel della texture: (x0, y0, x1, y1).
type PxRect = (f32, f32, f32, f32);

/// Rettangoli UV delle sei facce.
pub struct FigureUv {
    pub front: PxRect,
    pub back: PxRect,
    pub left: PxRect,
    pub right: PxRect,
    pub top: PxRect,
    pub bottom: PxRect,
}

impl FigureUv {
    /// Layout standard (bordi di griglia esclusi con un'inset di ~6 px).
    pub fn standard() -> Self {
        Self {
            front: (247.0, 348.0, 466.0, 888.0),
            back: (706.0, 348.0, 922.0, 888.0),
            left: (19.5, 348.0, 235.0, 888.0),
            right: (478.0, 348.0, 694.0, 888.0),
            top: (247.0, 30.0, 466.0, 236.0),
            bottom: (247.0, 866.0, 466.0, 886.0),
        }
    }

    /// Come `standard`, ma i lati della skin del player mostrano le braccia
    /// aperte su fondo grigio: si usa un'area di stoffa uniforme.
    pub fn solid_sides() -> Self {
        Self {
            left: (40.0, 462.0, 200.0, 520.0),
            right: (500.0, 462.0, 660.0, 520.0),
            ..Self::standard()
        }
    }
}

/// Mesh di un box `size` centrato in origine, con UV per faccia.
pub fn figure_mesh(size: Vec3, uv: &FigureUv) -> Mesh {
    let (hw, hh, hd) = (size.x / 2.0, size.y / 2.0, size.z / 2.0);
    // (angoli tl, tr, br, bl, normale, rettangolo UV)
    let faces: [([Vec3; 4], Vec3, PxRect); 6] = [
        // Fronte (-Z): a sinistra dello spettatore c'è +X.
        (
            [
                Vec3::new(hw, hh, -hd),
                Vec3::new(-hw, hh, -hd),
                Vec3::new(-hw, -hh, -hd),
                Vec3::new(hw, -hh, -hd),
            ],
            Vec3::NEG_Z,
            uv.front,
        ),
        // Retro (+Z).
        (
            [
                Vec3::new(-hw, hh, hd),
                Vec3::new(hw, hh, hd),
                Vec3::new(hw, -hh, hd),
                Vec3::new(-hw, -hh, hd),
            ],
            Vec3::Z,
            uv.back,
        ),
        // Lato +X (colonna 0 della skin).
        (
            [
                Vec3::new(hw, hh, hd),
                Vec3::new(hw, hh, -hd),
                Vec3::new(hw, -hh, -hd),
                Vec3::new(hw, -hh, hd),
            ],
            Vec3::X,
            uv.left,
        ),
        // Lato -X (colonna 2).
        (
            [
                Vec3::new(-hw, hh, -hd),
                Vec3::new(-hw, hh, hd),
                Vec3::new(-hw, -hh, hd),
                Vec3::new(-hw, -hh, -hd),
            ],
            Vec3::NEG_X,
            uv.right,
        ),
        // Cima (+Y): il basso dell'immagine è verso il fronte.
        (
            [
                Vec3::new(hw, hh, hd),
                Vec3::new(-hw, hh, hd),
                Vec3::new(-hw, hh, -hd),
                Vec3::new(hw, hh, -hd),
            ],
            Vec3::Y,
            uv.top,
        ),
        // Base (-Y).
        (
            [
                Vec3::new(hw, -hh, -hd),
                Vec3::new(-hw, -hh, -hd),
                Vec3::new(-hw, -hh, hd),
                Vec3::new(hw, -hh, hd),
            ],
            Vec3::NEG_Y,
            uv.bottom,
        ),
    ];

    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut uvs = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (corners, normal, (x0, y0, x1, y1)) in faces {
        let base = positions.len() as u32;
        let (u0, u1, v0, v1) = (x0 / TEX_W, x1 / TEX_W, y0 / TEX_H, y1 / TEX_H);
        let corner_uv = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
        for (c, t) in corners.iter().zip(corner_uv) {
            positions.push(c.to_array());
            normals.push(normal.to_array());
            uvs.push(t);
        }
        indices.extend_from_slice(&[base, base + 3, base + 2, base, base + 2, base + 1]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Materiale con la skin (texture bianca neutra, ruvido).
pub fn skin_material(texture: Handle<Image>) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(texture),
        perceptual_roughness: 0.9,
        ..default()
    }
}

/// Mesh e materiale condivisi delle mani (cubetti color pelle).
#[derive(Clone)]
pub struct HandAssets {
    mesh: Handle<Mesh>,
    mat: Handle<StandardMaterial>,
}

impl HandAssets {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            mesh: meshes.add(Cuboid::new(0.18, 0.18, 0.18)),
            mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.88, 0.7, 0.56),
                perceptual_roughness: 0.9,
                ..default()
            }),
        }
    }

    /// Spawna due mani ai lati del corpo di dimensioni `size` (fronte -Z, destra +X).
    pub fn spawn(&self, parent: &mut ChildSpawnerCommands, size: Vec3) {
        for side in [-1.0f32, 1.0] {
            parent.spawn((
                Mesh3d(self.mesh.clone()),
                MeshMaterial3d(self.mat.clone()),
                Transform::from_xyz(side * (size.x / 2.0 + 0.06), -size.y * 0.12, -0.04),
            ));
        }
    }
}
