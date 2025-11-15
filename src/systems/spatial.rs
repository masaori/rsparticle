use bevy::prelude::*;
use crate::components::*;
use crate::resources::*;

/// 空間グリッドを更新するシステム
pub fn update_spatial_grid(
    mut spatial_grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &Transform), With<Particle>>,
) {
    // グリッドをクリアするが、容量は保持
    for (_, entities) in spatial_grid.grid.iter_mut() {
        entities.clear();
    }

    for (entity, transform) in query.iter() {
        let pos = transform.translation;
        let cell = spatial_grid.get_cell(pos.x, pos.y);
        spatial_grid.insert(cell, entity);
    }
}
