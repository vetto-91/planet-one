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
use bevy::light::NotShadowCaster;
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
/// Spessore del tetto (m): sottofaccia e bordo di gronda.
const ROOF_T: f32 = 0.14;
/// Profondità dello zoccolo della parete dentro le fondamenta (m).
const SKIRT: f32 = 0.15;
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
/// Posizione locale della pozione sul tavolo (sopra il piano a Y0+0,78).
pub const POTION_ON_TABLE: Vec3 = Vec3::new(0.0, Y0 + 0.99, 1.0);
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
    /// Focolare di una casa del paese.
    Hearth,
    /// Interruttore della luce di casa.
    Light,
    /// Serranda del negozio.
    Shutter,
    /// Vetro scorrevole di una finestra (maniglia interna): `on` = alzato.
    WindowGlass,
    /// Scuri di una finestra comandati da dentro (serve il vetro alzato).
    WindowInner,
    /// Battenti di un cancello del villaggio (aperti di giorno, chiusi la notte).
    Gate,
    /// Mobile che contiene oggetti (vedi `containers`).
    Container,
    /// Oggetto gettato a terra dal giocatore.
    Pickup,
    /// Il quadro di rame dietro la stufa del Viandante (QS-13): si apre con la chiave di rame.
    CopperPanel,
}

/// Lo sportello del quadro di rame: ruota sul cardine quando il quadro è aperto.
#[derive(Component)]
pub struct CopperLid {
    pub state: Entity,
}

/// Sistema: lo sportello del quadro di rame segue lo stato del quadro.
pub fn copper_lid(states: Query<&PropState>, mut lids: Query<(&CopperLid, &mut Transform)>) {
    for (lid, mut tf) in &mut lids {
        let open = states.get(lid.state).is_ok_and(|s| s.on);
        let rotation = Quat::from_rotation_y(if open { -1.9 } else { 0.0 });
        // Scritto solo al cambio: niente propagazione delle trasformazioni a ogni fotogramma.
        if tf.rotation != rotation {
            tf.rotation = rotation;
        }
    }
}

/// Identificativo stabile di un oggetto con stato, per i salvataggi.
#[derive(Component, Clone, Debug)]
pub struct PropId(pub String);

/// Serratura di una porta: chi può aprirla senza chiave.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum Lock {
    /// Sempre aperta a tutti.
    Public,
    /// Aperta negli orari di lavoro del proprietario (inizio, fine).
    WorkHours(f32, f32),
    /// Solo con la chiave.
    Private,
}

/// Chiave di una porta del paese (`casa 2`): chi ce l'ha la apre (`Thievery::keys`).
#[derive(Component, Clone, Debug)]
pub struct DoorKey(pub String);

/// Esito del tentativo di aprire una porta chiusa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Free,
    /// Si apre, ma solo grazie al passepartout.
    Master,
    Locked,
}

impl Lock {
    /// La porta è aperta al pubblico a quest'ora?
    pub fn open_to_all(self, hour: f32) -> bool {
        match self {
            Lock::Public => true,
            Lock::WorkHours(a, b) => (a..b).contains(&hour),
            Lock::Private => false,
        }
    }

    /// Da dentro si esce sempre; da fuori serve l'orario giusto o il passepartout.
    pub fn access(self, hour: f32, master: bool, from_inside: bool) -> Access {
        if from_inside || self.open_to_all(hour) {
            Access::Free
        } else if master {
            Access::Master
        } else {
            Access::Locked
        }
    }

    /// Testo mostrato davanti a una porta chiusa a chiave.
    pub fn closed_text(self) -> String {
        match self {
            Lock::WorkHours(a, _) => format!("Chiusa (apre alle {})", a as u32),
            _ => "Chiusa a chiave".to_string(),
        }
    }
}

/// La maniglia `door` è dal lato interno rispetto al giocatore? La porta guarda verso +Z locale.
fn player_inside(door: &GlobalTransform, player: Vec3) -> bool {
    let out = door.rotation() * Vec3::Z;
    (player - door.translation()).dot(out) < 0.0
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

/// Collegamento delle maniglie interne di una finestra: scuri (maniglia esterna) e vetro.
#[derive(Component)]
pub struct WindowLink {
    pub shutters: Entity,
    pub glass: Entity,
}

/// Collider che blocca la vista (`BlocksCamera`) solo quando lo stato di `state`
/// vale `active_when_on` (serranda abbassata, scuri chiusi).
#[derive(Component)]
pub struct LosBlocker {
    pub state: Entity,
    pub active_when_on: bool,
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
    pub(crate) age: f32,
    pub(crate) life: f32,
    pub(crate) drift: Vec2,
    /// Dimensione rispetto allo sbuffo della stufa.
    pub(crate) size: f32,
    pub(crate) grey: f32,
    pub(crate) alpha: f32,
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
    /// Maniglie degli scuri delle finestre (stesso ordine di `WINDOWS`).
    pub windows: Vec<Entity>,
    /// Maniglie del vetro scorrevole delle finestre (stesso ordine).
    pub glass: Vec<Entity>,
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

/// Anta spessa (scuro) di una finestra: parallelepipedo con la texture delle
/// imposte sulle due facce grandi (metà `uv0..uv1`) e un po' di legno sui bordi.
/// Estensione x in [x0, x1], altezza `h` centrata su y=0, spessore verso -Z.
fn leaf_mesh(x0: f32, x1: f32, h: f32, thick: f32, uv0: f32, uv1: f32) -> Mesh {
    let mut b = MeshBuf::new();
    let (hy, z0, z1) = (h / 2.0, -thick, 0.0);
    let wood = [
        Vec2::new(uv0 + 0.1, 0.45),
        Vec2::new(uv0 + 0.15, 0.45),
        Vec2::new(uv0 + 0.15, 0.5),
        Vec2::new(uv0 + 0.1, 0.5),
    ];
    // Faccia esterna (-Z): da fuori la sinistra è +X.
    b.quad(
        [
            Vec3::new(x1, hy, z0),
            Vec3::new(x0, hy, z0),
            Vec3::new(x0, -hy, z0),
            Vec3::new(x1, -hy, z0),
        ],
        [
            Vec2::new(uv0, 0.0),
            Vec2::new(uv1, 0.0),
            Vec2::new(uv1, 1.0),
            Vec2::new(uv0, 1.0),
        ],
    );
    // Faccia interna (+Z), vista da dentro: la sinistra è -X.
    b.quad(
        [
            Vec3::new(x0, hy, z1),
            Vec3::new(x1, hy, z1),
            Vec3::new(x1, -hy, z1),
            Vec3::new(x0, -hy, z1),
        ],
        [
            Vec2::new(uv0, 0.0),
            Vec2::new(uv1, 0.0),
            Vec2::new(uv1, 1.0),
            Vec2::new(uv0, 1.0),
        ],
    );
    // Bordi: alto, basso, lato +X, lato -X.
    b.quad(
        [
            Vec3::new(x1, hy, z1),
            Vec3::new(x0, hy, z1),
            Vec3::new(x0, hy, z0),
            Vec3::new(x1, hy, z0),
        ],
        wood,
    );
    b.quad(
        [
            Vec3::new(x1, -hy, z0),
            Vec3::new(x0, -hy, z0),
            Vec3::new(x0, -hy, z1),
            Vec3::new(x1, -hy, z1),
        ],
        wood,
    );
    b.quad(
        [
            Vec3::new(x1, hy, z1),
            Vec3::new(x1, hy, z0),
            Vec3::new(x1, -hy, z0),
            Vec3::new(x1, -hy, z1),
        ],
        wood,
    );
    b.quad(
        [
            Vec3::new(x0, hy, z0),
            Vec3::new(x0, hy, z1),
            Vec3::new(x0, -hy, z1),
            Vec3::new(x0, -hy, z0),
        ],
        wood,
    );
    b.finish()
}

/// Dimensioni (m) delle finestre apribili.
const WIN_W: f32 = 1.0;
const WIN_H: f32 = 0.8;
/// Angolo di apertura delle imposte (verso l'esterno).
const WIN_OPEN_ANGLE: f32 = 1.75;
/// Gli scuri sono più larghi/alti del foro (sovrapposizione sulla cornice) e spessi.
const LEAF_OVER: f32 = 0.03;
const LEAF_THICK: f32 = 0.04;
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
        // Zoccolo: la parete scende di 15 cm dentro le fondamenta, così non c'è
        // fessura (né luce che passa) tra muro e pietra.
        b.quad(
            [
                c + left * half + up * y0,
                c - left * half + up * y0,
                c - left * half + up * (y0 - SKIRT),
                c + left * half + up * (y0 - SKIRT),
            ],
            rect_uv(ox, oy, 40.0, 211.0, 218.0, 218.0),
        );
        // Fascia tra la parete e il tetto (sotto la paglia), con 10 cm di
        // sovrapposizione sulla parete.
        b.quad(
            [
                c + left * half + up * (y1 + SOFFIT),
                c - left * half + up * (y1 + SOFFIT),
                c - left * half + up * (y1 - 0.1),
                c + left * half + up * (y1 - 0.1),
            ],
            rect_uv(ox, oy, 60.0, 98.0, 200.0, 110.0),
        );
        // Falda del tetto: dalla gronda (con sbalzo) alla cima piatta.
        let (eh, ed) = (half + ROOF_OVER, dist + ROOF_OVER);
        let (th, td) = (half - ROOF_INSET, dist - ROOF_INSET);
        let slope_uv = [
            p(ox + 39.0, oy + 28.0),
            p(ox + 215.0, oy + 28.0),
            p(ox + 237.0, oy + 112.0),
            p(ox + 24.0, oy + 112.0),
        ];
        let top = [
            dir * td + left * th + up * y2,
            dir * td - left * th + up * y2,
            dir * ed - left * eh + up * y1,
            dir * ed + left * eh + up * y1,
        ];
        b.quad(top, slope_uv);
        // Sottofaccia: stessa falda abbassata di ROOF_T, rivolta verso il basso
        // (il tetto non è più un foglio e la luce non passa sul bordo del muro).
        let down = Vec3::Y * ROOF_T;
        b.quad(
            [top[3] - down, top[2] - down, top[1] - down, top[0] - down],
            [slope_uv[3], slope_uv[2], slope_uv[1], slope_uv[0]],
        );
        // Bordo di gronda.
        b.quad(
            [
                dir * ed + left * eh + up * y1,
                dir * ed - left * eh + up * y1,
                dir * ed - left * eh + up * (y1 - ROOF_T),
                dir * ed + left * eh + up * (y1 - ROOF_T),
            ],
            rect_uv(ox, oy, 60.0, 98.0, 200.0, 110.0),
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
    // Sottofaccia del coperchio.
    let yt = y2 - ROOF_T;
    b.quad(
        [
            Vec3::new(tx, yt, -tz),
            Vec3::new(-tx, yt, -tz),
            Vec3::new(-tx, yt, tz),
            Vec3::new(tx, yt, tz),
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

/// Muro (collider) con un foro per la finestra: quattro blocchi attorno al vano.
/// `along_x`: il muro corre lungo X (piano a z = `plane`), altrimenti lungo Z.
fn wall_with_hole(
    c: &mut ChildSpawnerCommands,
    along_x: bool,
    plane: f32,
    len: f32,
    hole_c: f32,
    hole_y: f32,
) {
    let (hwid, hh) = (WIN_W / 2.0, WIN_H / 2.0);
    let (y0, y1) = (FLOOR_Y, FLOOR_Y + WALL_H);
    let half = len / 2.0;
    let mut piece = |a0: f32, a1: f32, b0: f32, b1: f32| {
        let (l, h) = (a1 - a0, b1 - b0);
        if l <= 0.001 || h <= 0.001 {
            return;
        }
        let (ac, yc) = ((a0 + a1) / 2.0, (b0 + b1) / 2.0);
        if along_x {
            add_wall(c, Vec3::new(l, h, 0.2), Vec3::new(ac, yc, plane));
        } else {
            add_wall(c, Vec3::new(0.2, h, l), Vec3::new(plane, yc, ac));
        }
    };
    piece(-half, hole_c - hwid, y0, y1);
    piece(hole_c + hwid, half, y0, y1);
    piece(hole_c - hwid, hole_c + hwid, y0, hole_y - hh);
    piece(hole_c - hwid, hole_c + hwid, hole_y + hh, y1);
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
        base_color_texture: Some(crate::pbr::load_gpu(asset_server, "textures/capanna.png")),
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
            s.asset_usage = RenderAssetUsages::RENDER_WORLD;
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
        base_color_texture: Some(crate::pbr::load_gpu(asset_server, "textures/porta.png")),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 0.9,
        ..default()
    });
    let shutter_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(crate::pbr::load_gpu(asset_server, "textures/serranda.png")),
        perceptual_roughness: 0.9,
        ..default()
    });
    let shutters_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(crate::pbr::load_gpu(asset_server, "textures/imposte.png")),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 0.9,
        ..default()
    });
    // Vetro naturale, lo stesso delle finestre del paese (segnalazione 34).
    let glass_pane_mat = materials.add(crate::pbr::glass_material());
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
        // Muri laterali e retro con il vano delle finestre (si vede attraverso, i vetri sono a parte).
        let frames_c = side_frames();
        let win_c = |i: usize| wall_point(&frames_c[WINDOWS[i].0], WINDOWS[i].1);
        let (w0, w1, w2) = (win_c(0), win_c(1), win_c(2));
        wall_with_hole(c, false, hw - 0.1, D, w0.z, w0.y); // lato +X
        wall_with_hole(c, true, hd - 0.1, W, w1.x, w1.y); // retro
        wall_with_hole(c, false, -hw + 0.1, D, w2.z, w2.y); // lato -X
        // Facciata a blocchi: davanzale e architrave attorno all'apertura del banco
        // (x da -0,38 a 1,86, y da +0,865 a +1,765), montanti, fianco, architrave sopra la porta.
        let (ap_lo, ap_hi) = (FLOOR_Y + 0.865, FLOOR_Y + 1.765);
        add_wall(
            c,
            Vec3::new(2.24, ap_lo - FLOOR_Y, 0.2),
            Vec3::new(0.74, (FLOOR_Y + ap_lo) / 2.0, -hd + 0.1),
        ); // davanzale
        add_wall(
            c,
            Vec3::new(2.24, FLOOR_Y + WALL_H - ap_hi, 0.2),
            Vec3::new(0.74, (ap_hi + FLOOR_Y + WALL_H) / 2.0, -hd + 0.1),
        ); // architrave
        add_wall(
            c,
            Vec3::new(0.07, WALL_H, 0.2),
            Vec3::new(-0.415, wall_y, -hd + 0.1),
        );
        add_wall(
            c,
            Vec3::new(0.14, WALL_H, 0.2),
            Vec3::new(1.93, wall_y, -hd + 0.1),
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
                PropId("casetta:porta".to_string()),
                PropState { on: false },
                Interactable {
                    prompt: "[E] Apri porta".to_string(),
                    range: 3.0,
                    aim: Some(0.7),
                    use_from: None,
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
                PropId("casetta:stufa".to_string()),
                PropState { on: false },
                Interactable {
                    prompt: "[E] Accendi stufa".to_string(),
                    range: 3.0,
                    aim: Some(0.55),
                    use_from: None,
                },
                Transform::from_xyz(CHIMNEY_XZ.x - 0.35, Y0 + 0.6, CHIMNEY_XZ.y),
                // Ha una luce come figlia: senza visibilità propria Bevy avvisa (B0004).
                Visibility::default(),
            ))
            .with_children(|s| {
                s.spawn((
                    StoveLight,
                    crate::graphics::ShadowWanted::default(),
                    PointLight {
                        color: Color::srgb(1.0, 0.55, 0.25),
                        intensity: 0.0,
                        range: 6.0,
                        shadow_maps_enabled: false,
                        shadow_depth_bias: 0.012,
                        shadow_normal_bias: 0.25,
                        ..default()
                    },
                    Transform::from_xyz(-0.15, 0.3, 0.0),
                ));
            })
            .id();
        // Il quadro di rame dietro la stufa (QS-13): si apre con la chiave di rame; dentro, i fili della
        // Luce Viva scendono nel pavimento verso la piazza.
        let panel_copper = solid(materials, Color::srgb(0.66, 0.38, 0.2), 0.9, 0.35);
        let panel_at = Vec3::new(1.95, Y0 + 1.6, CHIMNEY_XZ.y);
        add_box(
            c,
            meshes,
            &panel_copper,
            Vec3::new(0.03, 0.5, 0.4),
            panel_at,
            false,
        );
        for dz in [-0.1f32, 0.0, 0.1] {
            add_box(
                c,
                meshes,
                &panel_copper,
                Vec3::new(0.02, 1.35, 0.025),
                Vec3::new(1.97, Y0 + 0.675, CHIMNEY_XZ.y + dz),
                false,
            );
        }
        let panel = c
            .spawn((
                PropKind::CopperPanel,
                PropId("casetta:quadro".to_string()),
                PropState { on: false },
                Interactable {
                    prompt: "[E] Apri il quadro di rame".to_string(),
                    range: 2.5,
                    aim: Some(0.3),
                    use_from: None,
                },
                Transform::from_translation(panel_at - Vec3::X * 0.08),
            ))
            .id();
        // Lo sportello, incernierato sul bordo verso il fondo della casa.
        c.spawn((
            CopperLid { state: panel },
            Transform::from_translation(panel_at + Vec3::new(-0.04, 0.0, -0.2)),
            Visibility::default(),
        ))
        .with_children(|l| {
            l.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.025, 0.48, 0.4))),
                MeshMaterial3d(panel_copper.clone()),
                Transform::from_xyz(0.0, 0.0, 0.2),
            ));
        });
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
        c.spawn(crate::containers::bundle(
            crate::containers::ContainerKind::Chest,
            Vec3::new(-1.65, Y0 + 0.62, -0.8),
            "casetta:baule".to_string(),
            "casetta",
        ));
        c.spawn(crate::containers::bundle(
            crate::containers::ContainerKind::Shelf,
            Vec3::new(0.2, Y0 + 1.05, 2.5),
            "casetta:scaffale".to_string(),
            "casetta",
        ));
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
            crate::graphics::ShadowWanted::default(),
            PointLight {
                color: Color::srgb(1.0, 0.78, 0.5),
                intensity: 180_000.0,
                range: 9.0,
                shadow_maps_enabled: false,
                shadow_depth_bias: 0.012,
                shadow_normal_bias: 0.25,
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
                PropId("casetta:luce".to_string()),
                PropState { on: true },
                Interactable {
                    prompt: "[E] Spegni luce".to_string(),
                    range: 3.0,
                    aim: Some(0.3),
                    use_from: Some(Vec3::Z),
                },
                Transform::from_translation(sw),
            ))
            .id();

        // Serranda del negozio: pannello che scorre in verticale dietro l'apertura del banco.
        let shutter_closed = FLOOR_Y + 1.315;
        let shutter_open = shutter_closed + 1.05;
        // Pulsante della serranda: SOLO all'interno, sulla parete +X (alla destra del
        // Viandante quando vende), vicino allo spigolo anteriore e ben lontano dalla
        // finestra; placchetta ruotata di 90° rispetto alla facciata (guarda verso -X).
        let btn = Vec3::new(hw - 0.015, FLOOR_Y + 1.3, -2.75);
        add_box(
            c,
            meshes,
            &dark_wood,
            Vec3::new(0.03, 0.14, 0.07),
            btn,
            false,
        );
        add_box(
            c,
            meshes,
            &iron,
            Vec3::new(0.03, 0.05, 0.03),
            btn + Vec3::new(-0.025, 0.0, 0.0),
            false,
        );
        let shutter_h = c
            .spawn((
                PropKind::Shutter,
                PropId("casetta:serranda".to_string()),
                PropState { on: true },
                Interactable {
                    prompt: "[E] Abbassa serranda".to_string(),
                    range: 3.0,
                    aim: Some(0.2),
                    use_from: Some(Vec3::NEG_X),
                },
                Transform::from_translation(btn),
            ))
            .id();
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
        ))
        .with_children(|p| {
            // Abbassata blocca la vista (e il passaggio); alzata no.
            p.spawn((
                Collider::cuboid(2.24, 0.9, 0.1),
                BlocksCamera,
                LosBlocker {
                    state: shutter_h,
                    active_when_on: false,
                },
                Transform::default(),
            ));
        });

        // Finestre apribili: due imposte incernierate ai bordi esterni, si aprono verso l'esterno.
        let frames = side_frames();
        let mut window_handles: Vec<Entity> = Vec::new();
        let mut glass_handles: Vec<Entity> = Vec::new();
        for (side_idx, px) in WINDOWS {
            let side = &frames[side_idx];
            let (dir, left, _, _, _) = *side;
            let center = wall_point(side, px);
            // Sistema locale: x = "sinistra" vista da fuori, y = su, z = verso l'interno.
            let frame = Quat::from_mat3(&Mat3::from_cols(left, Vec3::Y, -dir));
            let win_h = c
                .spawn((
                    PropKind::Window,
                    PropId(format!("casetta:scuri:{}", window_handles.len())),
                    PropState { on: false },
                    Interactable {
                        prompt: "[E] Apri finestra".to_string(),
                        range: 3.0,
                        aim: Some(0.45),
                        use_from: Some(dir),
                    },
                    // Maniglia degli scuri: 12 cm FUORI dal muro (si usa da fuori).
                    Transform::from_translation(center + dir * 0.12),
                ))
                .id();
            // Dentro: vetro scorrevole (maniglia in basso) e scuri (maniglia in alto).
            let inner = center - dir * 0.12;
            let glass_h = c
                .spawn((
                    PropKind::WindowGlass,
                    PropId(format!("casetta:vetro:{}", glass_handles.len())),
                    PropState { on: false },
                    Interactable {
                        prompt: "[E] Alza vetro".to_string(),
                        range: 3.0,
                        aim: Some(0.22),
                        use_from: Some(-dir),
                    },
                    Transform::from_translation(inner - Vec3::Y * 0.22),
                ))
                .id();
            c.spawn((
                PropKind::WindowInner,
                PropState { on: false },
                WindowLink {
                    shutters: win_h,
                    glass: glass_h,
                },
                Interactable {
                    prompt: "[E] Apri scuri".to_string(),
                    range: 3.0,
                    aim: Some(0.22),
                    use_from: Some(-dir),
                },
                Transform::from_translation(inner + Vec3::Y * 0.2),
            ));
            let leaf_w = WIN_W / 2.0;
            // Anta sinistra: cerniera a +x, si estende verso il centro (-x).
            // Anta destra: cerniera a -x, si estende verso il centro (+x).
            for (sign, uv0, uv1) in [(1.0_f32, 0.0_f32, 0.5_f32), (-1.0, 0.5, 1.0)] {
                // Dalla cerniera (x=0) verso il centro (−sign·x), con 3 cm in più su
                // entrambi i lati (cornice e centro), e 6 cm in più in altezza.
                let (xa, xb) = (sign * LEAF_OVER, -sign * (leaf_w + LEAF_OVER));
                let (x0, x1) = (xa.min(xb), xa.max(xb));
                let mesh = leaf_mesh(x0, x1, WIN_H + 2.0 * LEAF_OVER, LEAF_THICK, uv0, uv1);
                c.spawn((
                    Hinge {
                        state: win_h,
                        open_angle: -sign * WIN_OPEN_ANGLE,
                        angle: 0.0,
                        base: frame,
                    },
                    Transform::from_translation(center + left * (sign * WIN_W / 2.0) + dir * 0.005)
                        .with_rotation(frame),
                    Visibility::default(),
                ))
                .with_children(|l| {
                    l.spawn((
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(shutters_mat.clone()),
                    ));
                });
            }
            // Vetro scorrevole (sottile): sale lungo la faccia interna del muro.
            let pane_pos = center - dir * 0.03;
            c.spawn((
                Slider {
                    state: glass_h,
                    closed_y: pane_pos.y,
                    open_y: pane_pos.y + WIN_H * 0.9,
                    y: pane_pos.y,
                },
                Mesh3d(meshes.add(Cuboid::new(WIN_W, WIN_H, 0.01))),
                MeshMaterial3d(glass_pane_mat.clone()),
                NotShadowCaster,
                Transform::from_translation(pane_pos).with_rotation(frame),
            ));
            // Vano: il vetro (fisico) resta, non blocca la vista; gli scuri chiusi sì.
            let size = if dir.x.abs() > 0.5 {
                Vec3::new(0.1, WIN_H, WIN_W)
            } else {
                Vec3::new(WIN_W, WIN_H, 0.1)
            };
            c.spawn((
                Collider::cuboid(size.x, size.y, size.z),
                Transform::from_translation(center - dir * 0.05),
            ));
            c.spawn((
                Collider::cuboid(size.x, size.y, size.z),
                BlocksCamera,
                LosBlocker {
                    state: win_h,
                    active_when_on: false,
                },
                Transform::from_translation(center - dir * 0.05),
            ));
            glass_handles.push(glass_h);
            window_handles.push(win_h);
        }

        parts = Some(HutParts {
            door: door_h,
            stove: stove_h,
            light: light_h,
            shutter: shutter_h,
            windows: window_handles,
            glass: glass_handles,
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
#[allow(clippy::too_many_arguments)]
pub fn handle_prop_toggle(
    mut reader: MessageReader<PropToggle>,
    mut states: Query<(&PropKind, &mut PropState)>,
    links: Query<&WindowLink>,
    buttons: Query<&crate::village::GateButton>,
    mut keepers: Query<&mut Keeper>,
    locks: Query<(&Lock, &GlobalTransform, Option<&DoorKey>)>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    clock: Res<crate::daynight::GameClock>,
    furniture: Query<&crate::containers::Container>,
    mut containers: ResMut<crate::containers::Containers>,
    mut picking: crate::thievery::PickParams,
    mut next: ResMut<NextState<crate::AppState>>,
    mut commands: Commands,
    pickups: Query<&crate::inventory::Pickup>,
    mut carried: ResMut<crate::inventory::Inventory>,
) {
    for PropToggle(handle) in reader.read() {
        // Un oggetto gettato si riprende.
        if let Ok(p) = pickups.get(*handle) {
            carried.add(p.kind, p.qty);
            commands.entity(*handle).despawn();
            info!("[INVENTORY] ripreso {} x{}", p.kind.name(), p.qty);
            continue;
        }
        // Un mobile non cambia stato: si apre la sua finestra.
        if let Ok(c) = furniture.get(*handle) {
            // Un baule chiuso a chiave si forza col grimaldello (ADR 0039).
            if let Some(state) = picking.open(c, &mut containers, &carried) {
                next.set(state);
            }
            continue;
        }
        // I bottoni dei cancelli commutano lo stato del cancello.
        let handle = &buttons.get(*handle).map_or(*handle, |b| b.0);
        // Scuri comandati da dentro: servono il vetro alzato; si agisce sulla maniglia esterna.
        let mut target = *handle;
        if let Ok(link) = links.get(*handle)
            && states
                .get(*handle)
                .is_ok_and(|(k, _)| *k == PropKind::WindowInner)
        {
            if !states.get(link.glass).is_ok_and(|(_, s)| s.on) {
                continue; // vetro abbassato: gli scuri non si raggiungono
            }
            target = link.shutters;
        }
        let Ok((kind, mut st)) = states.get_mut(target) else {
            continue;
        };
        // Il quadro di rame si apre solo con la chiave di rame (QS-13).
        if *kind == PropKind::CopperPanel && !st.on {
            let key = crate::inventory::ItemKind::Relic(crate::inventory::Relic::CopperKey);
            if carried.qty(key) == 0 {
                picking.notify("Il quadro è chiuso: serve una chiave antica di rame");
                continue;
            }
            crate::karma::flag("quadro-rame");
            crate::sfx::play(crate::sfx::Sfx::Unlock);
            info!("[HUT] quadro di rame aperto con la chiave");
        }
        // Porta chiusa a chiave: senza accesso non si apre (chiudere è sempre possibile).
        if !st.on
            && let (Ok((lock, door, key)), Ok(player)) = (locks.get(target), players.single())
        {
            // La chiave di questa casa (Dita di Mercurio) vale quanto il passepartout.
            let master = carried.qty(crate::inventory::ItemKind::MasterKey) > 0
                || key.is_some_and(|k| picking.has_key(&k.0));
            let inside = player_inside(door, player.translation());
            if lock.access(clock.hour, master, inside) == Access::Locked {
                info!("[HUT] porta chiusa a chiave");
                continue;
            }
        }
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
#[allow(clippy::type_complexity)]
pub fn update_prop_prompts(
    states: Query<&PropState>,
    buttons: Query<&crate::village::GateButton>,
    mut handles: Query<(
        Entity,
        &PropKind,
        &PropState,
        &mut Interactable,
        Option<&WindowLink>,
        Option<(&Lock, &GlobalTransform)>,
    )>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    clock: Res<crate::daynight::GameClock>,
    inventory: Res<crate::inventory::Inventory>,
) {
    let master = inventory.qty(crate::inventory::ItemKind::MasterKey) > 0;
    let player = players.single().ok().map(|p| p.translation());
    for (e, kind, st, mut inter, link, lock) in &mut handles {
        // Un bottone mostra lo stato del suo cancello.
        let st_on = buttons
            .get(e)
            .ok()
            .and_then(|b| states.get(b.0).ok())
            .map_or(st.on, |s| s.on);
        let text = match (kind, st_on) {
            // I mobili hanno un testo fisso, scelto quando vengono creati.
            (PropKind::Container | PropKind::Pickup, _) => continue,
            (PropKind::Door, true) => "[E] Chiudi porta",
            (PropKind::Door, false) => "[E] Apri porta",
            (PropKind::Window, true) => "[E] Chiudi scuri",
            (PropKind::Window, false) => "[E] Apri scuri",
            (PropKind::Stove, true) => "[E] Spegni stufa",
            (PropKind::Stove, false) => "[E] Accendi stufa",
            (PropKind::Hearth, true) => "[E] Spegni il fuoco",
            (PropKind::Hearth, false) => "[E] Accendi il fuoco",
            (PropKind::CopperPanel, true) => "[E] Chiudi il quadro di rame",
            (PropKind::CopperPanel, false) => "[E] Apri il quadro di rame",
            (PropKind::Light, true) => "[E] Spegni luce",
            (PropKind::Light, false) => "[E] Accendi luce",
            (PropKind::Shutter, true) => "[E] Abbassa serranda",
            (PropKind::Shutter, false) => "[E] Alza serranda",
            (PropKind::Gate, true) => "[E] Chiudi cancello",
            (PropKind::Gate, false) => "[E] Apri cancello",
            (PropKind::WindowGlass, true) => "[E] Abbassa vetro",
            (PropKind::WindowGlass, false) => "[E] Alza vetro",
            (PropKind::WindowInner, _) => {
                let glass_up = link.is_some_and(|l| states.get(l.glass).is_ok_and(|s| s.on));
                let shutters_open =
                    link.is_some_and(|l| states.get(l.shutters).is_ok_and(|s| s.on));
                match (glass_up, shutters_open) {
                    (false, _) => "Alza il vetro per raggiungere gli scuri",
                    (true, true) => "[E] Chiudi scuri",
                    (true, false) => "[E] Apri scuri",
                }
            }
        };
        // Una porta chiusa dice se è chiusa a chiave o si apre solo col passepartout.
        let locked = lock.filter(|_| !st.on).and_then(|(l, door)| {
            let inside = player.is_some_and(|p| player_inside(door, p));
            match l.access(clock.hour, master, inside) {
                Access::Free => None,
                Access::Master => Some("[E] Apri porta (passepartout)".to_string()),
                Access::Locked => Some(l.closed_text()),
            }
        });
        let text = locked.as_deref().unwrap_or(text);
        if inter.prompt != text {
            inter.prompt = text.to_string();
        }
    }
}

/// Collider che blocca la vista solo in certi stati (serranda abbassata, scuri chiusi).
pub fn update_los_blockers(
    mut commands: Commands,
    states: Query<&PropState>,
    blockers: Query<(Entity, &LosBlocker, Has<ColliderDisabled>)>,
) {
    for (e, b, disabled) in &blockers {
        let Ok(st) = states.get(b.state) else {
            continue;
        };
        let active = st.on == b.active_when_on;
        if active && disabled {
            commands.entity(e).try_remove::<ColliderDisabled>();
        } else if !active && !disabled {
            commands.entity(e).try_insert(ColliderDisabled);
        }
    }
}

/// Anima porta e imposte; attiva/disattiva il collider del pannello della porta.
/// Ferme, non scrivono nulla: con centinaia di imposte e vetri nel paese ogni scrittura della
/// trasformazione costa una propagazione a ogni fotogramma (ADR 0056, ADR 0060).
pub fn animate_hinges(
    time: Res<Time>,
    mut commands: Commands,
    states: Query<&PropState>,
    mut hinges: Query<(&mut Hinge, &mut Transform, Option<&Children>)>,
    colliders: Query<Has<ColliderDisabled>, With<DoorCollider>>,
) {
    for (mut h, mut tf, children) in &mut hinges {
        let Ok(st) = states.get(h.state) else {
            continue;
        };
        let target = if st.on { h.open_angle } else { 0.0 };
        if h.angle != target {
            let step = 2.6 * time.delta_secs();
            h.angle += (target - h.angle).clamp(-step, step);
        }
        let rotation = h.base * Quat::from_rotation_y(h.angle);
        if tf.rotation != rotation {
            tf.rotation = rotation;
        }
        if let Some(children) = children {
            let blocking = !st.on && h.angle.abs() < 0.05;
            for child in children.iter() {
                let Ok(disabled) = colliders.get(child) else {
                    continue;
                };
                if blocking && disabled {
                    commands.entity(child).try_remove::<ColliderDisabled>();
                } else if !blocking && !disabled {
                    commands.entity(child).try_insert(ColliderDisabled);
                }
            }
        }
    }
}

/// Anima serrande e vetri (scorrimento verticale); fermi, non scrivono nulla.
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
        if sl.y != target {
            let step = 1.1 * time.delta_secs();
            sl.y += (target - sl.y).clamp(-step, step);
        }
        if tf.translation.y != sl.y {
            tf.translation.y = sl.y;
        }
    }
}

/// Luce di casa: lampada (emissivo e luce) e levetta seguono l'interruttore.
pub fn apply_house_light(
    switches: Query<(&PropKind, &PropId, &PropState)>,
    mut lights: Query<(&mut PointLight, &mut crate::graphics::ShadowWanted), With<HouseLampLight>>,
    lamps: Query<&MeshMaterial3d<StandardMaterial>, With<HouseLamp>>,
    mut levers: Query<&mut Transform, With<SwitchLever>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<crate::audio::AudioSettings>,
) {
    let Some(on) = switches
        .iter()
        .find(|(k, id, _)| **k == PropKind::Light && id.0 == "casetta:luce")
        .map(|(.., s)| s.on)
    else {
        return;
    };
    // Si scrive solo ciò che cambia: luce, materiale e levetta restano fermi finché
    // l'interruttore non scatta (prima si ricaricavano a ogni fotogramma, ADR 0056).
    let intensity = if on { 180_000.0 } else { 0.0 };
    for (mut l, mut want) in &mut lights {
        if l.intensity != intensity {
            l.intensity = intensity;
        }
        // La lampada spenta non disegna ombre; nemmeno senza le ombre delle luci piccole (ADR 0057).
        // Se la fa davvero lo decide `graphics::budget_point_shadows`.
        want.set(on && settings.gfx.small_shadows);
    }
    let emissive = if on {
        LinearRgba::new(1.4, 0.9, 0.35, 1.0)
    } else {
        LinearRgba::BLACK
    };
    for m in &lamps {
        if materials
            .get(&m.0)
            .is_some_and(|mat| mat.emissive != emissive)
            && let Some(mut mat) = materials.get_mut(&m.0)
        {
            mat.emissive = emissive;
        }
    }
    let rotation = Quat::from_rotation_x(if on { 0.5 } else { -0.5 });
    for mut tf in &mut levers {
        if tf.rotation != rotation {
            tf.rotation = rotation;
        }
    }
}

/// Bagliore tremolante della stufa accesa.
pub fn stove_light(
    time: Res<Time>,
    stoves: Query<(&PropKind, &PropState, &Children)>,
    mut lights: Query<(&mut PointLight, &mut crate::graphics::ShadowWanted), With<StoveLight>>,
    settings: Res<crate::audio::AudioSettings>,
) {
    let t = time.elapsed_secs();
    for (kind, st, children) in &stoves {
        if *kind != PropKind::Stove {
            continue;
        }
        for child in children.iter() {
            if let Ok((mut l, mut want)) = lights.get_mut(child) {
                let intensity = if st.on {
                    60_000.0 * (1.0 + 0.2 * (t * 9.0).sin() * (t * 3.7).cos())
                } else {
                    0.0
                };
                if l.intensity != intensity {
                    l.intensity = intensity;
                }
                // La stufa spenta non disegna ombre (ADR 0056), né senza le ombre piccole (ADR 0057).
                want.set(st.on && settings.gfx.small_shadows);
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
    puffs: Query<(), (With<SmokePuff>, Without<crate::village_life::TownPuff>)>,
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
            size: 1.0,
            grey: 0.75,
            alpha: 0.5,
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
        tf.scale = Vec3::splat((0.15 + 0.6 * k) * puff.size);
        // Il materiale torna alla GPU solo quando l'alfa si sposta di almeno un gradino a 8 bit
        // (1/255): prima ogni sbuffo si ricaricava a ogni fotogramma (ADR 0056).
        let alpha = puff.alpha * (1.0 - k);
        if materials.get(&mat.0).is_some_and(|m| {
            let c = m.base_color.to_srgba();
            c.red != puff.grey || (c.alpha - alpha).abs() >= 1.0 / 255.0
        }) && let Some(mut m) = materials.get_mut(&mat.0)
        {
            m.base_color = Color::srgba(puff.grey, puff.grey, puff.grey + 0.03, alpha);
        }
    }
}

// ----- Routine del Viandante (giorno, sera, notte, mattina) -----

/// Velocità di camminata (m/s) e altezza del centro del corpo in piedi.
const KEEPER_SPEED: f32 = 1.5;
/// Entro questa distanza (m) il giocatore è un cliente: il Viandante torna al banco.
const CUSTOMER_NEAR: f32 = 5.0;
const STAND_Y: f32 = Y0 + 0.85;
/// Punti (x, z locali) della routine.
const SHOP_XZ: Vec2 = Vec2::new(KEEPER_SHOP.x, KEEPER_SHOP.z);
const STOVE_STAND: Vec2 = Vec2::new(0.85, 2.45);
const DOOR_STAND: Vec2 = Vec2::new(-1.0, -2.3);
const SWITCH_STAND: Vec2 = Vec2::new(-0.55, -2.35);
/// Davanti al pulsante della serranda (parete +X, vicino allo spigolo), rivolto a +X.
const SHUTTER_STAND: Vec2 = Vec2::new(1.4, -2.6);
const BED_FOOT: Vec2 = Vec2::new(-1.3, 0.55);
const BED_LIE: Vec3 = Vec3::new(-1.35, Y0 + 0.46 + 0.35, 1.85);
/// Incrocio libero davanti al tavolo, da cui partono i percorsi.
const HUB: Vec2 = Vec2::new(0.8, -0.5);
/// Ritardo (s reali) prima che chiuda da solo porta/finestre aperte: 45 s – 3 h di gioco (1 h = 60 s).
const CLOSE_DELAY: (f32, f32) = (45.0, 180.0);
/// Di notte si sveglia entro 1 h di gioco (60 s reali) se qualcosa è aperto.
const NIGHT_DELAY: (f32, f32) = (10.0, 60.0);
/// Di notte, se la luce è accesa, si alza quasi subito (3–8 s reali).
const LIGHT_DELAY: (f32, f32) = (3.0, 8.0);

/// La stufa del Viandante è accesa 6–10 e 17–21, spenta nel resto della giornata.
fn stove_wanted(hour: f32, cold: bool) -> bool {
    // Sotto gli 8 °C resta accesa tutto il giorno.
    (6.0..10.0).contains(&hour)
        || (17.0..21.0).contains(&hour)
        || (cold && (6.0..21.0).contains(&hour))
}

/// Fase della giornata del negoziante.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KeeperPhase {
    /// Al banco.
    Day,
    /// Sequenza serale (stufa, serranda, luce, porta e finestre, letto).
    Evening,
    /// Dorme nel letto.
    Asleep,
    /// Sequenza mattutina (luce, porta e finestre, stufa, serranda, banco).
    Morning,
    /// Va ad accendere/spegnere la stufa o a chiudere porta/finestre durante il giorno.
    Task,
    /// Di notte si alza a chiudere ciò che è stato aperto e torna a letto.
    NightCheck,
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
    /// Chiude la porta.
    CloseDoor,
    /// Chiude la finestra `i` (indice in `WINDOWS`).
    CloseWindow(usize),
    /// Apre gli scuri della finestra `i`.
    OpenWindow(usize),
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
    /// Conti alla rovescia (s) per chiudere porta/finestre rimaste aperte: (maniglia, secondi).
    open_timers: Vec<(Entity, f32)>,
    /// Conto alla rovescia (s) per la ronda notturna, se qualcosa è aperto.
    night_timer: Option<f32>,
    /// Sta tornando al banco perché è arrivato un cliente.
    returning: bool,
    /// Prima esecuzione della routine fatta (scuri aperti se la partita inizia di giorno).
    started: bool,
    rng: u64,
    /// Moltiplicatore dei ritardi (solo debug: `PLANET_ONE_DELAY_SCALE`).
    delay_scale: f32,
}

impl Keeper {
    pub fn is_asleep(&self) -> bool {
        self.phase == KeeperPhase::Asleep
    }

    pub fn stove_manual(&self) -> bool {
        self.stove_manual
    }

    /// Stato letto da un salvataggio: a letto, oppure sveglio al banco.
    pub fn restore(
        &mut self,
        asleep: bool,
        stove_manual: bool,
        tf: &mut Transform,
        npc: &mut crate::entities::Npc,
    ) {
        self.stove_manual = stove_manual;
        self.steps.clear();
        if asleep {
            self.phase = KeeperPhase::Asleep;
            npc.mode = crate::entities::NpcMode::Asleep;
            tf.translation = BED_LIE;
            tf.rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        }
    }

    pub fn new() -> Self {
        // Seme dall'orologio di sistema (nessuna dipendenza esterna).
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x9E37_79B9_7F4A_7C15, |d| d.as_nanos() as u64)
            | 1;
        Self {
            phase: KeeperPhase::Day,
            steps: Default::default(),
            wait: 0.0,
            stove_manual: false,
            returning: false,
            started: false,
            open_timers: Vec::new(),
            night_timer: None,
            rng: seed,
            delay_scale: std::env::var("PLANET_ONE_DELAY_SCALE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
        }
    }

    /// Numero casuale in [lo, hi) (xorshift64).
    fn rand_range(&mut self, (lo, hi): (f32, f32)) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (lo + (self.rng >> 11) as f32 / (1u64 << 53) as f32 * (hi - lo)) * self.delay_scale
    }
}

/// Yaw locale per guardare in direzione `d` (x, z); il fronte è -Z.
fn yaw_towards(d: Vec2) -> f32 {
    (-d.x).atan2(-d.y)
}

type Steps = std::collections::VecDeque<Step>;

/// Posizioni da cui chiude ciascuna finestra (stesso ordine di `WINDOWS`):
/// percorso di avvicinamento, punto finale e direzione dello sguardo (verso la finestra).
fn window_approach(i: usize) -> (Vec<Vec2>, Vec2, Vec2) {
    match i {
        // Lato +X, accanto allo spigolo anteriore, a z≈-0,93.
        0 => (vec![HUB], Vec2::new(1.35, -0.93), Vec2::new(1.0, 0.0)),
        // Retro, tra il letto e lo scaffale.
        1 => (
            vec![HUB, Vec2::new(0.85, 2.0)],
            Vec2::new(-0.65, 2.35),
            Vec2::new(0.0, 1.0),
        ),
        // Lato -X, davanti al baule.
        _ => (
            vec![HUB, Vec2::new(-0.3, -1.2)],
            Vec2::new(-1.3, -1.6),
            Vec2::new(-1.0, 0.0),
        ),
    }
}

/// Passi per andare alla stufa, commutarla e (se richiesto) tornare al banco.
fn stove_trip(on: bool, back_to_shop: bool) -> Vec<Step> {
    use Step::*;
    let mut v = vec![
        Walk(HUB),
        Walk(STOVE_STAND),
        Face(yaw_towards(Vec2::new(1.0, 0.0))),
        Wait(0.6),
        Stove(on),
        Wait(0.5),
    ];
    if back_to_shop {
        v.extend([Walk(HUB), Walk(SHOP_XZ), Face(0.0)]);
    }
    v
}

/// Passi per chiudere la porta (se `door`) e le finestre indicate, partendo dall'incrocio.
fn close_trip(door: bool, windows: &[usize]) -> Vec<Step> {
    use Step::*;
    let mut v = Vec::new();
    if door {
        v.extend([
            Walk(HUB),
            Walk(DOOR_STAND),
            Face(0.0),
            Wait(0.3),
            CloseDoor,
            Wait(0.6),
        ]);
    }
    for &i in windows {
        let (path, stand, look) = window_approach(i);
        v.push(Walk(HUB));
        v.extend(path.into_iter().map(Walk));
        v.extend([
            Walk(stand),
            Face(yaw_towards(look)),
            Wait(0.3),
            CloseWindow(i),
            Wait(0.6),
        ]);
    }
    v
}

/// Sera: (spegne la stufa se è sua) → serranda giù → luce spenta → porta e finestre chiuse → a letto.
fn evening_steps(stove_to_off: bool, door_open: bool, windows_open: &[usize]) -> Steps {
    use Step::*;
    let mut v: Vec<Step> = Vec::new();
    if stove_to_off {
        v.extend(stove_trip(false, false));
    }
    v.extend([
        Walk(HUB),
        Walk(SHUTTER_STAND),
        Face(yaw_towards(Vec2::new(1.0, 0.0))),
        Wait(0.5),
        Shutter(false),
        Wait(1.2),
        Walk(SWITCH_STAND),
        Face(0.0),
        Wait(0.4),
        Light(false),
        Wait(0.5),
    ]);
    v.extend(close_trip(door_open, windows_open));
    v.extend([Walk(Vec2::new(-1.0, -0.6)), Walk(BED_FOOT), Lie]);
    v.into()
}

/// Passi per aprire gli scuri delle prime `count` finestre, partendo dall'incrocio.
fn open_trip(count: usize) -> Vec<Step> {
    use Step::*;
    let mut v = Vec::new();
    for i in 0..count {
        let (path, stand, look) = window_approach(i);
        v.push(Walk(HUB));
        v.extend(path.into_iter().map(Walk));
        v.extend([
            Walk(stand),
            Face(yaw_towards(look)),
            Wait(0.3),
            OpenWindow(i),
            Wait(0.6),
        ]);
    }
    v
}

/// Mattina: si alza → luce accesa → porta chiusa → scuri aperti → stufa accesa → serranda su → banco.
fn morning_steps(door_open: bool, windows: usize) -> Steps {
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
    ];
    v.extend(close_trip(door_open, &[]));
    v.extend(open_trip(windows));
    v.extend(stove_trip(true, false));
    v.extend([
        Walk(HUB),
        Walk(SHUTTER_STAND),
        Face(yaw_towards(Vec2::new(1.0, 0.0))),
        Wait(0.4),
        Shutter(true),
        Wait(1.2),
        Walk(SHOP_XZ),
        Face(0.0),
    ]);
    v.into()
}

/// Ronda notturna: si alza, chiude prima porta/finestre aperte, poi spegne la
/// luce (se accesa) e torna a dormire.
fn night_check_steps(door_open: bool, windows_open: &[usize], light_on: bool) -> Steps {
    use Step::*;
    let mut v: Vec<Step> = vec![Rise, Wait(0.5), Walk(Vec2::new(-1.0, -0.6))];
    v.extend(close_trip(door_open, windows_open));
    if light_on {
        v.extend([
            Walk(HUB),
            Walk(SWITCH_STAND),
            Face(0.0),
            Wait(0.4),
            Light(false),
            Wait(0.5),
        ]);
    }
    v.extend([Walk(Vec2::new(-1.0, -0.6)), Walk(BED_FOOT), Lie]);
    v.into()
}

/// Sistema: gestisce la giornata del Viandante (stufa a orari, chiusura di
/// porta/finestre lasciate aperte, sera, notte, mattina). Non parte mentre
/// c'è un dialogo aperto.
#[allow(clippy::too_many_arguments)]
pub fn keeper_routine(
    time: Res<Time>,
    clock: Res<crate::daynight::GameClock>,
    weather: Res<crate::weather::WeatherState>,
    dialog: Res<crate::entities::DialogState>,
    parts: Option<Res<HutParts>>,
    mut commands: Commands,
    mut keepers: Query<(
        Entity,
        &mut Keeper,
        &mut Transform,
        &GlobalTransform,
        &mut crate::entities::Npc,
    )>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    mut states: Query<&mut PropState>,
) {
    use crate::entities::NpcMode;
    let Some(parts) = parts else {
        return;
    };
    let dt = time.delta_secs();
    let hour = clock.hour;
    let night = !clock.awake();
    let cold = weather.temp < 8.0;
    // Stato corrente degli oggetti.
    let is_on = |states: &Query<&mut PropState>, e: Entity| states.get(e).is_ok_and(|s| s.on);
    let player = players.single().ok().map(|p| p.translation());
    for (entity, mut k, mut tf, global, mut npc) in &mut keepers {
        // Se la partita inizia di giorno gli scuri sono già aperti.
        if !k.started {
            k.started = true;
            if !night {
                for w in &parts.windows {
                    if let Ok(mut s) = states.get_mut(*w) {
                        s.on = true;
                    }
                }
            }
        }
        // Un cliente vicino: interrompe il giro e torna al banco, e non ne inizia altri.
        let customer = player.is_some_and(|p| p.distance(global.translation()) < CUSTOMER_NEAR);
        if customer && k.phase == KeeperPhase::Task && !k.returning {
            k.returning = true;
            k.steps = [Step::Walk(HUB), Step::Walk(SHOP_XZ), Step::Face(0.0)].into();
            info!("[KEEPER] torna al banco: c'è un cliente");
        }
        let stove_on = is_on(&states, parts.stove);
        let door_open = is_on(&states, parts.door);
        // Finestra "aperta" = scuri aperti oppure vetro alzato.
        let win_open = |states: &Query<&mut PropState>, i: usize| {
            is_on(states, parts.windows[i]) || parts.glass.get(i).is_some_and(|g| is_on(states, *g))
        };
        let windows_open: Vec<usize> = (0..parts.windows.len())
            .filter(|&i| win_open(&states, i))
            .collect();

        // Conti alla rovescia per porta e finestre aperte (si azzerano alla chiusura).
        // Di giorno gli scuri restano aperti: si controlla solo la porta.
        let watched = [(parts.door, door_open)];
        for (e, open) in &watched {
            let has = k.open_timers.iter().any(|(t, _)| t == e);
            if *open && !has {
                let delay = k.rand_range(CLOSE_DELAY);
                k.open_timers.push((*e, delay));
            } else if !*open && has {
                k.open_timers.retain(|(t, _)| t != e);
            }
        }
        // I conti scorrono solo mentre è al banco o sveglio (non durante le sequenze).
        if matches!(k.phase, KeeperPhase::Day) {
            for (_, t) in k.open_timers.iter_mut() {
                *t -= dt;
            }
        }
        // Ronda notturna: se dorme e qualcosa è aperto, parte un conto (max 1 h di gioco).
        let light_on = is_on(&states, parts.light);
        let something_open = door_open || !windows_open.is_empty() || light_on;
        if k.phase == KeeperPhase::Asleep {
            match (something_open, k.night_timer) {
                (true, None) => {
                    let t = k.rand_range(if light_on { LIGHT_DELAY } else { NIGHT_DELAY });
                    k.night_timer = Some(t);
                }
                (true, Some(t)) => {
                    let t = if light_on {
                        t.min(LIGHT_DELAY.1 * k.delay_scale)
                    } else {
                        t
                    };
                    k.night_timer = Some(t - dt);
                }
                (false, _) => k.night_timer = None,
            }
        } else {
            k.night_timer = None;
        }

        // Avvio delle sequenze.
        match k.phase {
            KeeperPhase::Day if !dialog.open && night => {
                // Spegne la stufa solo se l'aveva accesa lui (non toccata dal giocatore).
                k.steps = evening_steps(stove_on && !k.stove_manual, door_open, &windows_open);
                k.phase = KeeperPhase::Evening;
                npc.mode = NpcMode::Busy;
                commands.entity(entity).try_insert(ColliderDisabled);
                info!("[KEEPER] inizia la sera");
            }
            KeeperPhase::Day
                if !dialog.open
                    && !customer
                    && !k.stove_manual
                    && stove_on != stove_wanted(hour, cold) =>
            {
                k.steps = stove_trip(stove_wanted(hour, cold), true).into();
                k.phase = KeeperPhase::Task;
                npc.mode = NpcMode::Busy;
                commands.entity(entity).try_insert(ColliderDisabled);
                info!("[KEEPER] va alla stufa ({})", stove_wanted(hour, cold));
            }
            KeeperPhase::Day
                if !dialog.open && !customer && k.open_timers.iter().any(|(_, t)| *t <= 0.0) =>
            {
                // Chiude la porta rimasta aperta da troppo tempo.
                let mut v = close_trip(true, &[]);
                v.extend([Step::Walk(HUB), Step::Walk(SHOP_XZ), Step::Face(0.0)]);
                k.steps = v.into();
                k.phase = KeeperPhase::Task;
                npc.mode = NpcMode::Busy;
                commands.entity(entity).try_insert(ColliderDisabled);
                info!("[KEEPER] va a chiudere la porta");
            }
            KeeperPhase::Asleep if !night => {
                k.stove_manual = false;
                k.steps = morning_steps(door_open, parts.windows.len());
                k.phase = KeeperPhase::Morning;
                npc.mode = NpcMode::Busy;
                info!("[KEEPER] si sveglia");
            }
            KeeperPhase::Asleep if k.night_timer.is_some_and(|t| t <= 0.0) => {
                k.steps = night_check_steps(door_open, &windows_open, light_on);
                k.phase = KeeperPhase::NightCheck;
                k.night_timer = None;
                npc.mode = NpcMode::Busy;
                info!(
                    "[KEEPER] ronda notturna (porta: {door_open}, finestre: {windows_open:?}, luce: {light_on})"
                );
            }
            _ => {}
        }
        if !matches!(
            k.phase,
            KeeperPhase::Evening
                | KeeperPhase::Morning
                | KeeperPhase::Task
                | KeeperPhase::NightCheck
        ) {
            continue;
        }
        let Some(step) = k.steps.front().copied() else {
            // Sequenza finita.
            match k.phase {
                KeeperPhase::Evening | KeeperPhase::NightCheck => {
                    k.phase = KeeperPhase::Asleep;
                    npc.mode = NpcMode::Asleep;
                    info!("[KEEPER] dorme");
                }
                _ => {
                    k.phase = KeeperPhase::Day;
                    k.returning = false;
                    npc.mode = NpcMode::Ready;
                    commands.entity(entity).try_remove::<ColliderDisabled>();
                    info!("[KEEPER] al banco");
                }
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
            Step::CloseWindow(i) => {
                for e in [parts.windows.get(i), parts.glass.get(i)]
                    .into_iter()
                    .flatten()
                {
                    if let Ok(mut s) = states.get_mut(*e) {
                        s.on = false;
                    }
                }
                k.steps.pop_front();
            }
            Step::OpenWindow(i) => {
                if let Some(mut s) = parts.windows.get(i).and_then(|e| states.get_mut(*e).ok()) {
                    s.on = true;
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
