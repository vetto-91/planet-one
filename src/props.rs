//! Appoggio a terra degli oggetti di scena (la casetta è in `hut.rs`).

use crate::terrain::TerrainField;
use bevy::prelude::*;

/// Appoggia l'entità al terreno sotto di lei (raycast verso il basso sui
/// soli collider `Ground`). `offset` si somma alla quota trovata.
#[derive(Component)]
pub struct SnapToGround {
    pub offset: f32,
    /// Semi-lati (m, x e z) dell'area da campionare: si usa la quota più bassa.
    pub half: Vec2,
}

/// Sistema: porta a terra le entità con [`SnapToGround`] usando il campo di altezze del
/// terreno (esatto, indipendente dalla fisica). Riprova ogni frame finché il terreno
/// non è stato generato: prima c'è solo il segnaposto piatto a y = 0,5.
pub fn snap_to_ground(
    mut commands: Commands,
    field: Option<Res<TerrainField>>,
    mut items: Query<(Entity, &mut Transform, &SnapToGround)>,
) {
    let Some(field) = field else {
        return;
    };
    for (entity, mut transform, snap) in &mut items {
        let p = transform.translation;
        let (hx, hz) = (snap.half.x, snap.half.y);
        let lowest = [(0.0, 0.0), (hx, hz), (-hx, hz), (hx, -hz), (-hx, -hz)]
            .iter()
            .map(|(dx, dz)| field.height_at(p.x + dx, p.z + dz))
            .fold(f32::MAX, f32::min);
        transform.translation.y = lowest + snap.offset;
        commands.entity(entity).remove::<SnapToGround>();
        debug!(
            "[PROPS] appoggiato al terreno a y={:.2}",
            transform.translation.y
        );
    }
}
