use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

mod components;
mod resources;
mod systems;

use resources::*;
use systems::lifecycle::*;
use systems::physics::*;
use systems::reproduction::*;
use systems::setup::*;
use systems::spatial::*;

fn main() {
    let config = SimulationConfig::default();
    let cell_size = config.interaction_radius / 2.0; // セルサイズを相互作用半径の半分に

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
        .insert_resource(config.clone())
        .insert_resource(EvolutionConfig::default())
        .insert_resource(SpatialGrid::new(
            config.world_width,
            config.world_height,
            cell_size,
            config.interaction_radius,
        ))
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            Update,
            (
                update_positions,
                apply_boundary_wrap,
                update_spatial_grid,
                calculate_accelerations,
                update_velocities,
                update_lifetimes,
                attempt_mating,
                despawn_dead_particles,
                update_ui,
            )
                .chain(),
        )
        .run();
}
