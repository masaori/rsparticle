use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::components::ParticleTypeInfo;

/// 粒子種類間の相互作用行列
#[derive(Resource)]
pub struct InteractionMatrix {
    values: Vec<f32>,
    num_types: usize,
}

impl InteractionMatrix {
    pub fn new(num_types: usize) -> Self {
        Self {
            values: vec![0.0; num_types * num_types],
            num_types,
        }
    }

    pub fn get(&self, type_a: usize, type_b: usize) -> f32 {
        if type_a >= self.num_types || type_b >= self.num_types {
            return 0.0;
        }
        self.values[type_a * self.num_types + type_b]
    }

    pub fn set(&mut self, type_a: usize, type_b: usize, value: f32) {
        if type_a < self.num_types && type_b < self.num_types {
            self.values[type_a * self.num_types + type_b] = value;
        }
    }
}

/// 粒子タイプのレジストリ
#[derive(Resource)]
pub struct ParticleTypeRegistry {
    pub types: Vec<ParticleTypeInfo>,
}

impl ParticleTypeRegistry {
    pub fn new() -> Self {
        Self { types: Vec::new() }
    }

    pub fn add_type(&mut self, color: Color, mass: f32, drag_coefficient: f32, radius: f32) -> usize {
        let type_id = self.types.len();
        self.types.push(ParticleTypeInfo {
            type_id,
            color,
            mass,
            drag_coefficient,
            radius,
        });
        type_id
    }

    pub fn get_type(&self, type_id: usize) -> Option<&ParticleTypeInfo> {
        self.types.get(type_id)
    }
}

/// シミュレーション設定
#[derive(Resource, Clone)]
pub struct SimulationConfig {
    pub world_width: f32,
    pub world_height: f32,
    pub softening_epsilon: f32,
    pub max_acceleration: f32,
    pub interaction_radius: f32,  // 相互作用の最大距離
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            world_width: 1600.0,
            world_height: 1200.0,
            softening_epsilon: 10.0,
            max_acceleration: 100.0,
            interaction_radius: 150.0,  // この距離以上離れた粒子は無視
        }
    }
}

/// 空間分割グリッド
#[derive(Resource)]
pub struct SpatialGrid {
    pub cell_size: f32,
    pub grid: HashMap<(i32, i32), Vec<Entity>>,
    pub cols: i32,
    pub rows: i32,
}

impl SpatialGrid {
    pub fn new(world_width: f32, world_height: f32, cell_size: f32) -> Self {
        Self {
            cell_size,
            grid: HashMap::new(),
            cols: (world_width / cell_size).ceil() as i32,
            rows: (world_height / cell_size).ceil() as i32,
        }
    }

    pub fn get_cell(&self, x: f32, y: f32) -> (i32, i32) {
        let cell_x = (x / self.cell_size).floor() as i32;
        let cell_y = (y / self.cell_size).floor() as i32;
        (cell_x, cell_y)
    }

    pub fn insert(&mut self, cell: (i32, i32), entity: Entity) {
        self.grid.entry(cell).or_insert_with(Vec::new).push(entity);
    }

    pub fn get_neighbors(&self, cell: (i32, i32)) -> Vec<Entity> {
        let (cx, cy) = cell;
        let mut capacity = 0;
        
        // 事前にサイズを計算
        for dx in -1..=1 {
            for dy in -1..=1 {
                let nx = (cx + dx).rem_euclid(self.cols);
                let ny = (cy + dy).rem_euclid(self.rows);
                if let Some(entities) = self.grid.get(&(nx, ny)) {
                    capacity += entities.len();
                }
            }
        }
        
        let mut neighbors = Vec::with_capacity(capacity);

        // 周囲9セルを探索（周期境界条件対応）
        for dx in -1..=1 {
            for dy in -1..=1 {
                let nx = (cx + dx).rem_euclid(self.cols);
                let ny = (cy + dy).rem_euclid(self.rows);
                
                if let Some(entities) = self.grid.get(&(nx, ny)) {
                    neighbors.extend_from_slice(entities);
                }
            }
        }

        neighbors
    }
}
