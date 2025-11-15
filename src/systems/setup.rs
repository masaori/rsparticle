use crate::components::*;
use crate::resources::*;
use bevy::prelude::*;
use rand::Rng;

/// 初期セットアップ
pub fn setup(
    mut commands: Commands,
    config: Res<SimulationConfig>,
    evolution: Res<EvolutionConfig>,
) {
    // カメラのセットアップ
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(config.world_width / 2.0, config.world_height / 2.0, 0.0),
        ..default()
    });

    let mut rng = rand::thread_rng();

    for _ in 0..config.initial_population {
        let pos = Vec2::new(
            rng.gen::<f32>() * config.world_width,
            rng.gen::<f32>() * config.world_height,
        );

        let mass = rng.gen_range(0.5..2.0);
        let drag = rng.gen_range(1.0..3.5);

        let affinity_raw = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        );
        let affinity_vector = affinity_raw.normalize_or_zero();

        let mate_kernel = MateKernelParams {
            bias: rng.gen_range(-0.5..0.5),
            distance_weight: rng.gen_range(0.5..1.5),
            distance_scale: rng
                .gen_range(evolution.mating_radius * 0.5..evolution.mating_radius * 1.5)
                .max(1.0),
            energy_weight: rng.gen_range(0.5..1.5),
            similarity_weight: rng.gen_range(-1.0..1.0),
            diversity_weight: rng.gen_range(0.2..1.2),
            crowding_weight: rng.gen_range(0.2..1.0),
            slope: rng.gen_range(0.5..2.0),
        };

        let mutation = MutationParams {
            sigma_base: evolution.mutation_sigma_base * rng.gen_range(0.8..1.2),
            sigma_scale: evolution.mutation_sigma_scale * rng.gen_range(0.8..1.2),
            trait_lock_probability: evolution.trait_lock_probability * rng.gen_range(0.7..1.3),
        };

        let genome = ParticleGenome {
            mass,
            drag_coefficient: drag,
            affinity_vector,
            mate_kernel,
            mutation,
            dominance_bias: rng.gen_range(0.1..0.9),
            reproductive_strength: rng.gen_range(0.5..1.5),
        };

        let appearance = ParticleAppearance::from_genome(&genome);
        let sprite_size = appearance.sprite_extents();
        let display_color = appearance.color;

        let state = ParticleState {
            lifetime: config.initial_lifetime,
            max_lifetime: config.initial_lifetime,
            distance_traveled: 0.0,
            offspring_count: 0,
            cooldown: 0.0,
        };

        let velocity = Vec2::new(rng.gen_range(-20.0..20.0), rng.gen_range(-20.0..20.0));

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: display_color,
                    custom_size: Some(sprite_size),
                    ..default()
                },
                transform: Transform::from_xyz(pos.x, pos.y, 0.0),
                ..default()
            },
            Particle {
                radius: appearance.collision_radius(),
            },
            genome,
            appearance,
            state,
            Velocity { value: velocity },
            Acceleration::default(),
            PhysicsParams {
                mass: mass,
                drag_coefficient: drag,
            },
            KinematicsHistory { last_position: pos },
        ));
    }

    info!("Spawned {} particles", config.initial_population);
}

/// FPS表示用のテキストコンポーネント
#[derive(Component)]
pub struct FpsText;

#[derive(Component)]
pub struct ParticleCountText;

/// UI セットアップ
pub fn setup_ui(mut commands: Commands) {
    // FPS テキスト
    commands.spawn((
        TextBundle::from_section(
            "FPS: 0",
            TextStyle {
                font_size: 20.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        FpsText,
    ));

    // 粒子数テキスト
    commands.spawn((
        TextBundle::from_section(
            "Particles: 0",
            TextStyle {
                font_size: 20.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Px(10.0),
            ..default()
        }),
        ParticleCountText,
    ));
}

/// FPS と粒子数を更新
pub fn update_ui(
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    particle_query: Query<&Particle>,
    mut fps_query: Query<&mut Text, (With<FpsText>, Without<ParticleCountText>)>,
    mut count_query: Query<&mut Text, (With<ParticleCountText>, Without<FpsText>)>,
) {
    // FPS更新
    if let Ok(mut text) = fps_query.get_single_mut() {
        if let Some(fps) = diagnostics.get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.sections[0].value = format!("FPS: {:.0}", value);
            }
        }
    }

    // 粒子数更新
    if let Ok(mut text) = count_query.get_single_mut() {
        let count = particle_query.iter().count();
        text.sections[0].value = format!("Particles: {}", count);
    }
}
