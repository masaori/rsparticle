use bevy::prelude::*;

/// 粒子のコンポーネント
#[derive(Component)]
pub struct Particle {
    pub particle_type: usize,
    pub radius: f32,
}

/// 速度コンポーネント
#[derive(Component, Default)]
pub struct Velocity {
    pub value: Vec2,
}

/// 加速度コンポーネント
#[derive(Component, Default)]
pub struct Acceleration {
    pub value: Vec2,
}

/// 粒子の物理パラメータ
#[derive(Component)]
pub struct PhysicsParams {
    pub mass: f32,
    pub drag_coefficient: f32,
}

/// 粒子種類の定義
#[derive(Clone, Debug)]
pub struct ParticleTypeInfo {
    #[allow(dead_code)]
    pub type_id: usize,
    pub color: Color,
    pub mass: f32,
    pub drag_coefficient: f32,
    pub radius: f32,
}

/// 空間グリッドセル内の粒子を追跡
#[allow(dead_code)]
#[derive(Component)]
pub struct GridCell {
    pub x: i32,
    pub y: i32,
}
