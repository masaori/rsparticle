use bevy::prelude::*;

use crate::resources::{FrameDelta, SimulationConfig};

/// delta_seconds() を一度だけ評価し、物理系とロジック系で共有する
pub fn update_frame_delta(
    time: Res<Time>,
    config: Res<SimulationConfig>,
    mut frame_delta: ResMut<FrameDelta>,
) {
    let raw_dt = time.delta_seconds();
    let mut dt = raw_dt.min(config.max_delta_seconds);

    if !dt.is_finite() || dt <= f32::EPSILON {
        dt = config.target_delta_seconds;
    }

    frame_delta.raw = raw_dt;
    frame_delta.dt = dt;
    frame_delta.dt_sq = dt * dt;
    frame_delta.target = config.target_delta_seconds;
}
