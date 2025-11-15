use bevy::prelude::*;
use rand::Rng;
use crate::components::*;
use crate::resources::*;

/// 初期セットアップ
pub fn setup(
    mut commands: Commands,
    mut particle_registry: ResMut<ParticleTypeRegistry>,
    mut interaction_matrix: ResMut<InteractionMatrix>,
    config: Res<SimulationConfig>,
) {
    // カメラのセットアップ
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(
            config.world_width / 2.0,
            config.world_height / 2.0,
            0.0,
        ),
        ..default()
    });

    // 粒子タイプを定義（Color, 質量, 抵抗係数, 半径）
    let type_red = particle_registry.add_type(Color::srgb(1.0, 0.0, 0.0), 1.0, 2.0, 2.0);
    let type_green = particle_registry.add_type(Color::srgb(0.0, 1.0, 0.0), 1.0, 2.0, 2.0);
    let type_blue = particle_registry.add_type(Color::srgb(0.0, 0.0, 1.0), 1.0, 2.0, 2.0);

    // 相互作用行列を設定
    interaction_matrix.set(type_red, type_red, 5000.0);
    interaction_matrix.set(type_red, type_green, -20000.0);
    interaction_matrix.set(type_red, type_blue, -10000.0);
    interaction_matrix.set(type_green, type_red, -5000.0);
    interaction_matrix.set(type_green, type_green, 5000.0);
    interaction_matrix.set(type_green, type_blue, 5000.0);
    interaction_matrix.set(type_blue, type_red, 5000.0);
    interaction_matrix.set(type_blue, type_green, -20000.0);
    interaction_matrix.set(type_blue, type_blue, 5000.0);

    // 粒子を生成
    let num_particles_per_type = 5000;
    let spread_radius = config.world_width.min(config.world_height) * 0.25;  // 画面サイズの25%

    let mut rng = rand::thread_rng();

    // 各タイプの中心位置（画面中央）
    let center = Vec2::new(config.world_width / 2.0, config.world_height / 2.0);
    let centers = [
        center, // 赤
        center, // 緑
        center, // 青
    ];

    let types = [type_red, type_green, type_blue];

    for (type_idx, &particle_type) in types.iter().enumerate() {
        let center = centers[type_idx];
        let type_info = particle_registry.get_type(particle_type).unwrap();

        for _ in 0..num_particles_per_type {
            let angle: f32 = rng.gen::<f32>() * std::f32::consts::TAU;
            let distance: f32 = rng.gen::<f32>() * spread_radius;
            let offset = Vec2::new(angle.cos() * distance, angle.sin() * distance);
            let pos = center + offset;

            let vel_x: f32 = rng.gen::<f32>() * 20.0 - 10.0;
            let vel_y: f32 = rng.gen::<f32>() * 20.0 - 10.0;
            let vel = Vec2::new(vel_x, vel_y);

            // 粒子を spawn
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: type_info.color,
                        custom_size: Some(Vec2::new(type_info.radius * 2.0, type_info.radius * 2.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(pos.x, pos.y, 0.0),
                    ..default()
                },
                Particle { 
                    particle_type,
                    radius: type_info.radius,
                },
                Velocity { value: vel },
                Acceleration::default(),
                PhysicsParams {
                    mass: type_info.mass,
                    drag_coefficient: type_info.drag_coefficient,
                },
            ));
        }
    }

    info!("Spawned {} particles", num_particles_per_type * 3);
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
