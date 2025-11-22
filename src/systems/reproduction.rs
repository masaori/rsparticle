use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy::utils::HashMap;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::components::*;
use crate::resources::{EvolutionConfig, SimulationAlgorithm, SimulationConfig, SimulationState, SpatialGrid};

#[derive(Clone)]
struct ParentSnapshot {
    entity: Entity,
    position: Vec2,
    genome: ParticleGenome,
    state: ParticleState,
    velocity: Vec2,
    radius: f32,
}

/// 交配と子粒子生成を試みる
pub fn attempt_mating(
    mut commands: Commands,
    state: Res<SimulationState>,
    algorithm: Res<SimulationAlgorithm>,
    sim_config: Res<SimulationConfig>,
    evolution: Res<EvolutionConfig>,
    spatial_grid: Res<SpatialGrid>,
    query: Query<(
        Entity,
        &Transform,
        &ParticleGenome,
        &ParticleState,
        &Velocity,
        &Particle,
    )>,
) {
    // シミュレーションが停止中なら何もしない
    if *state == SimulationState::Paused {
        return;
    }

    // PhysicsOnly モードでは交配を行わない
    if *algorithm == SimulationAlgorithm::PhysicsOnly {
        return;
    }

    if query.is_empty() {
        return;
    }

    // FastReproduction モードでは子供を増やす
    let max_children = match *algorithm {
        SimulationAlgorithm::FastReproduction => evolution.max_children_per_tick * 3,
        _ => evolution.max_children_per_tick,
    };

    let mut snapshots: HashMap<Entity, ParentSnapshot> = HashMap::new();
    for (entity, transform, genome, state, velocity, particle) in query.iter() {
        snapshots.insert(
            entity,
            ParentSnapshot {
                entity,
                position: transform.translation.truncate(),
                genome: genome.clone(),
                state: state.clone(),
                velocity: velocity.value,
                radius: particle.radius.max(1.0),
            },
        );
    }

    let mut rng = rand::thread_rng();
    let mut updated_states: HashMap<Entity, ParticleState> = HashMap::new();
    let mut spawned_children = 0usize;

    for snapshot in snapshots.values() {
        if spawned_children >= max_children {
            break;
        }

        let current_state = updated_states
            .get(&snapshot.entity)
            .cloned()
            .unwrap_or_else(|| snapshot.state.clone());

        if current_state.cooldown > 0.0 || current_state.lifetime <= evolution.reproduction_cost {
            continue;
        }

        let cell = spatial_grid.get_cell(snapshot.position.x, snapshot.position.y);
        let neighbors = spatial_grid.get_neighbors(cell);

        let mut eligible_neighbors: Vec<ParentSnapshot> = Vec::new();
        for entity in neighbors {
            if entity == snapshot.entity {
                continue;
            }
            if let Some(candidate) = snapshots.get(&entity) {
                let candidate_state = updated_states
                    .get(&entity)
                    .cloned()
                    .unwrap_or_else(|| candidate.state.clone());
                if candidate_state.cooldown > 0.0
                    || candidate_state.lifetime <= evolution.reproduction_cost
                {
                    continue;
                }
                let effective_radius = size_scaled_radius(
                    snapshot.radius,
                    candidate.radius,
                    evolution.mating_radius_ratio,
                );
                if snapshot.position.distance(candidate.position) > effective_radius {
                    continue;
                }
                let mut with_state = candidate.clone();
                with_state.state = candidate_state;
                eligible_neighbors.push(with_state);
            }
        }

        let min_required = evolution.min_parents.max(1);

        let available_count = eligible_neighbors.len() + 1;
        if available_count < min_required {
            continue;
        }

        let max_parents = evolution.max_parents.max(min_required).min(available_count);
        let parent_count = if min_required == max_parents {
            max_parents
        } else {
            rng.gen_range(min_required..=max_parents)
        };

        eligible_neighbors.shuffle(&mut rng);
        let mut parents = Vec::with_capacity(parent_count);
        let mut initiator = snapshot.clone();
        initiator.state = current_state;
        parents.push(initiator);
        parents.extend(eligible_neighbors.into_iter().take(parent_count - 1));

        let kernel = aggregate_kernel(&parents);
        let success_prob =
            mating_success_probability(&parents, &kernel, evolution.distance_weight_scale);
        if rng.gen::<f32>() > success_prob {
            continue;
        }

        spawned_children += 1;

        for parent in &parents {
            let mut updated = parent.state.clone();
            updated.lifetime = (updated.lifetime - evolution.reproduction_cost).max(0.0);
            updated.cooldown = evolution.cooldown_base;
            updated.offspring_count = updated.offspring_count.saturating_add(1);
            updated_states.insert(parent.entity, updated);
        }

        let child_genome = synthesize_child_genome(&parents, &evolution, &mut rng);
        let appearance = ParticleAppearance::from_genome(&child_genome);
        let sprite_size = appearance.sprite_extents();
        let color = appearance.color;
        let radius = appearance.collision_radius();
        let weights = normalized_weights(&parents);
        let max_lifetime = weighted_sum(&parents, &weights, |p| p.state.max_lifetime).max(20.0);
        let lifetime = (max_lifetime * rng.gen_range(0.4..0.8)).clamp(5.0, max_lifetime);
        let child_state = ParticleState {
            lifetime,
            max_lifetime,
            distance_traveled: 0.0,
            offspring_count: 0,
            cooldown: evolution.cooldown_base * 0.5,
        };
        let avg_radius = average_radius(&parents);
        let mut position = weighted_vec2(&parents, &weights, |p| p.position)
            + random_offset(
                &mut rng,
                avg_radius * evolution.mating_radius_ratio.max(0.1) * 0.2,
            );
        position.x = position.x.rem_euclid(sim_config.world_width);
        position.y = position.y.rem_euclid(sim_config.world_height);

        let velocity =
            weighted_vec2(&parents, &weights, |p| p.velocity) + random_offset(&mut rng, 10.0);

        let mass = child_genome.mass;
        let drag = child_genome.drag_coefficient;

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(sprite_size),
                    ..default()
                },
                transform: Transform::from_xyz(position.x, position.y, 0.0),
                ..default()
            },
            Particle { radius },
            child_genome,
            appearance,
            child_state,
            Velocity { value: velocity },
            Acceleration::default(),
            PhysicsParams {
                mass,
                drag_coefficient: drag,
            },
            KinematicsHistory {
                last_position: position,
            },
        ));
    }

    for (entity, state) in updated_states {
        commands.entity(entity).insert(state);
    }
}

fn aggregate_kernel(parents: &[ParentSnapshot]) -> MateKernelParams {
    let inv = 1.0 / parents.len() as f32;
    let mut kernel = MateKernelParams {
        bias: 0.0,
        distance_weight: 0.0,
        distance_scale: 0.0,
        energy_weight: 0.0,
        similarity_weight: 0.0,
        diversity_weight: 0.0,
        crowding_weight: 0.0,
        slope: 0.0,
    };

    for parent in parents {
        kernel.bias += parent.genome.mate_kernel.bias * inv;
        kernel.distance_weight += parent.genome.mate_kernel.distance_weight * inv;
        kernel.distance_scale += parent.genome.mate_kernel.distance_scale * inv;
        kernel.energy_weight += parent.genome.mate_kernel.energy_weight * inv;
        kernel.similarity_weight += parent.genome.mate_kernel.similarity_weight * inv;
        kernel.diversity_weight += parent.genome.mate_kernel.diversity_weight * inv;
        kernel.crowding_weight += parent.genome.mate_kernel.crowding_weight * inv;
        kernel.slope += parent.genome.mate_kernel.slope * inv;
    }

    kernel.distance_scale = kernel.distance_scale.max(1.0);
    kernel.slope = kernel.slope.max(0.1);
    kernel
}

fn mating_success_probability(
    parents: &[ParentSnapshot],
    kernel: &MateKernelParams,
    distance_weight_scale: f32,
) -> f32 {
    let n = parents.len();
    if n == 0 {
        return 0.0;
    }

    let mut total_distance = 0.0;
    let mut total_similarity = 0.0;
    let mut pair_count = 0;
    for i in 0..n {
        for j in (i + 1)..n {
            total_distance += parents[i].position.distance(parents[j].position);
            total_similarity += parents[i]
                .genome
                .response_vector
                .dot(parents[j].genome.response_vector);
            pair_count += 1;
        }
    }
    let avg_distance = if pair_count > 0 {
        total_distance / pair_count as f32
    } else {
        0.0
    };
    let avg_similarity = if pair_count > 0 {
        total_similarity / pair_count as f32
    } else {
        0.0
    };

    let avg_energy = parents.iter().map(|p| p.state.energy_ratio()).sum::<f32>() / n as f32;

    let diversity = affinity_variance(parents);
    let crowding = n as f32;

    let distance_term = (-avg_distance / kernel.distance_scale).exp();
    let scaled_distance_weight = kernel.distance_weight * distance_weight_scale.max(0.1);

    let mut score = kernel.bias;
    score += scaled_distance_weight * distance_term;
    score += kernel.energy_weight * (avg_energy - 0.5);
    score += kernel.similarity_weight * avg_similarity;
    score += kernel.diversity_weight * diversity;
    score -= kernel.crowding_weight * crowding;

    1.0 / (1.0 + (-kernel.slope * score).exp())
}

fn affinity_variance(parents: &[ParentSnapshot]) -> f32 {
    if parents.is_empty() {
        return 0.0;
    }

    let mean = parents
        .iter()
        .fold(Vec3::ZERO, |acc, p| acc + p.genome.response_vector)
        / parents.len() as f32;

    let variance = parents
        .iter()
        .map(|p| (p.genome.response_vector - mean).length_squared())
        .sum::<f32>()
        / parents.len() as f32;

    variance.sqrt()
}

fn synthesize_child_genome(
    parents: &[ParentSnapshot],
    evolution: &EvolutionConfig,
    rng: &mut rand::rngs::ThreadRng,
) -> ParticleGenome {
    let avg_radius = average_radius(parents).max(1.0);
    let weights = normalized_weights(parents);
    let lock_probability = parents
        .iter()
        .map(|p| p.genome.mutation.trait_lock_probability)
        .sum::<f32>()
        / parents.len() as f32;
    let sigma_base = parents
        .iter()
        .map(|p| p.genome.mutation.sigma_base)
        .sum::<f32>()
        / parents.len() as f32;
    let sigma_scale = parents
        .iter()
        .map(|p| p.genome.mutation.sigma_scale)
        .sum::<f32>()
        / parents.len() as f32;

    let sigma_base = sigma_base.max(evolution.mutation_sigma_base * 0.5);
    let sigma_scale = sigma_scale.max(evolution.mutation_sigma_scale * 0.5);

    let mass = mix_scalar(
        parents,
        &weights,
        lock_probability,
        sigma_base,
        sigma_scale,
        |g| g.mass,
        0.2,
        3.0,
        rng,
    );
    let drag = mix_scalar(
        parents,
        &weights,
        lock_probability,
        sigma_base,
        sigma_scale,
        |g| g.drag_coefficient,
        0.3,
        5.0,
        rng,
    );

    let signal_vector = mix_vec3(
        parents,
        &weights,
        lock_probability,
        sigma_base,
        sigma_scale,
        rng,
        |g| g.signal_vector,
    )
    .normalize_or_zero();

    let response_vector = mix_vec3(
        parents,
        &weights,
        lock_probability,
        sigma_base,
        sigma_scale,
        rng,
        |g| g.response_vector,
    )
    .normalize_or_zero();

    let mate_kernel = MateKernelParams {
        bias: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.mate_kernel.bias,
            -2.0,
            2.0,
            rng,
        ),
        distance_weight: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.mate_kernel.distance_weight,
            0.1,
            3.0,
            rng,
        ),
        distance_scale: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.mate_kernel.distance_scale,
            (avg_radius * evolution.mating_radius_ratio * 0.5).max(1.0),
            (avg_radius * evolution.mating_radius_ratio * 2.5).max(2.0),
            rng,
        )
        .max(1.0),
        energy_weight: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.mate_kernel.energy_weight,
            -3.0,
            3.0,
            rng,
        ),
        similarity_weight: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.mate_kernel.similarity_weight,
            -3.0,
            3.0,
            rng,
        ),
        diversity_weight: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.mate_kernel.diversity_weight,
            0.0,
            3.0,
            rng,
        ),
        crowding_weight: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.mate_kernel.crowding_weight,
            0.0,
            3.0,
            rng,
        ),
        slope: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.mate_kernel.slope,
            0.2,
            4.0,
            rng,
        ),
    };

    let mutation = MutationParams {
        sigma_base: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.mutation.sigma_base,
            0.01,
            1.0,
            rng,
        ),
        sigma_scale: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.mutation.sigma_scale,
            0.01,
            1.0,
            rng,
        ),
        trait_lock_probability: (lock_probability + rand_noise(rng, sigma_base)).clamp(0.0, 1.0),
    };

    ParticleGenome {
        mass,
        drag_coefficient: drag,
        signal_vector,
        response_vector,
        mate_kernel,
        mutation,
        dominance_bias: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.dominance_bias,
            0.0,
            1.0,
            rng,
        ),
        reproductive_strength: mix_scalar(
            parents,
            &weights,
            lock_probability,
            sigma_base,
            sigma_scale,
            |g| g.reproductive_strength,
            0.2,
            2.5,
            rng,
        ),
    }
}

fn average_radius(parents: &[ParentSnapshot]) -> f32 {
    if parents.is_empty() {
        return 1.0;
    }
    parents.iter().map(|p| p.radius).sum::<f32>() / parents.len() as f32
}

fn size_scaled_radius(a: f32, b: f32, ratio: f32) -> f32 {
    let avg = ((a + b) * 0.5).max(1.0);
    (avg * ratio.max(0.1)).max(1.0)
}

fn normalized_weights(parents: &[ParentSnapshot]) -> Vec<f32> {
    let mut weights: Vec<f32> = parents
        .iter()
        .map(|p| (p.genome.reproductive_strength + p.genome.dominance_bias.max(0.01)).max(0.05))
        .collect();
    let sum: f32 = weights.iter().sum();
    if sum > 0.0 {
        for w in &mut weights {
            *w /= sum;
        }
    } else {
        let uniform = 1.0 / weights.len() as f32;
        for w in &mut weights {
            *w = uniform;
        }
    }
    weights
}

fn weighted_sum<F>(parents: &[ParentSnapshot], weights: &[f32], getter: F) -> f32
where
    F: Fn(&ParentSnapshot) -> f32,
{
    parents
        .iter()
        .zip(weights.iter())
        .map(|(parent, weight)| getter(parent) * *weight)
        .sum()
}

fn weighted_vec2<F>(parents: &[ParentSnapshot], weights: &[f32], getter: F) -> Vec2
where
    F: Fn(&ParentSnapshot) -> Vec2,
{
    parents
        .iter()
        .zip(weights.iter())
        .fold(Vec2::ZERO, |acc, (parent, weight)| {
            acc + getter(parent) * *weight
        })
}

fn mix_scalar<F>(
    parents: &[ParentSnapshot],
    weights: &[f32],
    lock_prob: f32,
    sigma_base: f32,
    sigma_scale: f32,
    accessor: F,
    min_value: f32,
    max_value: f32,
    rng: &mut rand::rngs::ThreadRng,
) -> f32
where
    F: Fn(&ParticleGenome) -> f32,
{
    let values: Vec<f32> = parents.iter().map(|p| accessor(&p.genome)).collect();
    if rng.gen::<f32>() < lock_prob {
        let idx = pick_parent(weights, rng);
        return values[idx].clamp(min_value, max_value);
    }

    let mean = values
        .iter()
        .zip(weights.iter())
        .map(|(value, weight)| *value * *weight)
        .sum::<f32>();

    let avg = values.iter().copied().sum::<f32>() / values.len() as f32;
    let variance = values.iter().map(|v| (v - avg).powi(2)).sum::<f32>() / values.len() as f32;
    let sigma = sigma_base + sigma_scale * variance.sqrt();
    let noisy = mean + rand_noise(rng, sigma);
    noisy.clamp(min_value, max_value)
}

fn mix_vec3<F>(
    parents: &[ParentSnapshot],
    weights: &[f32],
    lock_prob: f32,
    sigma_base: f32,
    sigma_scale: f32,
    rng: &mut rand::rngs::ThreadRng,
    accessor: F,
) -> Vec3
where
    F: Fn(&ParticleGenome) -> Vec3,
{
    let values: Vec<Vec3> = parents.iter().map(|p| accessor(&p.genome)).collect();
    if rng.gen::<f32>() < lock_prob {
        let idx = pick_parent(weights, rng);
        return values[idx];
    }

    let mean = values
        .iter()
        .zip(weights.iter())
        .fold(Vec3::ZERO, |acc, (value, weight)| acc + *value * *weight);

    let avg = values.iter().fold(Vec3::ZERO, |acc, value| acc + *value) / values.len() as f32;
    let variance = values
        .iter()
        .map(|value| (*value - avg).length_squared())
        .sum::<f32>()
        / values.len() as f32;
    let sigma = sigma_base + sigma_scale * variance.sqrt();
    mean + random_offset_vec3(rng, sigma)
}

fn pick_parent(weights: &[f32], rng: &mut rand::rngs::ThreadRng) -> usize {
    let mut roll = rng.gen::<f32>();
    for (idx, weight) in weights.iter().enumerate() {
        roll -= *weight;
        if roll <= 0.0 {
            return idx;
        }
    }
    weights.len().saturating_sub(1)
}

fn rand_noise(rng: &mut rand::rngs::ThreadRng, scale: f32) -> f32 {
    if scale <= 0.0 {
        return 0.0;
    }
    rng.gen_range(-scale..scale)
}

fn random_offset(rng: &mut rand::rngs::ThreadRng, max_radius: f32) -> Vec2 {
    if max_radius <= 0.0 {
        return Vec2::ZERO;
    }
    let angle = rng.gen_range(0.0..TAU);
    let radius = rng.gen_range(0.0..max_radius);
    Vec2::new(angle.cos(), angle.sin()) * radius
}

fn random_offset_vec3(rng: &mut rand::rngs::ThreadRng, scale: f32) -> Vec3 {
    if scale <= 0.0 {
        return Vec3::ZERO;
    }
    Vec3::new(
        rng.gen_range(-scale..scale),
        rng.gen_range(-scale..scale),
        rng.gen_range(-scale..scale),
    )
}
