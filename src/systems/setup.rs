use crate::components::*;
use crate::resources::*;
use bevy::prelude::*;
use rand::Rng;

/// 粒子をスポーンする共通ヘルパー関数
fn spawn_particle(
    commands: &mut Commands,
    config: &SimulationConfig,
    evolution: &EvolutionConfig,
    rng: &mut rand::rngs::ThreadRng,
) {
    let pos = Vec2::new(
        rng.gen::<f32>() * config.world_width,
        rng.gen::<f32>() * config.world_height,
    );

    let mass = rng.gen_range(0.5..2.0);
    let drag_coefficient = rng.gen_range(0.01..3.5);

    let signal_raw = Vec3::new(
        rng.gen_range(-1.0..1.0),
        rng.gen_range(-1.0..1.0),
        rng.gen_range(-1.0..1.0),
    );
    let response_raw = Vec3::new(
        rng.gen_range(-1.0..1.0),
        rng.gen_range(-1.0..1.0),
        rng.gen_range(-1.0..1.0),
    );
    let signal_vector = signal_raw.normalize_or_zero();
    let response_vector = response_raw.normalize_or_zero();

    let mate_kernel = MateKernelParams {
        bias: rng.gen_range(-0.5..0.5),
        distance_weight: rng.gen_range(0.5..1.5),
        distance_scale: 1.0,
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

    let mut genome = ParticleGenome {
        mass,
        drag_coefficient,
        signal_vector,
        response_vector,
        mate_kernel,
        mutation,
        dominance_bias: rng.gen_range(0.1..0.9),
        reproductive_strength: rng.gen_range(0.5..1.5),
    };

    let appearance = ParticleAppearance::from_genome(&genome);
    let collision_radius = appearance.collision_radius();
    genome.mate_kernel.distance_scale =
        (collision_radius * evolution.mating_radius_ratio * rng.gen_range(0.5..1.5)).max(1.0);
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
            radius: collision_radius,
        },
        genome,
        appearance,
        state,
        Velocity { value: velocity },
        Acceleration::default(),
        PhysicsParams {
            mass,
            drag_coefficient,
        },
        KinematicsHistory { last_position: pos },
    ));
}

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
        spawn_particle(&mut commands, &config, &evolution, &mut rng);
    }

    info!("Spawned {} particles", config.initial_population);
}

/// FPS表示用のテキストコンポーネント
#[derive(Component)]
pub struct FpsText;

#[derive(Component)]
pub struct ParticleCountText;

#[derive(Component)]
pub struct AlgorithmText;

#[derive(Component)]
pub struct StateText;

#[derive(Component)]
pub struct StartStopButton;

#[derive(Component)]
pub struct ResetButton;

#[derive(Component)]
pub struct AlgorithmButton;

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

    // アルゴリズム表示テキスト
    commands.spawn((
        TextBundle::from_section(
            "Algorithm: Standard Evolution",
            TextStyle {
                font_size: 20.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(70.0),
            left: Val::Px(10.0),
            ..default()
        }),
        AlgorithmText,
    ));

    // 状態表示テキスト
    commands.spawn((
        TextBundle::from_section(
            "State: Running",
            TextStyle {
                font_size: 20.0,
                color: Color::srgb(0.0, 1.0, 0.0),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(100.0),
            left: Val::Px(10.0),
            ..default()
        }),
        StateText,
    ));

    // コントロールパネル用のコンテナ
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Px(20.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // Start/Stop ボタン
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(120.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::srgb(0.2, 0.6, 0.2).into(),
                        ..default()
                    },
                    StartStopButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Stop",
                        TextStyle {
                            font_size: 20.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ));
                });

            // Reset ボタン
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(120.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::srgb(0.6, 0.2, 0.2).into(),
                        ..default()
                    },
                    ResetButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Reset",
                        TextStyle {
                            font_size: 20.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ));
                });

            // Algorithm 切り替えボタン
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(180.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::srgb(0.2, 0.2, 0.6).into(),
                        ..default()
                    },
                    AlgorithmButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Change Algorithm",
                        TextStyle {
                            font_size: 20.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ));
                });
        });
}

/// UI ボタンの相互作用を処理
pub fn handle_ui_interaction(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, Option<&StartStopButton>, Option<&ResetButton>, Option<&AlgorithmButton>),
        (Changed<Interaction>, With<Button>),
    >,
    mut state: ResMut<SimulationState>,
    mut algorithm: ResMut<SimulationAlgorithm>,
    particle_query: Query<Entity, With<Particle>>,
    config: Res<SimulationConfig>,
    evolution: Res<EvolutionConfig>,
) {
    for (interaction, mut color, start_stop, reset, algo) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if start_stop.is_some() {
                    // Start/Stop ボタンが押された
                    *state = match *state {
                        SimulationState::Running => SimulationState::Paused,
                        SimulationState::Paused => SimulationState::Running,
                    };
                } else if reset.is_some() {
                    // Reset ボタンが押された
                    // すべての粒子を削除
                    for entity in particle_query.iter() {
                        commands.entity(entity).despawn_recursive();
                    }
                    // 初期粒子を再生成
                    spawn_initial_particles(&mut commands, &config, &evolution);
                    // シミュレーションを再開
                    *state = SimulationState::Running;
                } else if algo.is_some() {
                    // Algorithm ボタンが押された - 次のアルゴリズムに切り替え
                    *algorithm = match *algorithm {
                        SimulationAlgorithm::Standard => SimulationAlgorithm::PhysicsOnly,
                        SimulationAlgorithm::PhysicsOnly => SimulationAlgorithm::FastReproduction,
                        SimulationAlgorithm::FastReproduction => SimulationAlgorithm::Standard,
                    };
                }
                *color = Color::srgb(0.35, 0.75, 0.35).into();
            }
            Interaction::Hovered => {
                if start_stop.is_some() {
                    *color = Color::srgb(0.25, 0.7, 0.25).into();
                } else if reset.is_some() {
                    *color = Color::srgb(0.7, 0.25, 0.25).into();
                } else if algo.is_some() {
                    *color = Color::srgb(0.25, 0.25, 0.7).into();
                }
            }
            Interaction::None => {
                if start_stop.is_some() {
                    *color = Color::srgb(0.2, 0.6, 0.2).into();
                } else if reset.is_some() {
                    *color = Color::srgb(0.6, 0.2, 0.2).into();
                } else if algo.is_some() {
                    *color = Color::srgb(0.2, 0.2, 0.6).into();
                }
            }
        }
    }
}

/// 初期粒子をスポーン（リセット用）
fn spawn_initial_particles(
    commands: &mut Commands,
    config: &SimulationConfig,
    evolution: &EvolutionConfig,
) {
    let mut rng = rand::thread_rng();

    for _ in 0..config.initial_population {
        spawn_particle(commands, config, evolution, &mut rng);
    }
}

/// FPS と粒子数を更新
pub fn update_ui(
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    particle_query: Query<&Particle>,
    state: Res<SimulationState>,
    algorithm: Res<SimulationAlgorithm>,
    mut fps_query: Query<&mut Text, (With<FpsText>, Without<ParticleCountText>, Without<AlgorithmText>, Without<StateText>)>,
    mut count_query: Query<&mut Text, (With<ParticleCountText>, Without<FpsText>, Without<AlgorithmText>, Without<StateText>)>,
    mut algo_query: Query<&mut Text, (With<AlgorithmText>, Without<FpsText>, Without<ParticleCountText>, Without<StateText>)>,
    mut state_query: Query<&mut Text, (With<StateText>, Without<FpsText>, Without<ParticleCountText>, Without<AlgorithmText>)>,
    button_query: Query<(&Children, Option<&StartStopButton>), With<Button>>,
    mut button_text_query: Query<&mut Text, (Without<FpsText>, Without<ParticleCountText>, Without<AlgorithmText>, Without<StateText>)>,
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

    // アルゴリズム表示更新
    if let Ok(mut text) = algo_query.get_single_mut() {
        text.sections[0].value = format!("Algorithm: {}", algorithm.name());
    }

    // 状態表示更新
    if let Ok(mut text) = state_query.get_single_mut() {
        match *state {
            SimulationState::Running => {
                text.sections[0].value = "State: Running".to_string();
                text.sections[0].style.color = Color::srgb(0.0, 1.0, 0.0);
            }
            SimulationState::Paused => {
                text.sections[0].value = "State: Paused".to_string();
                text.sections[0].style.color = Color::srgb(1.0, 1.0, 0.0);
            }
        }
    }

    // Start/Stop ボタンのテキスト更新
    for (children, start_stop) in &button_query {
        if start_stop.is_some() {
            for &child in children.iter() {
                if let Ok(mut text) = button_text_query.get_mut(child) {
                    text.sections[0].value = match *state {
                        SimulationState::Running => "Stop".to_string(),
                        SimulationState::Paused => "Start".to_string(),
                    };
                }
            }
        }
    }
}
