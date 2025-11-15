use bevy::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;

mod components;
mod resources;
mod systems;

use resources::*;
use systems::physics::*;
use systems::spatial::*;
use systems::setup::*;

fn main() {
    let config = SimulationConfig::default();
    let cell_size = config.interaction_radius / 2.0;  // セルサイズを相互作用半径の半分に
    
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "2D Particle Simulator (Bevy)".to_string(),
                    resolution: (config.world_width, config.world_height).into(),
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
        ))
        .init_resource::<ParticleTypeRegistry>()
        .insert_resource(config.clone())
        .insert_resource(InteractionMatrix::new(3))
        .insert_resource(SpatialGrid::new(config.world_width, config.world_height, cell_size))
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            Update,
            (
                update_positions,
                apply_boundary_wrap,
                update_spatial_grid,
                calculate_accelerations,
                update_velocities,
                update_ui,
            )
                .chain(),
        )
        .run();
}

impl Default for ParticleTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}