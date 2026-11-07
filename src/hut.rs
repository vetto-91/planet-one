//! Casetta-chiosco del Viandante: esterni, fondamenta e scale di pietra, porta
//! apribile, interni arredati, stufa con camino e fumo.
//!
//! Il mesh delle pareti/tetto usa le UV dalle 4 viste della foto
//! (`assets/textures/capanna.png`, ingrandita 2x, con alfa 0 sull'apertura del
//! banco e sulla porta). Origine = centro della pianta, quota del terreno più
//! basso sotto la pianta (vedi `props::SnapToGround`). Il fronte guarda -Z.

use crate::camera::BlocksCamera;
use crate::entities::Interactable;
use crate::props::SnapToGround;
use crate::scene::SceneNode;
use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Larghezza (x) e profondità (z) della pianta, altezza delle pareti (m).
const W: f32 = 4.0;
const D: f32 = 6.0;
const WALL_H: f32 = 2.36;
/// Di quanto il blocco della casetta è affondato nel terreno più basso (m): banco e
/// pavimento più vicini al giocatore.
const HUT_SINK: f32 = 0.30;
/// Quota del pavimento sopra il terreno più basso (le fondamenta lo reggono).
const FLOOR_Y: f32 = 0.5;
/// Superficie calpestabile (assi) appena sopra il pavimento.
const Y0: f32 = FLOOR_Y + 0.02;
/// Tetto a padiglione: altezza, sbalzo e rientro della cima piatta.
const ROOF_H: f32 = 1.1;
const ROOF_OVER: f32 = 0.3;
const ROOF_INSET: f32 = 0.4;
/// Altezza della fascia che chiude lo spazio tra parete e tetto.
const SOFFIT: f32 = 0.4714;
/// Gradini davanti alla porta (gli ultimi finiscono sotto il terreno).
const STEPS: i32 = 6;
/// Fondamenta: sporgenza oltre i muri e profondità sotto il pavimento.
const FOUND_MARGIN: f32 = 0.15;
const FOUND_BOTTOM: f32 = -1.0;
/// Porta (x locale: cerniera a destra guardando la facciata) e apertura.
const DOOR_X_HINGE: f32 = -1.73;
const DOOR_W: f32 = 0.99;
const DOOR_H: f32 = 1.96;
/// Angolo di apertura della porta (verso l'interno).
const DOOR_OPEN_ANGLE: f32 = -1.75;
/// Posizione locale del negoziante dietro il banco (centro del corpo).
pub const KEEPER_SHOP: Vec3 = Vec3::new(0.74, Y0 + 0.85, -2.0);
/// Camino: posizione orizzontale e punta del tubo.
const CHIMNEY_XZ: Vec2 = Vec2::new(1.5, 2.45);
const CHIMNEY_TOP_Y: f32 = FLOOR_Y + WALL_H + ROOF_H + 0.7;

// ----- Componenti e messaggi -----

/// Tipo di oggetto interagibile della casetta.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropKind {
    Door,
    Window,
    Stove,
    /// Interruttore della luce di casa.
    Light,
    /// Serranda del negozio.
    Shutter,
}

/// Stato di un oggetto, sull'entità "maniglia" (quella con `PropKind` e
/// `Interactable`). `on`: porta/finestra/serranda = aperta, stufa/luce = accesa.
#[derive(Component)]
pub struct PropState {
    pub on: bool,
}

/// Anta incernierata (porta o imposta): ruota verso `open_angle` quando lo
/// stato della maniglia `state` è `on`.
#[derive(Component)]
pub struct Hinge {
    pub state: Entity,
    pub open_angle: f32,
    pub angle: f32,
    /// Rotazione di base del pivot (orientamento della parete).
    pub base: Quat,
}

/// Pannello che scorre in verticale (serranda): `y` locale tra chiusa e aperta.
#[derive(Component)]
pub struct Slider {
    pub state: Entity,
    pub closed_y: f32,
    pub open_y: f32,
    pub y: f32,
}

/// Collider del pannello della porta (disattivato quando è aperta).
#[derive(Component)]
pub struct DoorCollider;

/// Luce della stufa (bagliore quando accesa).
#[derive(Component)]
pub struct StoveLight;

/// Lampada di casa: mesh emissivo e luce, seguono l'interruttore.
#[derive(Component)]
pub struct HouseLamp;
#[derive(Component)]
pub struct HouseLampLight;
/// Levetta dell'interruttore.
#[derive(Component)]
pub struct SwitchLever;

/// Punta del camino: da qui esce il fumo.
#[derive(Component)]
pub struct Chimney;

/// Sbuffo di fumo.
#[derive(Component)]
pub struct SmokePuff {
    age: f32,
    life: f32,
    drift: Vec2,
}

/// Richiesta di commutare un oggetto (inviata da `entities::interact`).
#[derive(Message)]
pub struct PropToggle(pub Entity);

/// Le maniglie degli oggetti della casetta (per la routine del Viandante).
#[derive(Resource)]
pub struct HutParts {
    pub door: Entity,
    pub stove: Entity,
    pub light: Entity,
    pub shutter: Entity,
}

// ----- Mesh -----

struct MeshBuf {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl MeshBuf {
    fn new() -> Self {
        Self {
            positions: vec![],
            normals: vec![],
            uvs: vec![],
            indices: vec![],
        }
    }

    /// Faccia con gli angoli in ordine alto-sx, alto-dx, basso-dx, basso-sx
    /// visti da fuori; `uv` nello stesso ordine.
    fn quad(&mut self, c: [Vec3; 4], uv: [Vec2; 4]) {
        let normal = (c[3] - c[0]).cross(c[2] - c[3]).normalize_or_zero();
        let base = self.positions.len() as u32;
        for (p, t) in c.iter().zip(uv) {
            self.positions.push(p.to_array());
            self.normals.push(normal.to_array());
            self.uvs.push(t.to_array());
        }
        self.indices
            .extend_from_slice(&[base, base + 3, base + 2, base, base + 2, base + 1]);
    }

    fn finish(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// UV in pixel della foto (512x512) -> 0..1.
fn p(x: f32, y: f32) -> Vec2 {
    Vec2::new(x / 512.0, y / 512.0)
}

/// Rettangolo UV (alto-sx, alto-dx, basso-dx, basso-sx) in pixel della foto.
fn rect_uv(ox: f32, oy: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> [Vec2; 4] {
    [
        p(ox + x0, oy + y0),
        p(ox + x1, oy + y0),
        p(ox + x1, oy + y1),
        p(ox + x0, oy + y1),
    ]
}

/// Le 4 pareti: (direzione verso l'esterno, "sinistra" vista da fuori, mezza
/// lunghezza, distanza dal centro, origine della vista nella foto).
type Side = (Vec3, Vec3, f32, f32, (f32, f32));

fn side_frames() -> [Side; 4] {
    let (hw, hd) = (W / 2.0, D / 2.0);
    [
        (Vec3::NEG_Z, Vec3::X, hw, hd, (0.0, 0.0)), // 0 fronte: banco e porta
        (Vec3::X, Vec3::Z, hd, hw, (256.0, 256.0)), // 1 lato +X
        (Vec3::Z, Vec3::NEG_X, hw, hd, (0.0, 256.0)), // 2 retro
        (Vec3::NEG_X, Vec3::NEG_Z, hd, hw, (256.0, 0.0)), // 3 lato -X
    ]
}

/// Punto sulla parete corrispondente a un pixel (relativo alla vista) della foto.
fn wall_point(side: &Side, px: (f32, f32)) -> Vec3 {
    let (dir, left, half, dist, _) = *side;
    let u = (px.0 - 40.0) / 178.0;
    let v = (px.1 - 113.0) / 105.0;
    dir * dist + left * (half * (1.0 - 2.0 * u)) + Vec3::Y * (FLOOR_Y + WALL_H - v * WALL_H)
}

/// Dimensioni (m) delle finestre apribili.
const WIN_W: f32 = 1.0;
const WIN_H: f32 = 0.8;
/// Angolo di apertura delle imposte (verso l'esterno).
const WIN_OPEN_ANGLE: f32 = 1.75;
/// Finestre: (indice parete, centro in pixel relativi alla vista).
const WINDOWS: [(usize, (f32, f32)); 3] = [
    (1, (156.5, 150.0)), // lato +X
    (2, (94.5, 150.0)),  // retro
    (3, (90.0, 158.0)),  // lato -X
];

/// Pareti, fascia sotto il tetto, tetto a padiglione, coperchio e pavimento.
#[allow(clippy::type_complexity)]
fn hut_mesh() -> Mesh {
    let mut b = MeshBuf::new();
    let (hw, hd) = (W / 2.0, D / 2.0);
    let (y0, y1) = (FLOOR_Y, FLOOR_Y + WALL_H);
    let y2 = y1 + ROOF_H;
    // (direzione verso l'esterno, "sinistra" da fuori, mezza lunghezza, distanza dal centro, origine vista)
    let sides: [(Vec3, Vec3, f32, f32, (f32, f32)); 4] = [
        (Vec3::NEG_Z, Vec3::X, hw, hd, (0.0, 0.0)), // fronte: banco e porta
        (Vec3::X, Vec3::Z, hd, hw, (256.0, 256.0)), // lato +X
        (Vec3::Z, Vec3::NEG_X, hw, hd, (0.0, 256.0)), // retro
        (Vec3::NEG_X, Vec3::NEG_Z, hd, hw, (256.0, 0.0)), // lato -X
    ];
    for (dir, left, half, dist, (ox, oy)) in sides {
        let c = dir * dist;
        let up = Vec3::Y;
        // Parete (riquadro della parete nella vista).
        b.quad(
            [
                c + left * half + up * y1,
                c - left * half + up * y1,
                c - left * half + up * y0,
                c + left * half + up * y0,
            ],
            rect_uv(ox, oy, 40.0, 113.0, 218.0, 218.0),
        );
        // Fascia tra la parete e il tetto (sotto la paglia).
        b.quad(
            [
                c + left * half + up * (y1 + SOFFIT),
                c - left * half + up * (y1 + SOFFIT),
                c - left * half + up * y1,
                c + left * half + up * y1,
            ],
            rect_uv(ox, oy, 60.0, 98.0, 200.0, 110.0),
        );
        // Falda del tetto: dalla gronda (con sbalzo) alla cima piatta.
        let (eh, ed) = (half + ROOF_OVER, dist + ROOF_OVER);
        let (th, td) = (half - ROOF_INSET, dist - ROOF_INSET);
        b.quad(
            [
                dir * td + left * th + up * y2,
                dir * td - left * th + up * y2,
                dir * ed - left * eh + up * y1,
                dir * ed + left * eh + up * y1,
            ],
            [
                p(ox + 39.0, oy + 28.0),
                p(ox + 215.0, oy + 28.0),
                p(ox + 237.0, oy + 112.0),
                p(ox + 24.0, oy + 112.0),
            ],
        );
    }
    // Coperchio piatto del tetto.
    let (tx, tz) = (hw - ROOF_INSET, hd - ROOF_INSET);
    b.quad(
        [
            Vec3::new(tx, y2, tz),
            Vec3::new(-tx, y2, tz),
            Vec3::new(-tx, y2, -tz),
            Vec3::new(tx, y2, -tz),
        ],
        rect_uv(0.0, 0.0, 70.0, 45.0, 190.0, 95.0),
    );
    // Pavimento di assi.
    b.quad(
        [
            Vec3::new(hw, Y0, hd),
            Vec3::new(-hw, Y0, hd),
            Vec3::new(-hw, Y0, -hd),
            Vec3::new(hw, Y0, -hd),
        ],
        rect_uv(0.0, 0.0, 80.0, 190.0, 180.0, 210.0),
    );
    b.finish()
}

/// Aggiunge un parallelepipedo di pietra (5 facce, niente base) con UV in metri.
fn stone_box(b: &mut MeshBuf, min: Vec3, max: Vec3) {
    const S: f32 = 0.7; // tile di pietra per metro
    let (lx, ly, lz) = (
        (max.x - min.x) * S,
        (max.y - min.y) * S,
        (max.z - min.z) * S,
    );
    let uv = |w: f32, h: f32| {
        [
            Vec2::new(0.0, 0.0),
            Vec2::new(w, 0.0),
            Vec2::new(w, h),
            Vec2::new(0.0, h),
        ]
    };
    // fronte (-Z)
    b.quad(
        [
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
        ],
        uv(lx, ly),
    );
    // retro (+Z)
    b.quad(
        [
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(min.x, min.y, max.z),
        ],
        uv(lx, ly),
    );
    // +X
    b.quad(
        [
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, min.y, max.z),
        ],
        uv(lz, ly),
    );
    // -X
    b.quad(
        [
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(min.x, min.y, min.z),
        ],
        uv(lz, ly),
    );
    // cima
    b.quad(
        [
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
        ],
        uv(lx, lz),
    );
}

/// Fondamenta e gradini di pietra.
fn stone_mesh() -> Mesh {
    let mut b = MeshBuf::new();
    let (fx, fz) = (W / 2.0 + FOUND_MARGIN, D / 2.0 + FOUND_MARGIN);
    stone_box(
        &mut b,
        Vec3::new(-fx, FOUND_BOTTOM, -fz),
        Vec3::new(fx, FLOOR_Y, fz),
    );
    // 3 gradini davanti alla porta, centrati sulla porta.
    let cx = DOOR_X_HINGE + DOOR_W / 2.0;
    let rise = FLOOR_Y / 3.0;
    for k in 1..=STEPS {
        let z_in = -fz - (k - 1) as f32 * 0.30;
        let z_out = z_in - 0.30;
        stone_box(
            &mut b,
            Vec3::new(cx - 0.55, FOUND_BOTTOM, z_out),
            Vec3::new(cx + 0.55, FLOOR_Y - rise * k as f32, z_in),
        );
    }
    b.finish()
}

// ----- Costruzione -----

fn solid(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    rough: f32,
    metal: f32,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: rough,
        metallic: metal,
        ..default()
    })
}

/// Aggiunge un parallelepipedo colorato (con collider opzionale).
fn add_box(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    mat: &Handle<StandardMaterial>,
    size: Vec3,
    pos: Vec3,
    collide: bool,
) {
    let mut e = parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(pos),
    ));
    if collide {
        e.insert(Collider::cuboid(size.x, size.y, size.z));
    }
}

/// Aggiunge un cilindro verticale colorato.
fn add_cyl(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    mat: &Handle<StandardMaterial>,
    radius: f32,
    height: f32,
    pos: Vec3,
) {
    parent.spawn((
        Mesh3d(meshes.add(Cylinder::new(radius, height))),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(pos),
    ));
}

/// Collider invisibile che blocca anche la camera.
fn add_wall(parent: &mut ChildSpawnerCommands, size: Vec3, pos: Vec3) {
    parent.spawn((
        Collider::cuboid(size.x, size.y, size.z),
        Transform::from_translation(pos),
        BlocksCamera,
    ));
}

/// Crea la casetta in `pos` con il fronte ruotato di `yaw`. Restituisce
/// l'entità radice (a cui agganciare il negoziante).
pub fn spawn_hut(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    pos: Vec3,
    yaw: f32,
) -> Entity {
    let hut_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(asset_server.load("textures/capanna.png")),
        alpha_mode: AlphaMode::Mask(0.5),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 0.9,
        ..default()
    });
    let stone_tex = asset_server
        .load_builder()
        .with_settings(|s: &mut ImageLoaderSettings| {
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
        .load("textures/pietra.png");
    let stone_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(stone_tex),
        perceptual_roughness: 0.95,
        ..default()
    });
    let door_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(asset_server.load("textures/porta.png")),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 0.9,
        ..default()
    });
    let shutter_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(asset_server.load("textures/serranda.png")),
        perceptual_roughness: 0.9,
        ..default()
    });
    let shutters_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(asset_server.load("textures/imposte.png")),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 0.9,
        ..default()
    });
    // Colori degli arredi.
    let wood = solid(materials, Color::srgb(0.45, 0.30, 0.18), 0.85, 0.0);
    let dark_wood = solid(materials, Color::srgb(0.28, 0.18, 0.11), 0.85, 0.0);
    let iron = solid(materials, Color::srgb(0.16, 0.16, 0.18), 0.5, 0.6);
    let cloth = solid(materials, Color::srgb(0.86, 0.80, 0.68), 0.95, 0.0);
    let blanket = solid(materials, Color::srgb(0.28, 0.42, 0.34), 0.95, 0.0);
    let pillow = solid(materials, Color::srgb(0.92, 0.90, 0.86), 0.95, 0.0);
    let rug = solid(materials, Color::srgb(0.52, 0.17, 0.14), 0.95, 0.0);
    let copper = solid(materials, Color::srgb(0.72, 0.42, 0.22), 0.4, 0.7);
    let glass = solid(materials, Color::srgb(0.45, 0.65, 0.58), 0.3, 0.0);
    let lamp_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.8, 0.4),
        emissive: LinearRgba::new(1.4, 0.9, 0.35, 1.0),
        ..default()
    });
    let book_cols = [
        Color::srgb(0.6, 0.2, 0.2),
        Color::srgb(0.2, 0.35, 0.55),
        Color::srgb(0.25, 0.45, 0.25),
        Color::srgb(0.55, 0.45, 0.15),
    ];
    let books: Vec<Handle<StandardMaterial>> = book_cols
        .iter()
        .map(|c| solid(materials, *c, 0.9, 0.0))
        .collect();

    let hut_mesh = meshes.add(hut_mesh());
    let stone = meshes.add(stone_mesh());
    let (hw, hd) = (W / 2.0, D / 2.0);
    let door_cx = DOOR_X_HINGE + DOOR_W / 2.0;
    let wall_y = FLOOR_Y + WALL_H / 2.0;
    let roof_top = FLOOR_Y + WALL_H + ROOF_H;

    let root = commands
        .spawn((
            SceneNode,
            SnapToGround {
                offset: -HUT_SINK,
                half: Vec2::new(hw + 0.3, hd + 0.3),
            },
            RigidBody::Static,
            Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
        ))
        .id();

    let mut parts: Option<HutParts> = None;
    commands.entity(root).with_children(|c| {
        // Pareti e tetto, fondamenta e gradini.
        c.spawn((Mesh3d(hut_mesh), MeshMaterial3d(hut_mat)));
        c.spawn((Mesh3d(stone), MeshMaterial3d(stone_mat)));

        // --- Collisioni ---
        // Fondamenta (e pavimento).
        c.spawn((
            Collider::cuboid(
                W + 2.0 * FOUND_MARGIN,
                FLOOR_Y - FOUND_BOTTOM,
                D + 2.0 * FOUND_MARGIN,
            ),
            Transform::from_xyz(0.0, (FLOOR_Y + FOUND_BOTTOM) / 2.0, 0.0),
            BlocksCamera,
        ));
        // Muri.
        add_wall(
            c,
            Vec3::new(W, WALL_H, 0.2),
            Vec3::new(0.0, wall_y, hd - 0.1),
        ); // retro
        add_wall(
            c,
            Vec3::new(0.2, WALL_H, D),
            Vec3::new(hw - 0.1, wall_y, 0.0),
        ); // lato +X
        add_wall(
            c,
            Vec3::new(0.2, WALL_H, D),
            Vec3::new(-hw + 0.1, wall_y, 0.0),
        ); // lato -X
        // Facciata a blocchi: banco (pieno), montante, fianco, architrave sopra la porta.
        add_wall(
            c,
            Vec3::new(2.45, WALL_H, 0.2),
            Vec3::new(0.775, wall_y, -hd + 0.1),
        );
        add_wall(
            c,
            Vec3::new(0.38, WALL_H, 0.2),
            Vec3::new(-0.55, wall_y, -hd + 0.1),
        );
        add_wall(
            c,
            Vec3::new(0.27, WALL_H, 0.2),
            Vec3::new(-1.865, wall_y, -hd + 0.1),
        );
        add_wall(
            c,
            Vec3::new(DOOR_W, WALL_H - DOOR_H, 0.2),
            Vec3::new(
                door_cx,
                FLOOR_Y + DOOR_H + (WALL_H - DOOR_H) / 2.0,
                -hd + 0.1,
            ),
        );
        // Tetto.
        add_wall(
            c,
            Vec3::new(W + 2.0 * ROOF_OVER, ROOF_H, D + 2.0 * ROOF_OVER),
            Vec3::new(0.0, FLOOR_Y + WALL_H + ROOF_H / 2.0, 0.0),
        );
        // Rampa sotto i gradini (collider liscio che segue la scala).
        let rise_total = FLOOR_Y / 3.0 * STEPS as f32; // dal pavimento fin sotto il terreno
        let run = 0.3 * STEPS as f32;
        let ang = (rise_total / run).atan();
        let len = (run * run + rise_total * rise_total).sqrt();
        let mid = Vec3::new(
            door_cx,
            FLOOR_Y - rise_total / 2.0,
            -(hd + FOUND_MARGIN) - run / 2.0,
        );
        let normal = Vec3::new(0.0, ang.cos(), -ang.sin());
        c.spawn((
            Collider::cuboid(1.1, 0.1, len),
            Transform::from_translation(mid - normal * 0.05)
                .with_rotation(Quat::from_rotation_x(-ang)),
        ));

        // --- Porta apribile: stato sulla maniglia, pivot sulla cerniera ---
        // Punto di interazione sul piano della porta: si usa da fuori e da dentro.
        let door_h = c
            .spawn((
                PropKind::Door,
                PropState { on: false },
                Interactable {
                    prompt: "[E] Apri porta".to_string(),
                    range: 3.0,
                    aim: Some(0.7),
                },
                Transform::from_xyz(door_cx, FLOOR_Y + 1.0, -hd),
            ))
            .id();
        let hinge = Vec3::new(DOOR_X_HINGE, FLOOR_Y, -hd);
        c.spawn((
            Hinge {
                state: door_h,
                open_angle: DOOR_OPEN_ANGLE,
                angle: 0.0,
                base: Quat::IDENTITY,
            },
            Transform::from_translation(hinge),
            Visibility::default(),
        ))
        .with_children(|d| {
            // Pannello: x locale 0..DOOR_W (la cerniera è a x=0), visto da fuori il bordo sinistro è +X.
            let mut mb = MeshBuf::new();
            mb.quad(
                [
                    Vec3::new(DOOR_W, DOOR_H, 0.0),
                    Vec3::new(0.0, DOOR_H, 0.0),
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(DOOR_W, 0.0, 0.0),
                ],
                [
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ],
            );
            d.spawn((Mesh3d(meshes.add(mb.finish())), MeshMaterial3d(door_mat)));
            d.spawn((
                DoorCollider,
                Collider::cuboid(DOOR_W, DOOR_H, 0.1),
                Transform::from_xyz(DOOR_W / 2.0, DOOR_H / 2.0, 0.0),
                BlocksCamera,
            ));
        });

        // --- Interni (pavimento a Y0) ---
        // Letto lungo la parete -X, testata al fondo.
        add_box(
            c,
            meshes,
            &wood,
            Vec3::new(1.1, 0.28, 2.0),
            Vec3::new(-1.35, Y0 + 0.14, 1.9),
            true,
        );
        add_box(
            c,
            meshes,
            &cloth,
            Vec3::new(1.0, 0.18, 1.9),
            Vec3::new(-1.35, Y0 + 0.37, 1.9),
            false,
        );
        add_box(
            c,
            meshes,
            &blanket,
            Vec3::new(1.02, 0.07, 1.2),
            Vec3::new(-1.35, Y0 + 0.49, 1.35),
            false,
        );
        add_box(
            c,
            meshes,
            &pillow,
            Vec3::new(0.6, 0.1, 0.35),
            Vec3::new(-1.35, Y0 + 0.51, 2.62),
            false,
        );
        add_box(
            c,
            meshes,
            &dark_wood,
            Vec3::new(1.1, 0.75, 0.08),
            Vec3::new(-1.35, Y0 + 0.37, 2.86),
            false,
        );
        // Tappeto.
        add_box(
            c,
            meshes,
            &rug,
            Vec3::new(1.6, 0.02, 1.2),
            Vec3::new(0.15, Y0 + 0.01, -0.3),
            false,
        );
        // Tavolo con due sedie.
        add_box(
            c,
            meshes,
            &wood,
            Vec3::new(0.9, 0.06, 0.9),
            Vec3::new(0.0, Y0 + 0.75, 1.0),
            false,
        );
        for (lx, lz) in [(-0.4, 0.6), (0.4, 0.6), (-0.4, 1.4), (0.4, 1.4)] {
            add_box(
                c,
                meshes,
                &dark_wood,
                Vec3::new(0.06, 0.75, 0.06),
                Vec3::new(lx, Y0 + 0.375, lz),
                false,
            );
        }
        c.spawn((
            Collider::cuboid(0.9, 0.8, 0.9),
            Transform::from_xyz(0.0, Y0 + 0.4, 1.0),
        ));
        for (cz, back_dz) in [(0.2, -0.18), (1.8, 0.18)] {
            add_box(
                c,
                meshes,
                &wood,
                Vec3::new(0.4, 0.05, 0.4),
                Vec3::new(0.0, Y0 + 0.45, cz),
                false,
            );
            add_box(
                c,
                meshes,
                &wood,
                Vec3::new(0.4, 0.45, 0.05),
                Vec3::new(0.0, Y0 + 0.7, cz + back_dz),
                false,
            );
            for (lx, lz) in [(-0.17, -0.17), (0.17, -0.17), (-0.17, 0.17), (0.17, 0.17)] {
                add_box(
                    c,
                    meshes,
                    &dark_wood,
                    Vec3::new(0.04, 0.43, 0.04),
                    Vec3::new(lx, Y0 + 0.215, cz + lz),
                    false,
                );
            }
        }
        // Cucinino lungo la parete +X: piano con lavello, stufa, mensola.
        add_box(
            c,
            meshes,
            &wood,
            Vec3::new(0.6, 0.9, 2.2),
            Vec3::new(1.6, Y0 + 0.45, 0.9),
            true,
        );
        add_box(
            c,
            meshes,
            &iron,
            Vec3::new(0.4, 0.03, 0.55),
            Vec3::new(1.6, Y0 + 0.9, 0.4),
            false,
        ); // lavello
        add_cyl(
            c,
            meshes,
            &iron,
            0.025,
            0.3,
            Vec3::new(1.82, Y0 + 1.05, 0.4),
        ); // rubinetto
        // Stufa a legna.
        let stove_pos = Vec3::new(CHIMNEY_XZ.x, Y0 + 0.475, CHIMNEY_XZ.y);
        add_box(
            c,
            meshes,
            &iron,
            Vec3::new(0.6, 0.95, 0.75),
            stove_pos,
            true,
        );
        add_box(
            c,
            meshes,
            &dark_wood,
            Vec3::new(0.02, 0.3, 0.35),
            stove_pos + Vec3::new(-0.31, 0.0, 0.0),
            false,
        ); // sportello
        // Tubo che sale e buca il tetto, fino al comignolo.
        let pipe_bottom = Y0 + 0.95;
        let pipe_h = CHIMNEY_TOP_Y - pipe_bottom;
        add_cyl(
            c,
            meshes,
            &iron,
            0.07,
            pipe_h,
            Vec3::new(CHIMNEY_XZ.x, pipe_bottom + pipe_h / 2.0, CHIMNEY_XZ.y),
        );
        add_cyl(
            c,
            meshes,
            &iron,
            0.11,
            0.08,
            Vec3::new(CHIMNEY_XZ.x, CHIMNEY_TOP_Y, CHIMNEY_XZ.y),
        );
        c.spawn((
            Chimney,
            Transform::from_xyz(CHIMNEY_XZ.x, CHIMNEY_TOP_Y + 0.15, CHIMNEY_XZ.y),
        ));
        // Stufa: maniglia con stato, luce con ombre (resta dentro la casa).
        let stove_h = c
            .spawn((
                PropKind::Stove,
                PropState { on: false },
                Interactable {
                    prompt: "[E] Accendi stufa".to_string(),
                    range: 3.0,
                    aim: Some(0.55),
                },
                Transform::from_xyz(CHIMNEY_XZ.x - 0.35, Y0 + 0.6, CHIMNEY_XZ.y),
            ))
            .with_children(|s| {
                s.spawn((
                    StoveLight,
                    PointLight {
                        color: Color::srgb(1.0, 0.55, 0.25),
                        intensity: 0.0,
                        range: 6.0,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    Transform::from_xyz(-0.15, 0.3, 0.0),
                ));
            })
            .id();
        // Mensola con pentole e barattoli.
        add_box(
            c,
            meshes,
            &wood,
            Vec3::new(0.3, 0.04, 1.6),
            Vec3::new(1.75, Y0 + 1.5, 0.8),
            false,
        );
        for (z, r, h) in [(0.2, 0.09, 0.14), (0.55, 0.07, 0.1), (1.35, 0.08, 0.12)] {
            add_cyl(
                c,
                meshes,
                &copper,
                r,
                h,
                Vec3::new(1.75, Y0 + 1.52 + h / 2.0, z),
            );
        }
        for z in [0.85, 1.05] {
            add_cyl(
                c,
                meshes,
                &glass,
                0.05,
                0.14,
                Vec3::new(1.75, Y0 + 1.52 + 0.07, z),
            );
        }
        // Baule vicino alla porta.
        add_box(
            c,
            meshes,
            &wood,
            Vec3::new(0.5, 0.45, 0.9),
            Vec3::new(-1.65, Y0 + 0.225, -0.8),
            true,
        );
        add_box(
            c,
            meshes,
            &dark_wood,
            Vec3::new(0.52, 0.08, 0.92),
            Vec3::new(-1.65, Y0 + 0.49, -0.8),
            false,
        );
        // Scaffale con libri al fondo.
        add_box(
            c,
            meshes,
            &wood,
            Vec3::new(1.2, 1.7, 0.3),
            Vec3::new(0.2, Y0 + 0.85, 2.72),
            true,
        );
        for (i, mat) in books.iter().enumerate() {
            add_box(
                c,
                meshes,
                mat,
                Vec3::new(0.12, 0.28, 0.2),
                Vec3::new(-0.2 + i as f32 * 0.17, Y0 + 1.0 + 0.14, 2.55),
                false,
            );
        }
        // Lanterna appesa al soffitto: mesh e luce seguono l'interruttore.
        let lamp_y = Y0 + 2.1;
        add_cyl(
            c,
            meshes,
            &iron,
            0.01,
            roof_top - lamp_y,
            Vec3::new(0.0, lamp_y + (roof_top - lamp_y) / 2.0, 0.3),
        );
        c.spawn((
            HouseLamp,
            Mesh3d(meshes.add(Cuboid::new(0.16, 0.2, 0.16))),
            MeshMaterial3d(lamp_mat),
            Transform::from_xyz(0.0, lamp_y, 0.3),
        ));
        c.spawn((
            HouseLampLight,
            PointLight {
                color: Color::srgb(1.0, 0.78, 0.5),
                intensity: 180_000.0,
                range: 9.0,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_xyz(0.0, lamp_y - 0.1, 0.3),
        ));

        // Interruttore della luce: sul montante tra porta e finestra del negozio, lato interno.
        let sw = Vec3::new(-0.55, FLOOR_Y + 1.3, -hd + 0.015);
        add_box(
            c,
            meshes,
            &dark_wood,
            Vec3::new(0.12, 0.18, 0.03),
            sw,
            false,
        );
        c.spawn((
            SwitchLever,
            Mesh3d(meshes.add(Cuboid::new(0.03, 0.09, 0.03))),
            MeshMaterial3d(iron.clone()),
            Transform::from_translation(sw + Vec3::new(0.0, 0.0, 0.03)),
        ));
        let light_h = c
            .spawn((
                PropKind::Light,
                PropState { on: true },
                Interactable {
                    prompt: "[E] Spegni luce".to_string(),
                    range: 3.0,
                    aim: Some(0.3),
                },
                Transform::from_translation(sw),
            ))
            .id();

        // Serranda del negozio: pannello che scorre in verticale dietro l'apertura del banco.
        let shutter_closed = FLOOR_Y + 1.315;
        let shutter_open = shutter_closed + 1.05;
        let shutter_h = c
            .spawn((
                PropKind::Shutter,
                PropState { on: true },
                Interactable {
                    prompt: "[E] Abbassa serranda".to_string(),
                    range: 3.0,
                    // Piccolo raggio, sul lato dell'apertura: guardare il Viandante
                    // dietro il banco non deve attivare la serranda.
                    aim: Some(0.3),
                },
                Transform::from_xyz(1.6, FLOOR_Y + 1.5, -hd - 0.03),
            ))
            .id();
        // Maniglia (pomello) della serranda, sul lato dell'apertura, fuori dalla parete.
        add_box(
            c,
            meshes,
            &dark_wood,
            Vec3::new(0.1, 0.1, 0.05),
            Vec3::new(1.6, FLOOR_Y + 1.5, -hd - 0.03),
            false,
        );
        c.spawn((
            Slider {
                state: shutter_h,
                closed_y: shutter_closed,
                open_y: shutter_open,
                y: shutter_open,
            },
            Mesh3d(meshes.add(Cuboid::new(2.24, 0.9, 0.04))),
            MeshMaterial3d(shutter_mat),
            Transform::from_xyz(0.74, shutter_open, -hd + 0.06),
        ));

        // Finestre apribili: due imposte incernierate ai bordi esterni, si aprono verso l'esterno.
        let frames = side_frames();
        for (side_idx, px) in WINDOWS {
            let side = &frames[side_idx];
            let (dir, left, _, _, _) = *side;
            let center = wall_point(side, px);
            // Sistema locale: x = "sinistra" vista da fuori, y = su, z = verso l'interno.
            let frame = Quat::from_mat3(&Mat3::from_cols(left, Vec3::Y, -dir));
            let win_h = c
                .spawn((
                    PropKind::Window,
                    PropState { on: false },
                    Interactable {
                        prompt: "[E] Apri finestra".to_string(),
                        range: 3.0,
                        aim: Some(0.55),
                    },
                    Transform::from_translation(center),
                ))
                .id();
            let leaf_w = WIN_W / 2.0;
            // Anta sinistra: cerniera a +x, si estende verso il centro (-x).
            // Anta destra: cerniera a -x, si estende verso il centro (+x).
            for (sign, uv0, uv1) in [(1.0_f32, 0.0_f32, 0.5_f32), (-1.0, 0.5, 1.0)] {
                let mut mb = MeshBuf::new();
                let (xa, xb) = (0.0, -sign * leaf_w); // bordo alla cerniera, bordo libero
                let (tl_x, tr_x) = if sign > 0.0 { (xa, xb) } else { (xb, xa) };
                mb.quad(
                    [
                        Vec3::new(tl_x, WIN_H / 2.0, 0.0),
                        Vec3::new(tr_x, WIN_H / 2.0, 0.0),
                        Vec3::new(tr_x, -WIN_H / 2.0, 0.0),
                        Vec3::new(tl_x, -WIN_H / 2.0, 0.0),
                    ],
                    [
                        Vec2::new(uv0, 0.0),
                        Vec2::new(uv1, 0.0),
                        Vec2::new(uv1, 1.0),
                        Vec2::new(uv0, 1.0),
                    ],
                );
                c.spawn((
                    Hinge {
                        state: win_h,
                        open_angle: -sign * WIN_OPEN_ANGLE,
                        angle: 0.0,
                        base: frame,
                    },
                    Transform::from_translation(center + left * (sign * WIN_W / 2.0) + dir * 0.02)
                        .with_rotation(frame),
                    Visibility::default(),
                ))
                .with_children(|l| {
                    l.spawn((
                        Mesh3d(meshes.add(mb.finish())),
                        MeshMaterial3d(shutters_mat.clone()),
                    ));
                });
            }
        }

        parts = Some(HutParts {
            door: door_h,
            stove: stove_h,
            light: light_h,
            shutter: shutter_h,
        });
    });
    if let Some(p) = parts {
        commands.insert_resource(p);
    }
    root
}

// ----- Sistemi -----

/// Commuta l'oggetto quando il giocatore preme E guardandolo. Se è la stufa, il
/// Viandante da quel momento non la gestisce più fino al mattino dopo.
pub fn handle_prop_toggle(
    mut reader: MessageReader<PropToggle>,
    mut states: Query<(&PropKind, &mut PropState)>,
    mut keepers: Query<&mut Keeper>,
) {
    for PropToggle(handle) in reader.read() {
        let Ok((kind, mut st)) = states.get_mut(*handle) else {
            continue;
        };
        st.on = !st.on;
        info!("[HUT] {:?}: {}", kind, if st.on { "on" } else { "off" });
        if *kind == PropKind::Stove {
            for mut k in &mut keepers {
                k.stove_manual = true;
            }
        }
    }
}

/// Aggiorna il testo del prompt di ogni oggetto secondo il suo stato.
pub fn update_prop_prompts(mut handles: Query<(&PropKind, &PropState, &mut Interactable)>) {
    for (kind, st, mut inter) in &mut handles {
        let text = match (kind, st.on) {
            (PropKind::Door, true) => "[E] Chiudi porta",
            (PropKind::Door, false) => "[E] Apri porta",
            (PropKind::Window, true) => "[E] Chiudi finestra",
            (PropKind::Window, false) => "[E] Apri finestra",
            (PropKind::Stove, true) => "[E] Spegni stufa",
            (PropKind::Stove, false) => "[E] Accendi stufa",
            (PropKind::Light, true) => "[E] Spegni luce",
            (PropKind::Light, false) => "[E] Accendi luce",
            (PropKind::Shutter, true) => "[E] Abbassa serranda",
            (PropKind::Shutter, false) => "[E] Alza serranda",
        };
        if inter.prompt != text {
            inter.prompt = text.to_string();
        }
    }
}

/// Anima porta e imposte; attiva/disattiva il collider del pannello della porta.
pub fn animate_hinges(
    time: Res<Time>,
    mut commands: Commands,
    states: Query<&PropState>,
    mut hinges: Query<(&mut Hinge, &mut Transform, Option<&Children>)>,
    colliders: Query<(), With<DoorCollider>>,
) {
    for (mut h, mut tf, children) in &mut hinges {
        let Ok(st) = states.get(h.state) else {
            continue;
        };
        let target = if st.on { h.open_angle } else { 0.0 };
        let step = 2.6 * time.delta_secs();
        h.angle += (target - h.angle).clamp(-step, step);
        tf.rotation = h.base * Quat::from_rotation_y(h.angle);
        if let Some(children) = children {
            let blocking = !st.on && h.angle.abs() < 0.05;
            for child in children.iter() {
                if colliders.contains(child) {
                    if blocking {
                        commands.entity(child).remove::<ColliderDisabled>();
                    } else {
                        commands.entity(child).insert(ColliderDisabled);
                    }
                }
            }
        }
    }
}

/// Anima la serranda (scorrimento verticale).
pub fn animate_sliders(
    time: Res<Time>,
    states: Query<&PropState>,
    mut sliders: Query<(&mut Slider, &mut Transform)>,
) {
    for (mut sl, mut tf) in &mut sliders {
        let Ok(st) = states.get(sl.state) else {
            continue;
        };
        let target = if st.on { sl.open_y } else { sl.closed_y };
        let step = 1.1 * time.delta_secs();
        sl.y += (target - sl.y).clamp(-step, step);
        tf.translation.y = sl.y;
    }
}

/// Luce di casa: lampada (emissivo e luce) e levetta seguono l'interruttore.
pub fn apply_house_light(
    switches: Query<(&PropKind, &PropState)>,
    mut lights: Query<&mut PointLight, With<HouseLampLight>>,
    lamps: Query<&MeshMaterial3d<StandardMaterial>, With<HouseLamp>>,
    mut levers: Query<&mut Transform, With<SwitchLever>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(on) = switches
        .iter()
        .find(|(k, _)| **k == PropKind::Light)
        .map(|(_, s)| s.on)
    else {
        return;
    };
    for mut l in &mut lights {
        l.intensity = if on { 180_000.0 } else { 0.0 };
    }
    for m in &lamps {
        if let Some(mut mat) = materials.get_mut(&m.0) {
            mat.emissive = if on {
                LinearRgba::new(1.4, 0.9, 0.35, 1.0)
            } else {
                LinearRgba::BLACK
            };
        }
    }
    for mut tf in &mut levers {
        tf.rotation = Quat::from_rotation_x(if on { 0.5 } else { -0.5 });
    }
}

/// Bagliore tremolante della stufa accesa.
pub fn stove_light(
    time: Res<Time>,
    stoves: Query<(&PropKind, &PropState, &Children)>,
    mut lights: Query<&mut PointLight, With<StoveLight>>,
) {
    let t = time.elapsed_secs();
    for (kind, st, children) in &stoves {
        if *kind != PropKind::Stove {
            continue;
        }
        for child in children.iter() {
            if let Ok(mut l) = lights.get_mut(child) {
                l.intensity = if st.on {
                    60_000.0 * (1.0 + 0.2 * (t * 9.0).sin() * (t * 3.7).cos())
                } else {
                    0.0
                };
            }
        }
    }
}

/// Emette sbuffi di fumo dal comignolo mentre la stufa è accesa.
#[allow(clippy::too_many_arguments)]
pub fn emit_smoke(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    stoves: Query<(&PropKind, &PropState)>,
    chimneys: Query<&GlobalTransform, With<Chimney>>,
    puffs: Query<(), With<SmokePuff>>,
    mut timer: Local<f32>,
    mut mesh: Local<Option<Handle<Mesh>>>,
) {
    let lit = stoves.iter().any(|(k, s)| *k == PropKind::Stove && s.on);
    if !lit {
        *timer = 0.0;
        return;
    }
    *timer -= time.delta_secs();
    if *timer > 0.0 || puffs.iter().count() >= 40 {
        return;
    }
    *timer = 0.3;
    let Ok(top) = chimneys.single() else {
        return;
    };
    let m = mesh
        .get_or_insert_with(|| meshes.add(Sphere::new(0.5)))
        .clone();
    let t = time.elapsed_secs();
    let jitter = Vec2::new((t * 12.9898).sin() * 0.05, (t * 78.233).cos() * 0.05);
    commands.spawn((
        SceneNode,
        SmokePuff {
            age: 0.0,
            life: 4.0,
            drift: Vec2::new(0.12 + jitter.x, 0.05 + jitter.y),
        },
        Mesh3d(m),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.75, 0.75, 0.78, 0.5),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_translation(top.translation()).with_scale(Vec3::splat(0.15)),
    ));
}

/// Sbuffi: salgono, si allargano e svaniscono.
pub fn update_smoke(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut puffs: Query<(
        Entity,
        &mut SmokePuff,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let dt = time.delta_secs();
    for (e, mut puff, mut tf, mat) in &mut puffs {
        puff.age += dt;
        if puff.age >= puff.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = puff.age / puff.life;
        tf.translation += Vec3::new(puff.drift.x, 0.8, puff.drift.y) * dt;
        tf.scale = Vec3::splat(0.15 + 0.6 * k);
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.base_color = Color::srgba(0.75, 0.75, 0.78, 0.5 * (1.0 - k));
        }
    }
}

// ----- Routine del Viandante (giorno, sera, notte, mattina) -----

/// Ore: dopo le 21 va a dormire, alle 6 si sveglia.
const SLEEP_HOUR: f32 = 21.0;
const WAKE_HOUR: f32 = 6.0;
/// Velocità di camminata (m/s) e altezza del centro del corpo in piedi.
const KEEPER_SPEED: f32 = 1.5;
const STAND_Y: f32 = Y0 + 0.85;
/// Punti (x, z locali) della routine.
const SHOP_XZ: Vec2 = Vec2::new(KEEPER_SHOP.x, KEEPER_SHOP.z);
const STOVE_STAND: Vec2 = Vec2::new(0.85, 2.45);
const DOOR_STAND: Vec2 = Vec2::new(-1.0, -2.3);
const SWITCH_STAND: Vec2 = Vec2::new(-0.55, -2.35);
const BED_FOOT: Vec2 = Vec2::new(-1.3, 0.55);
const BED_LIE: Vec3 = Vec3::new(-1.35, Y0 + 0.46 + 0.35, 1.85);

/// La stufa del Viandante è accesa 6–10 e 17–21, spenta nel resto della giornata.
fn stove_wanted(hour: f32) -> bool {
    (6.0..10.0).contains(&hour) || (17.0..21.0).contains(&hour)
}

/// Fase della giornata del negoziante.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KeeperPhase {
    /// Al banco.
    Day,
    /// Sequenza serale (stufa, serranda, luce, porta, letto).
    Evening,
    /// Dorme nel letto.
    Asleep,
    /// Sequenza mattutina (luce, porta, stufa, serranda, banco).
    Morning,
    /// Va ad accendere/spegnere la stufa durante il giorno.
    StoveTask,
}

/// Passo di una sequenza della routine.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// Cammina fino a (x, z) locali.
    Walk(Vec2),
    /// Si gira verso l'angolo (yaw locale).
    Face(f32),
    /// Aspetta (secondi).
    Wait(f32),
    /// Accende (true) o spegne la stufa.
    Stove(bool),
    /// Accende (true) o spegne la luce di casa.
    Light(bool),
    /// Alza (true) o abbassa la serranda.
    Shutter(bool),
    /// Chiude la porta se è aperta.
    CloseDoor,
    /// Si sdraia sul letto.
    Lie,
    /// Si alza e si mette in piedi accanto al letto.
    Rise,
}

/// Routine del negoziante, sul Viandante.
#[derive(Component)]
pub struct Keeper {
    phase: KeeperPhase,
    steps: std::collections::VecDeque<Step>,
    wait: f32,
    /// Il giocatore ha toccato la stufa: il Viandante non la gestisce fino al mattino.
    pub stove_manual: bool,
}

impl Keeper {
    pub fn new() -> Self {
        Self {
            phase: KeeperPhase::Day,
            steps: Default::default(),
            wait: 0.0,
            stove_manual: false,
        }
    }
}

/// Yaw locale per guardare in direzione `d` (x, z); il fronte è -Z.
fn yaw_towards(d: Vec2) -> f32 {
    (-d.x).atan2(-d.y)
}

type Steps = std::collections::VecDeque<Step>;

/// Passi per andare alla stufa, commutarla e tornare al banco.
fn stove_trip(on: bool, back_to_shop: bool) -> Vec<Step> {
    use Step::*;
    let mut v = vec![
        Walk(Vec2::new(0.8, -0.5)),
        Walk(STOVE_STAND),
        Face(yaw_towards(Vec2::new(1.0, 0.0))),
        Wait(0.6),
        Stove(on),
        Wait(0.5),
    ];
    if back_to_shop {
        v.extend([Walk(Vec2::new(0.8, -0.5)), Walk(SHOP_XZ), Face(0.0)]);
    }
    v
}

/// Sera: (spegne la stufa se è sua) → serranda giù → luce spenta → porta chiusa → a letto.
fn evening_steps(stove_to_off: bool) -> Steps {
    use Step::*;
    let mut v: Vec<Step> = Vec::new();
    if stove_to_off {
        v.extend(stove_trip(false, false));
        v.extend([Walk(Vec2::new(0.8, -0.5)), Walk(SHOP_XZ)]);
    }
    v.extend([
        Walk(SHOP_XZ),
        Face(0.0),
        Wait(0.5),
        Shutter(false),
        Wait(1.2),
        Walk(SWITCH_STAND),
        Face(0.0),
        Wait(0.4),
        Light(false),
        Wait(0.5),
        Walk(DOOR_STAND),
        Face(0.0),
        Wait(0.3),
        CloseDoor,
        Wait(0.8),
        Walk(Vec2::new(-1.0, -0.6)),
        Walk(BED_FOOT),
        Lie,
    ]);
    v.into()
}

/// Mattina: si alza → luce accesa → porta controllata/chiusa → stufa accesa → serranda su → banco.
fn morning_steps() -> Steps {
    use Step::*;
    let mut v: Vec<Step> = vec![
        Rise,
        Wait(0.5),
        Walk(Vec2::new(-1.0, -0.6)),
        Walk(SWITCH_STAND),
        Face(0.0),
        Wait(0.4),
        Light(true),
        Wait(0.5),
        Walk(DOOR_STAND),
        Face(0.0),
        Wait(0.3),
        CloseDoor,
        Wait(0.6),
    ];
    v.extend(stove_trip(true, false));
    v.extend([
        Walk(Vec2::new(0.8, -0.5)),
        Walk(SHOP_XZ),
        Face(0.0),
        Wait(0.4),
        Shutter(true),
        Wait(1.2),
    ]);
    v.into()
}

/// Sistema: gestisce la giornata del Viandante (stufa a orari, sera, notte,
/// mattina). Non parte mentre c'è un dialogo aperto.
#[allow(clippy::too_many_arguments)]
pub fn keeper_routine(
    time: Res<Time>,
    clock: Res<crate::daynight::GameClock>,
    dialog: Res<crate::entities::DialogState>,
    parts: Option<Res<HutParts>>,
    mut commands: Commands,
    mut keepers: Query<(
        Entity,
        &mut Keeper,
        &mut Transform,
        &mut crate::entities::Npc,
    )>,
    mut states: Query<&mut PropState>,
) {
    use crate::entities::NpcMode;
    let Some(parts) = parts else {
        return;
    };
    let dt = time.delta_secs();
    let hour = clock.hour;
    let night = !(WAKE_HOUR..SLEEP_HOUR).contains(&hour);
    for (entity, mut k, mut tf, mut npc) in &mut keepers {
        let stove_on = states.get(parts.stove).is_ok_and(|s| s.on);
        // Avvio delle sequenze.
        match k.phase {
            KeeperPhase::Day if !dialog.open && night => {
                // Spegne la stufa solo se l'aveva accesa lui (non toccata dal giocatore).
                k.steps = evening_steps(stove_on && !k.stove_manual);
                k.phase = KeeperPhase::Evening;
                npc.mode = NpcMode::Busy;
                commands.entity(entity).insert(ColliderDisabled);
                info!("[KEEPER] inizia la sera");
            }
            KeeperPhase::Day
                if !dialog.open && !k.stove_manual && stove_on != stove_wanted(hour) =>
            {
                k.steps = stove_trip(stove_wanted(hour), true).into();
                k.phase = KeeperPhase::StoveTask;
                npc.mode = NpcMode::Busy;
                commands.entity(entity).insert(ColliderDisabled);
                info!("[KEEPER] va alla stufa ({})", stove_wanted(hour));
            }
            KeeperPhase::Asleep if !night => {
                k.stove_manual = false;
                k.steps = morning_steps();
                k.phase = KeeperPhase::Morning;
                npc.mode = NpcMode::Busy;
                info!("[KEEPER] si sveglia");
            }
            _ => {}
        }
        if !matches!(
            k.phase,
            KeeperPhase::Evening | KeeperPhase::Morning | KeeperPhase::StoveTask
        ) {
            continue;
        }
        let Some(step) = k.steps.front().copied() else {
            // Sequenza finita.
            if k.phase == KeeperPhase::Evening {
                k.phase = KeeperPhase::Asleep;
                npc.mode = NpcMode::Asleep;
                info!("[KEEPER] dorme");
            } else {
                k.phase = KeeperPhase::Day;
                npc.mode = NpcMode::Ready;
                commands.entity(entity).remove::<ColliderDisabled>();
                info!("[KEEPER] al banco");
            }
            continue;
        };
        match step {
            Step::Walk(target) => {
                let here = Vec2::new(tf.translation.x, tf.translation.z);
                let to = target - here;
                if to.length() < 0.06 {
                    k.steps.pop_front();
                } else {
                    let dir = to.normalize();
                    let stepd = (KEEPER_SPEED * dt).min(to.length());
                    tf.translation.x += dir.x * stepd;
                    tf.translation.z += dir.y * stepd;
                    tf.translation.y = STAND_Y;
                    let want = Quat::from_rotation_y(yaw_towards(dir));
                    tf.rotation = tf.rotation.slerp(want, (10.0 * dt).clamp(0.0, 1.0));
                }
            }
            Step::Face(yaw) => {
                let want = Quat::from_rotation_y(yaw);
                tf.rotation = tf.rotation.slerp(want, (10.0 * dt).clamp(0.0, 1.0));
                if tf.rotation.angle_between(want) < 0.05 {
                    tf.rotation = want;
                    k.steps.pop_front();
                }
            }
            Step::Wait(t) => {
                k.wait += dt;
                if k.wait >= t {
                    k.wait = 0.0;
                    k.steps.pop_front();
                }
            }
            Step::Stove(on) => {
                if let Ok(mut s) = states.get_mut(parts.stove) {
                    s.on = on;
                }
                k.steps.pop_front();
            }
            Step::Light(on) => {
                if let Ok(mut s) = states.get_mut(parts.light) {
                    s.on = on;
                }
                k.steps.pop_front();
            }
            Step::Shutter(open) => {
                if let Ok(mut s) = states.get_mut(parts.shutter) {
                    s.on = open;
                }
                k.steps.pop_front();
            }
            Step::CloseDoor => {
                if let Ok(mut s) = states.get_mut(parts.door) {
                    s.on = false;
                }
                k.steps.pop_front();
            }
            Step::Lie => {
                tf.translation = BED_LIE;
                tf.rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
                k.steps.pop_front();
            }
            Step::Rise => {
                tf.translation = Vec3::new(BED_FOOT.x, STAND_Y, BED_FOOT.y);
                tf.rotation = Quat::from_rotation_y(yaw_towards(Vec2::new(0.0, -1.0)));
                k.steps.pop_front();
            }
        }
    }
}
