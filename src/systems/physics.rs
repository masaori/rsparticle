use bevy::prelude::*;
use rayon::prelude::*;
use crate::components::*;
use crate::resources::*;

/// 位置を更新（Velocity Verlet 積分の前半）
pub fn update_positions(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Velocity, &Acceleration), With<Particle>>,
) {
    let dt = time.delta_seconds();
    let dt_sq = dt * dt;
    let half_dt_sq = 0.5 * dt_sq;  // 事前計算

    for (mut transform, velocity, acceleration) in query.iter_mut() {
        // p(t+dt) = p(t) + v(t)*dt + 0.5*a(t)*dt^2
        transform.translation.x += velocity.value.x * dt + acceleration.value.x * half_dt_sq;
        transform.translation.y += velocity.value.y * dt + acceleration.value.y * half_dt_sq;
    }
}

/// 境界でのラップアラウンド処理
pub fn apply_boundary_wrap(
    config: Res<SimulationConfig>,
    mut query: Query<&mut Transform, With<Particle>>,
) {
    for mut transform in query.iter_mut() {
        transform.translation.x = transform.translation.x.rem_euclid(config.world_width);
        transform.translation.y = transform.translation.y.rem_euclid(config.world_height);
    }
}

/// 加速度を計算（並列化 + 最適化版）
pub fn calculate_accelerations(
    config: Res<SimulationConfig>,
    interaction_matrix: Res<InteractionMatrix>,
    spatial_grid: Res<SpatialGrid>,
    mut query: Query<(Entity, &Transform, &Particle, &PhysicsParams, &mut Acceleration)>,
) {
    let softening_sq = config.softening_epsilon * config.softening_epsilon;
    let interaction_radius_sq = config.interaction_radius * config.interaction_radius;
    let max_acc_sq = config.max_acceleration * config.max_acceleration;
    let half_world_width = config.world_width * 0.5;
    let half_world_height = config.world_height * 0.5;

    // エンティティのデータをHashMapに収集（O(1)でアクセス可能に）
    use bevy::utils::HashMap;
    let particles: HashMap<Entity, (Vec3, usize, f32, f32)> = query
        .iter()
        .map(|(entity, transform, particle, physics, _)| {
            (entity, (transform.translation, particle.particle_type, physics.mass, particle.radius))
        })
        .collect();

    // エンティティリストをVecに収集（並列処理用）
    let entities: Vec<Entity> = particles.keys().copied().collect();

    // 並列計算で各粒子の加速度を計算
    let accelerations: Vec<(Entity, Vec2)> = entities
        .par_iter()
        .map(|&entity_i| {
            let (pos_i, type_i, mass_i, radius_i) = particles[&entity_i];
            let cell = spatial_grid.get_cell(pos_i.x, pos_i.y);
            let neighbors = spatial_grid.get_neighbors(cell);

            let mut total_acceleration = Vec2::ZERO;

            for &entity_j in &neighbors {
                if entity_i == entity_j {
                    continue;
                }

                if let Some((pos_j, type_j, mass_j, radius_j)) = particles.get(&entity_j) {
                    // 周期境界条件を考慮した距離ベクトルの計算（最適化版）
                    let mut dx = pos_j.x - pos_i.x;
                    let mut dy = pos_j.y - pos_i.y;

                    // 分岐を減らす
                    if dx > half_world_width {
                        dx -= config.world_width;
                    } else if dx < -half_world_width {
                        dx += config.world_width;
                    }
                    if dy > half_world_height {
                        dy -= config.world_height;
                    } else if dy < -half_world_height {
                        dy += config.world_height;
                    }

                    let dist_sq = dx * dx + dy * dy;

                    // 早期リターン
                    if dist_sq > interaction_radius_sq || dist_sq < 1e-10 {
                        continue;
                    }

                    let dist = dist_sq.sqrt();
                    let min_dist = radius_i + radius_j;

                    // 力の方向（正規化） - dx, dy は j から i への方向
                    let inv_dist = 1.0 / dist;
                    let norm_dx = dx * inv_dist;
                    let norm_dy = dy * inv_dist;

                    let inv_mass = 1.0 / mass_i;

                    // 衝突判定: 粒子が重なっている場合は強い反発力を加える
                    let (acc_x, acc_y) = if dist < min_dist {
                        // 衝突時の反発加速度（ハードコア反発）
                        // 反発力は i から j を押し出す方向（-dx, -dy方向）
                        let overlap = min_dist - dist;
                        let stiffness = 2000.0; // 反発の強さ（加速度ベース）
                        let repulsion_acc = stiffness * overlap / min_dist; // 正規化
                        (-norm_dx * repulsion_acc, -norm_dy * repulsion_acc)
                    } else {
                        // 通常の相互作用力（引力/斥力）
                        let softened_dist_sq = dist_sq + softening_sq;
                        let g_ij = interaction_matrix.get(type_i, *type_j);
                        let force_mag = g_ij * mass_i * mass_j / softened_dist_sq;
                        let force_x = norm_dx * force_mag;
                        let force_y = norm_dy * force_mag;
                        
                        let mut acc_x = force_x * inv_mass;
                        let mut acc_y = force_y * inv_mass;

                        // 通常の力のみ加速度の上限チェック
                        let acc_sq = acc_x * acc_x + acc_y * acc_y;
                        if acc_sq > max_acc_sq {
                            let scale = config.max_acceleration / acc_sq.sqrt();
                            acc_x *= scale;
                            acc_y *= scale;
                        }
                        
                        (acc_x, acc_y)
                    };

                    total_acceleration.x += acc_x;
                    total_acceleration.y += acc_y;
                }
            }

            (entity_i, total_acceleration)
        })
        .collect();
    
    // VecからHashMapに変換
    let accelerations_map: HashMap<Entity, Vec2> = accelerations.into_iter().collect();

    // 計算した加速度を適用
    for (entity, _, _, _, mut acceleration) in query.iter_mut() {
        acceleration.value = *accelerations_map.get(&entity).unwrap_or(&Vec2::ZERO);
    }
}

/// 速度を更新（Velocity Verlet 積分の後半）
pub fn update_velocities(
    time: Res<Time>,
    mut query: Query<(&mut Velocity, &Acceleration, &PhysicsParams), With<Particle>>,
) {
    let dt = time.delta_seconds();

    for (mut velocity, acceleration, physics) in query.iter_mut() {
        // v(t+dt) = v(t) + a(t+dt)*dt
        velocity.value.x += acceleration.value.x * dt;
        velocity.value.y += acceleration.value.y * dt;

        // 抵抗による減衰（事前計算して再利用）
        let damping_factor = (-physics.drag_coefficient * dt / physics.mass).exp();
        velocity.value.x *= damping_factor;
        velocity.value.y *= damping_factor;
    }
}
