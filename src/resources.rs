use bevy::prelude::*;
use bevy::utils::HashMap;

/// シミュレーション設定
#[derive(Resource, Clone)]
pub struct SimulationConfig {
    pub world_width: f32,
    pub world_height: f32,
    pub softening_epsilon: f32,
    pub max_acceleration: f32,
    pub interaction_radius: f32,
    pub interaction_strength: f32,
    pub collision_stiffness: f32,
    pub initial_population: usize,
    pub initial_lifetime: f32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            world_width: 1600.0,
            world_height: 900.0,
            softening_epsilon: 10.0,
            max_acceleration: 100.0,
            interaction_radius: 150.0,
            interaction_strength: 10000.0,
            collision_stiffness: 1000.0,
            initial_population: 10000,
            initial_lifetime: 50.0,
        }
    }
}

/// 進化・交配ルール
#[derive(Resource, Clone)]
pub struct EvolutionConfig {
    pub mating_radius: f32,
    pub min_parents: usize,
    pub max_parents: usize,
    pub reproduction_cost: f32,
    pub base_decay_per_second: f32,
    pub life_loss_per_distance: f32,
    pub cooldown_base: f32,
    pub mutation_sigma_base: f32,
    pub mutation_sigma_scale: f32,
    pub trait_lock_probability: f32,
    pub max_children_per_tick: usize,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            mating_radius: 30.0,
            min_parents: 1,
            max_parents: 8,
            reproduction_cost: 5.0,
            base_decay_per_second: 0.5,
            life_loss_per_distance: 0.1,
            cooldown_base: 1.5,
            mutation_sigma_base: 0.05,
            mutation_sigma_scale: 0.2,
            trait_lock_probability: 0.3,
            max_children_per_tick: 50,
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
