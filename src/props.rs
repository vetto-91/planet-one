//! Appoggio a terra degli oggetti di scena (la casetta è in `hut.rs`).

use crate::scene::Ground;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Appoggia l'entità al terreno sotto di lei (raycast verso il basso sui
/// soli collider `Ground`). `offset` si somma alla quota trovata.
#[derive(Component)]
pub struct SnapToGround {
    pub offset: f32,
    /// Semi-lati (m, x e z) dell'area da campionare: si usa la quota più bassa.
    pub half: Vec2,
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
        let (hx, hz) = (snap.half.x, snap.half.y);
        let mut lowest: Option<f32> = None;
        for (dx, dz) in [(0.0, 0.0), (hx, hz), (-hx, hz), (hx, -hz), (-hx, -hz)] {
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
