use bevy::prelude::*;

/// マーカーコンポーネント（物理半径は外部コンポーネントから同期）
#[derive(Component, Clone, Copy)]
pub struct Particle {
    pub radius: f32,
}

/// 粒子の遺伝情報と物理・交配パラメータ
#[derive(Component, Clone)]
pub struct ParticleGenome {
    /// 慣性および相互作用強度に影響する質量（0.2〜3.0程度を想定）
    pub mass: f32,
    /// 空気抵抗係数。大きいほど減速しやすい（0.3〜5.0程度）
    pub drag_coefficient: f32,
    /// 他個体へ放つシグナル（力の起点）
    pub signal_vector: Vec3,
    /// 受信時の反応ベクトル（力の向きや嗜好に影響）
    pub response_vector: Vec3,
    /// 交配成功率計算のパラメータ集合
    pub mate_kernel: MateKernelParams,
    /// 突然変異分布の幅や固定化確率を決める値
    pub mutation: MutationParams,
    /// 支配的な遺伝子寄与の重み（0.0〜1.0）
    pub dominance_bias: f32,
    /// 繁殖力（交配の重み付けや外観の輝度に影響）0.2〜2.5程度
    pub reproductive_strength: f32,
}

impl ParticleGenome {
    pub fn interaction_scalar(&self, other: &ParticleGenome) -> f32 {
        self.response_vector.dot(other.signal_vector)
    }
}

/// 派生済みの見た目・描画用パラメータ
#[derive(Component, Clone, Copy)]
pub struct ParticleAppearance {
    pub color: Color,
    pub size: f32,
    pub aspect_ratio: f32,
}

impl ParticleAppearance {
    pub fn from_genome(genome: &ParticleGenome) -> Self {
        let normalized_response = genome.response_vector.normalize_or_zero();
        let normalized_signal = genome.signal_vector.normalize_or_zero();

        let mut hue_direction = Vec2::ZERO;
        if normalized_response.length_squared() > 0.0 {
            hue_direction += normalized_response.xy();
        }
        if normalized_signal.length_squared() > 0.0 {
            hue_direction += normalized_signal.xy() * 0.6; // signalは方向に軽く寄与
        }

        let hue = if hue_direction.length_squared() > 0.0 {
            hue_direction
                .y
                .atan2(hue_direction.x)
                .to_degrees()
                .rem_euclid(360.0)
        } else {
            0.0
        };

        let kernel_weights = [
            genome.mate_kernel.distance_weight,
            genome.mate_kernel.similarity_weight,
            genome.mate_kernel.diversity_weight,
            genome.mate_kernel.energy_weight,
            genome.mate_kernel.crowding_weight,
        ];
        let weights_avg =
            kernel_weights.iter().map(|w| w.abs()).sum::<f32>() / kernel_weights.len() as f32;
        let weights_norm = ((weights_avg - 0.25) / 0.85).clamp(0.0, 1.0);
        let slope_norm = ((genome.mate_kernel.slope - 0.5) / 1.5).clamp(0.0, 1.0);
        let signal_alignment =
            ((normalized_signal.dot(normalized_response) + 1.0) * 0.5).clamp(0.0, 1.0);
        let saturation =
            (0.55 * weights_norm + 0.25 * slope_norm + 0.20 * signal_alignment).clamp(0.0, 1.0);

        let fertility_norm = ((genome.reproductive_strength - 0.5) / 1.0).clamp(0.0, 1.0);
        let response_norm = (normalized_response.z * 0.5 + 0.5).clamp(0.0, 1.0);
        let signal_norm = (normalized_signal.z * 0.5 + 0.5).clamp(0.0, 1.0);
        let lightness =
            (0.6 * fertility_norm + 0.25 * response_norm + 0.15 * signal_norm).clamp(0.0, 1.0);
        let color = Color::hsl(hue, saturation, lightness);

        let drag_norm = ((genome.drag_coefficient - 1.0) / 2.5).clamp(0.0, 1.0);
        let size = (10.0 - (10.0 - 2.0) * drag_norm).clamp(2.0, 10.0);

        let mutation_sum =
            (genome.mutation.sigma_base + genome.mutation.sigma_scale).clamp(0.05, 4.0);
        let mutation_norm = ((mutation_sum - 0.1) / 1.5).clamp(0.0, 1.0);
        let aspect_ratio = 0.5 + mutation_norm * (2.0 - 0.5);

        Self {
            color,
            size,
            aspect_ratio,
        }
    }

    pub fn sprite_extents(&self) -> Vec2 {
        let width = self.size;
        let height = self.size / self.aspect_ratio.max(0.25);
        Vec2::new(width, height)
    }

    pub fn collision_radius(&self) -> f32 {
        let extents = self.sprite_extents();
        0.25 * (extents.x + extents.y)
    }
}

/// 交配成功確率算出関数のパラメータ
#[derive(Component, Clone)]
pub struct MateKernelParams {
    pub bias: f32,
    pub distance_weight: f32,
    pub distance_scale: f32,
    pub energy_weight: f32,
    pub similarity_weight: f32,
    pub diversity_weight: f32,
    pub crowding_weight: f32,
    pub slope: f32,
}

/// 突然変異や継承アルゴリズム用パラメータ
#[derive(Component, Clone)]
pub struct MutationParams {
    pub sigma_base: f32,
    pub sigma_scale: f32,
    pub trait_lock_probability: f32,
}

/// 粒子の状態（寿命、クールダウンなど）
#[derive(Component, Clone)]
pub struct ParticleState {
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub distance_traveled: f32,
    pub offspring_count: u32,
    pub cooldown: f32,
}

impl ParticleState {
    pub fn energy_ratio(&self) -> f32 {
        (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
    }
}

/// 速度コンポーネント
#[derive(Component, Default, Clone, Copy)]
pub struct Velocity {
    pub value: Vec2,
}

/// 加速度コンポーネント
#[derive(Component, Default, Clone, Copy)]
pub struct Acceleration {
    pub value: Vec2,
}

/// 粒子の物理パラメータ（過去互換のため分離）
#[derive(Component, Clone, Copy)]
pub struct PhysicsParams {
    pub mass: f32,
    pub drag_coefficient: f32,
}

/// 近傍探索のための履歴
#[derive(Component, Default, Clone, Copy)]
pub struct KinematicsHistory {
    pub last_position: Vec2,
}

/// 空間グリッドセル内の粒子を追跡
#[allow(dead_code)]
#[derive(Component, Clone, Copy)]
pub struct GridCell {
    pub x: i32,
    pub y: i32,
}
