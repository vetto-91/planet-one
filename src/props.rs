//! Scenografia: casetta-chiosco del Viandante e appoggio a terra degli oggetti.
//!
//! La casetta è un unico mesh (pareti, tetto a padiglione, pavimento) con le
//! UV prese dalle 4 viste della foto in `assets/textures/capanna.png`
//! (ingrandita 2x; l'apertura del banco ha alfa 0 così si vede l'interno).
//! Il fronte della casetta guarda verso -Z locale, come i personaggi.

use crate::scene::{Ground, SceneNode};
use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Lato (m) della pianta della casetta.
const HUT_SIZE: f32 = 4.0;
/// Altezza (m) delle pareti.
const WALL_H: f32 = 2.36;
/// Altezza (m) del tetto a padiglione e lato (m) della sua cima piatta.
const ROOF_H: f32 = 1.1;
const ROOF_TOP: f32 = 3.2;
/// Lato (m) del tetto alla gronda (con sbalzo).
const ROOF_EAVE: f32 = 4.6;

/// Posizione locale (m) del negoziante dietro il banco: a sinistra del centro
/// guardando la facciata (+X locale) e verso il fronte (-Z locale).
pub const KEEPER_LOCAL: Vec3 = Vec3::new(0.68, 0.03 + 0.85, -1.0);

/// Dimensione in px della texture ritagliata dalla foto (4 viste da 256 px, 512 px totali).
const PHOTO: f32 = 512.0;
/// Riquadro della parete in ogni vista (px della foto, relativi alla vista).
const WALL_RECT: (f32, f32, f32, f32) = (40.0, 113.0, 218.0, 218.0);
/// Trapezio del tetto in ogni vista: (sx-alto, dx-alto, y-alto, sx-basso, dx-basso, y-basso).
const ROOF_TRAP: (f32, f32, f32, f32, f32, f32) = (39.0, 215.0, 28.0, 24.0, 237.0, 112.0);

/// Appoggia l'entità al terreno sotto di lei (raycast verso il basso sui
/// soli collider `Ground`). `offset` si somma alla quota trovata.
#[derive(Component)]
pub struct SnapToGround {
    pub offset: f32,
    /// Semi-lato (m) dell'area da campionare: si usa la quota più bassa.
    pub half_extent: f32,
}

/// Sistema: porta a terra le entità con [`SnapToGround`] (riprova ogni frame
/// finché il collider del terreno non è pronto).
pub fn snap_to_ground(
    mut commands: Commands,
    spatial: SpatialQuery,
    grounds: Query<(), With<Ground>>,
    mut items: Query<(Entity, &mut Transform, &SnapToGround)>,
) {
    for (entity, mut transform, snap) in &mut items {
        let p = transform.translation;
        let h = snap.half_extent;
        let mut lowest: Option<f32> = None;
        for (dx, dz) in [(0.0, 0.0), (h, h), (-h, h), (h, -h), (-h, -h)] {
            let origin = Vec3::new(p.x + dx, 40.0, p.z + dz);
            let hit = spatial.cast_ray_predicate(
                origin,
                Dir3::NEG_Y,
                100.0,
                true,
                &SpatialQueryFilter::default(),
                &|e| grounds.contains(e),
            );
            let Some(hit) = hit else {
                lowest = None;
                break;
            };
            let y = 40.0 - hit.distance;
            lowest = Some(lowest.map_or(y, |l: f32| l.min(y)));
        }
        let Some(y) = lowest else {
            continue; // terreno non ancora pronto
        };
        transform.translation.y = y + snap.offset;
        commands.entity(entity).remove::<SnapToGround>();
        info!(
            "[PROPS] appoggiato al terreno a y={:.2}",
            transform.translation.y
        );
    }
}

/// Aggiunge una faccia (quad) a un mesh in costruzione.
struct MeshBuf {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl MeshBuf {
    /// `c`: angoli in ordine alto-sx, alto-dx, basso-dx, basso-sx visti da fuori;
    /// `uv`: UV (in px della foto) dei 4 angoli nello stesso ordine.
    fn quad(&mut self, c: [Vec3; 4], uv: [(f32, f32); 4]) {
        let normal = (c[3] - c[0]).cross(c[2] - c[3]).normalize_or_zero();
        let base = self.positions.len() as u32;
        for (p, t) in c.iter().zip(uv) {
            self.positions.push(p.to_array());
            self.normals.push(normal.to_array());
            self.uvs.push([t.0 / PHOTO, t.1 / PHOTO]);
        }
        self.indices
            .extend_from_slice(&[base, base + 3, base + 2, base, base + 2, base + 1]);
    }
}

/// Costruisce il mesh della casetta (origine al centro del pavimento).
fn hut_mesh() -> Mesh {
    let mut b = MeshBuf {
        positions: vec![],
        normals: vec![],
        uvs: vec![],
        indices: vec![],
    };
    let hw = HUT_SIZE / 2.0;
    let y = Vec3::Y;
    // (direzione verso l'esterno, "sinistra" vista da fuori, origine della vista nella foto)
    let sides: [(Vec3, Vec3, (f32, f32)); 4] = [
        (Vec3::NEG_Z, Vec3::X, (0.0, 0.0)),       // fronte: banco e porta
        (Vec3::X, Vec3::Z, (256.0, 256.0)),       // lato +X
        (Vec3::Z, Vec3::NEG_X, (0.0, 256.0)),     // retro
        (Vec3::NEG_X, Vec3::NEG_Z, (256.0, 0.0)), // lato -X
    ];
    let (wx0, wy0, wx1, wy1) = WALL_RECT;
    let (tl, tr, ty, bl, br, by) = ROOF_TRAP;
    let roof_y = WALL_H + ROOF_H;
    for (dir, left, (ox, oy)) in sides {
        // Parete.
        let c = dir * hw;
        b.quad(
            [
                c + left * hw + y * WALL_H,
                c - left * hw + y * WALL_H,
                c - left * hw,
                c + left * hw,
            ],
            [
                (ox + wx0, oy + wy0),
                (ox + wx1, oy + wy0),
                (ox + wx1, oy + wy1),
                (ox + wx0, oy + wy1),
            ],
        );
        // Falda del tetto: dalla gronda (alta WALL_H) alla cima piatta.
        let e = ROOF_EAVE / 2.0;
        let t = ROOF_TOP / 2.0;
        b.quad(
            [
                dir * t + left * t + y * roof_y,
                dir * t - left * t + y * roof_y,
                dir * e - left * e + y * WALL_H,
                dir * e + left * e + y * WALL_H,
            ],
            [
                (ox + tl, oy + ty),
                (ox + tr, oy + ty),
                (ox + br, oy + by),
                (ox + bl, oy + by),
            ],
        );
    }
    // Coperchio piatto del tetto (toppa di paglia) e pavimento (assi).
    let t = ROOF_TOP / 2.0;
    b.quad(
        [
            Vec3::new(t, roof_y, t),
            Vec3::new(-t, roof_y, t),
            Vec3::new(-t, roof_y, -t),
            Vec3::new(t, roof_y, -t),
        ],
        [(70.0, 45.0), (190.0, 45.0), (190.0, 95.0), (70.0, 95.0)],
    );
    let f = hw;
    b.quad(
        [
            Vec3::new(f, 0.03, f),
            Vec3::new(-f, 0.03, f),
            Vec3::new(-f, 0.03, -f),
            Vec3::new(f, 0.03, -f),
        ],
        [(80.0, 190.0), (180.0, 190.0), (180.0, 210.0), (80.0, 210.0)],
    );

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, b.positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, b.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, b.uvs)
    .with_inserted_indices(Indices::U32(b.indices))
}

/// Crea la casetta in `pos` (la quota viene corretta da [`snap_to_ground`])
/// con il fronte ruotato di `yaw` radianti. Restituisce l'entità, a cui
/// agganciare il negoziante come figlio.
pub fn spawn_hut(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    pos: Vec3,
    yaw: f32,
) -> Entity {
    let mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(asset_server.load("textures/capanna.png")),
        alpha_mode: AlphaMode::Mask(0.5),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 0.9,
        ..default()
    });
    commands
        .spawn((
            SceneNode,
            SnapToGround {
                offset: -0.05,
                half_extent: HUT_SIZE / 2.0,
            },
            RigidBody::Static,
            // Un solo blocco: non ci si cammina attraverso né si entra dal banco.
            Collider::compound(vec![(
                Vec3::new(0.0, (WALL_H + ROOF_H) / 2.0, 0.0),
                Quat::IDENTITY,
                Collider::cuboid(HUT_SIZE + 0.4, WALL_H + ROOF_H, HUT_SIZE + 0.4),
            )]),
            Mesh3d(meshes.add(hut_mesh())),
            MeshMaterial3d(mat),
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)),
        ))
        .id()
}
