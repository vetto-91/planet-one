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

/// Quota più bassa del terreno tra centro e angoli dell'area: è la base degli edifici.
pub fn ground_under(field: &TerrainField, p: Vec2, half: Vec2) -> f32 {
    let (hx, hz) = (half.x, half.y);
    let heights = [(0.0, 0.0), (hx, hz), (-hx, hz), (hx, -hz), (-hx, -hz)]
        .map(|(dx, dz)| field.height_at(p.x + dx, p.y + dz));
    let low = heights.iter().copied().fold(f32::MAX, f32::min);
    let high = heights.iter().copied().fold(f32::MIN, f32::max);
    // Sul pendio la casa non resta mai sepolta di più di 35 cm dal lato alto.
    low.max(high - 0.35)
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
        let lowest = ground_under(&field, Vec2::new(p.x, p.z), snap.half);
        transform.translation.y = lowest + snap.offset;
        commands.entity(entity).remove::<SnapToGround>();
        debug!(
            "[PROPS] appoggiato al terreno a y={:.2}",
            transform.translation.y
        );
    }
}
