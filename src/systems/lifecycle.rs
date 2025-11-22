use bevy::prelude::*;

use crate::components::*;
use crate::resources::{EvolutionConfig, SimulationConfig, SimulationState};

/// 移動距離と時間経過による寿命の減衰を適用
pub fn update_lifetimes(
    time: Res<Time>,
    state: Res<SimulationState>,
    config: Res<EvolutionConfig>,
    sim_config: Res<SimulationConfig>,
    mut query: Query<(&Transform, &mut ParticleState, &mut KinematicsHistory)>,
) {
    // シミュレーションが停止中なら何もしない
    if *state == SimulationState::Paused {
        return;
    }
    let dt = time.delta_seconds();
    let half_world_width = sim_config.world_width * 0.5;
    let half_world_height = sim_config.world_height * 0.5;

    for (transform, mut state, mut history) in query.iter_mut() {
        let current_pos = transform.translation.truncate();
        let mut delta = current_pos - history.last_position;

        if delta.x > half_world_width {
            delta.x -= sim_config.world_width;
        } else if delta.x < -half_world_width {
            delta.x += sim_config.world_width;
        }

        if delta.y > half_world_height {
            delta.y -= sim_config.world_height;
        } else if delta.y < -half_world_height {
            delta.y += sim_config.world_height;
        }
        let distance = delta.length();

        if distance > f32::EPSILON {
            state.distance_traveled += distance;
            state.lifetime -= distance * config.life_loss_per_distance;
            history.last_position = current_pos;
        }

        state.lifetime -= dt * config.base_decay_per_second;
        if state.cooldown > 0.0 {
            state.cooldown = (state.cooldown - dt).max(0.0);
        }
    }
}

/// 寿命が切れた粒子を削除
pub fn despawn_dead_particles(mut commands: Commands, query: Query<(Entity, &ParticleState)>) {
    for (entity, state) in query.iter() {
        if state.lifetime <= 0.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}
