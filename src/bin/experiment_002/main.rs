use bevy::{
    app::AppExit,
    core_pipeline::core_3d::Camera3dDepthLoadOp,
    log::Level,
    prelude::*,
    render::{camera::ScalingMode, view::RenderLayers},
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    transform::TransformSystem,
    utils::tracing::span,
    window::{CursorGrabMode, PresentMode, PrimaryWindow, WindowMode},
};
use bevy_rapier3d::prelude::*;
use bevy_scene_hook::{HookPlugin, HookedSceneBundle, SceneHook};
use bevy_space_program::{generate_mipmaps, MipmapGeneratorPlugin, MipmapGeneratorSettings};
use big_space::{
    camera::{CameraController, CameraInput},
    reference_frame::RootReferenceFrame,
    world_query::GridTransformReadOnly,
    FloatingOrigin, GridCell, IgnoreFloatingOrigin,
};

const BACKGROUND: RenderLayers = RenderLayers::layer(1);
const FOREGROUND: RenderLayers = RenderLayers::layer(2);
const OVERLAY: RenderLayers = RenderLayers::layer(3);

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Loading,
    PreRunning,
    Running,
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct OverlayGizmos {}

fn main() {
    println!("main() start");
    let app = App::new()
        .insert_state(AppState::Loading)
        .add_plugins((
            DefaultPlugins.build().disable::<TransformPlugin>(),
            big_space::FloatingOriginPlugin::<i64>::default(),
            big_space::debug::FloatingOriginDebugPlugin::<i64>::default(),
            big_space::camera::CameraControllerPlugin::<i64>::default(),
            bevy_framepace::FramepacePlugin,
            RapierDebugRenderPlugin::default(),
        ))
        .add_plugins((
            RapierPhysicsPlugin::<NoUserData>::default(),
            // RapierPhysicsPlugin::<NoUserData>::default().with_default_system_setup(false),
        ))
        .add_plugins(HookPlugin)
        .add_plugins(MipmapGeneratorPlugin)
        .init_gizmo_group::<OverlayGizmos>()
        .insert_resource(MipmapGeneratorSettings {
            anisotropic_filtering: 16,
            ..default()
        })
        .insert_resource(RapierConfiguration {
            gravity: Vec3::ZERO,
            physics_pipeline_active: true,
            query_pipeline_active: true,
            timestep_mode: TimestepMode::Interpolated {
                dt: 0.016666667,
                time_scale: 1.0,
                substeps: 1,
            },
            scaled_shape_subdivision: 2,
            force_update_from_transform_changes: true,
        })
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa::Sample8)
        .add_systems(
            Startup,
            (initiate_asset_loading, floating_origin_workaround)
                .run_if(in_state(AppState::Loading)),
        )
        .add_systems(
            Update,
            (wait_for_asset_loading).run_if(in_state(AppState::Loading)),
        )
        .add_systems(
            Update,
            (
                setup,
                ui_setup,
                overlay_render_setup,
                generate_mipmaps::<StandardMaterial>,
            )
                .run_if(in_state(AppState::PreRunning)),
        )
        .add_systems(
            PreUpdate,
            (cursor_grab_system, spawn).run_if(in_state(AppState::Running)),
        )
        .add_systems(
            Update,
            (modify_gravity, update_hud).run_if(in_state(AppState::Running)),
        )
        .add_systems(
            PostUpdate,
            (
                ui_text_system,
                highlight_nearest_object.after(TransformSystem::TransformPropagate),
            )
                .run_if(in_state(AppState::Running)),
        )
        .run();
    println!("main() stop");
    app
}

fn wait_for_asset_loading(
    meshes: Res<Assets<Mesh>>,
    mesh_assets: Res<MeshAssets>,
    scenes: Res<Assets<Scene>>,
    scene_assets: Res<SceneAssets>,
    mut state: ResMut<NextState<AppState>>,
    fpopeq: Query<Entity, With<FloatingOriginPlaceholderComponent>>,
) {
    let span = span!(Level::INFO, "wait_for_asset_loading()");
    let _enter = span.enter();
    debug!("start");
    let nav_ring_scene_option = scenes.get(&scene_assets.nav_ball_scene);
    let nav_ball_scene_option = scenes.get(&scene_assets.nav_ball_scene);
    let nav_ball_orbital_scene_option = scenes.get(&scene_assets.nav_ball_orbital_scene);
    let inverted_xyz_scene_option = scenes.get(&scene_assets.inverted_xyz_ball_scene);
    let nav_ring_mesh_option = meshes.get(&mesh_assets.nav_ball_mesh);
    let nav_ball_mesh_option = meshes.get(&mesh_assets.nav_ball_mesh);
    let nav_ball_orbital_mesh_option = meshes.get(&mesh_assets.nav_ball_orbital_mesh);
    let inverted_xyz_mesh_option = meshes.get(&mesh_assets.inverted_xyz_ball_mesh);
    let mut scenes_loaded = false;
    match (
        nav_ring_scene_option,
        nav_ball_scene_option,
        nav_ball_orbital_scene_option,
        inverted_xyz_scene_option,
    ) {
        (Some(_), Some(_), Some(_), Some(_)) => {
            debug!("scenes loaded");
            scenes_loaded = true;
        }
        _ => {}
    }
    let mut meshes_loaded = false;
    match (
        nav_ring_mesh_option,
        nav_ball_mesh_option,
        nav_ball_orbital_mesh_option,
        inverted_xyz_mesh_option,
    ) {
        (Some(_), Some(_), Some(_), Some(_)) => {
            debug!("meshes loaded");
            meshes_loaded = true;
        }
        _ => {}
    }
    if scenes_loaded && meshes_loaded {
        debug!("loading complete");
        state.set(AppState::PreRunning);
    }
    for each in fpopeq.iter() {
        debug!("{:?}", each);
    }
    debug!("stop");
}

#[derive(Resource, Debug, Default)]
pub struct MeshAssets {
    pub nav_ring_mesh: Handle<Mesh>,
    pub nav_ball_mesh: Handle<Mesh>,
    pub nav_ball_orbital_mesh: Handle<Mesh>,
    pub inverted_xyz_ball_mesh: Handle<Mesh>,
}

#[derive(Resource, Debug, Default)]
pub struct SceneAssets {
    pub nav_ring_scene: Handle<Scene>,
    pub nav_ball_scene: Handle<Scene>,
    pub nav_ball_orbital_scene: Handle<Scene>,
    pub inverted_xyz_ball_scene: Handle<Scene>,
}

#[derive(Component)]
pub struct Planet;

#[derive(Component)]
pub struct HUD;

#[derive(Component)]
pub struct TargetDisplay;

#[derive(Component)]
pub struct FloatingOriginPlaceholderComponent;

#[derive(Component)]
pub struct Crosshair;

fn floating_origin_workaround(mut commands: Commands, space: Res<RootReferenceFrame<i64>>) {
    let span = span!(Level::INFO, "floating_origin_workaround()");
    let _enter = span.enter();
    debug!("start");
    let (cam_cell, cam_pos) = space.imprecise_translation_to_grid(Vec3 {
        x: 200.0,
        y: 0.0,
        z: 0.0,
    });
    let cam_transform = Transform::from_translation(cam_pos);
    debug!("cam_transform: {:?}", cam_transform);
    commands.spawn((
        BACKGROUND,
        Camera3dBundle {
            transform: cam_transform,
            projection: Projection::Perspective(PerspectiveProjection {
                near: 1e-18,
                ..default()
            }),
            ..default()
        },
        cam_cell,
        FloatingOrigin,
        CameraController::default()
            .with_speed_bounds([10e-18, 10e35])
            .with_smoothness(0.9, 0.8)
            .with_speed(1.0),
    ));
    debug!("stop");
}

fn initiate_asset_loading(mut commands: Commands, asset_server: Res<AssetServer>) {
    let span = span!(Level::INFO, "initiate_asset_loading()");
    let _enter = span.enter();
    debug!("start");
    commands.insert_resource(MeshAssets {
        nav_ring_mesh: asset_server.load("experiment_002/nav_ring.glb#Mesh0/Primitive0"),
        nav_ball_mesh: asset_server.load("experiment_002/nav_ball.glb#Mesh0/Primitive0"),
        nav_ball_orbital_mesh: asset_server
            .load("experiment_002/nav_ball_orbital.glb#Mesh0/Primitive0"),
        inverted_xyz_ball_mesh: asset_server
            .load("experiment_002/inverted_xyz_ball.glb#Mesh0/Primitive0"),
    });
    commands.insert_resource(SceneAssets {
        nav_ring_scene: asset_server.load("experiment_002/nav_ring.glb#Scene0"),
        nav_ball_scene: asset_server.load("experiment_002/nav_ball.glb#Scene0"),
        nav_ball_orbital_scene: asset_server.load("experiment_002/nav_ball_orbital.glb#Scene0"),
        inverted_xyz_ball_scene: asset_server.load("experiment_002/inverted_xyz_ball.glb#Scene0"),
    });
    debug!("stop");
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    space: Res<RootReferenceFrame<i64>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut cam: ResMut<CameraInput>,
    scene_assets: Res<SceneAssets>,
    mut state: ResMut<NextState<AppState>>,
    mut perspective_hud_query: Query<Entity, (With<Camera3d>, With<CameraController>)>,
) {
    let Some(mut window) = windows.get_single_mut().ok() else {
        return;
    };
    window.mode = WindowMode::BorderlessFullscreen;
    window.present_mode = PresentMode::Fifo;
    window.cursor.grab_mode = CursorGrabMode::None;
    window.cursor.visible = true;
    cam.defaults_disabled = true;

    for each_perspective_hud_entity in perspective_hud_query.iter_mut() {
        commands
            .entity(each_perspective_hud_entity)
            .with_children(|parent| {
                parent.spawn((
                    BACKGROUND,
                    HookedSceneBundle {
                        hook: SceneHook::new(|entity, cmds| {
                            match entity.get::<Name>().map(|t| t.as_str()) {
                                _ => cmds.insert(BACKGROUND),
                            };
                        }),
                        scene: SceneBundle {
                            scene: scene_assets.nav_ball_orbital_scene.clone(),
                            transform: Transform::IDENTITY
                                .with_translation(Transform::IDENTITY.forward() * 18.0)
                                * Transform::IDENTITY
                                    .with_translation(Transform::IDENTITY.down() * 6.0),

                            ..default()
                        },
                    },
                    HUD,
                ));
            });
    }

    commands.spawn((
        OVERLAY,
        Camera2dBundle {
            camera: Camera {
                order: 3,
                ..default()
            },
            ..default()
        },
    ));
    let short_horizontal = Mesh2dHandle(meshes.add(Rectangle::new(10.0, 1.0)));
    let short_vertical = Mesh2dHandle(meshes.add(Rectangle::new(1.0, 10.0)));
    let long_horizontal = Mesh2dHandle(meshes.add(Rectangle::new(2000.0, 1.0)));
    let long_vertical = Mesh2dHandle(meshes.add(Rectangle::new(1.0, 2000.0)));
    let color = match Color::hex("FE9F00") {
        Ok(c) => c,
        Err(_) => Color::rgb(1.0, 1.0, 1.0),
    };
    commands
        .spawn((
            OVERLAY,
            Crosshair,
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Hidden,
            InheritedVisibility::HIDDEN,
        ))
        .with_children(|parent| {
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    visibility: Visibility::Inherited,
                    inherited_visibility: InheritedVisibility::HIDDEN,
                    mesh: long_horizontal.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: -1100.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: long_horizontal.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: 1100.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: long_vertical.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: 0.0,
                            y: -1100.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: long_vertical.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: 0.0,
                            y: 1100.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: short_horizontal.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: 25.0,
                            y: 30.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: short_horizontal.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: -25.0,
                            y: -30.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: short_horizontal.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: -25.0,
                            y: 30.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: short_horizontal.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: 25.0,
                            y: -30.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: short_vertical.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: 30.0,
                            y: 25.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: short_vertical.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: -30.0,
                            y: -25.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: short_vertical.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: -30.0,
                            y: 25.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: short_vertical.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: 30.0,
                            y: -25.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(color),
                    ..default()
                },
            ));
        });

    let hud_cam_transform = Transform::from_xyz(-7.5, 3.75, 3.0);
    debug!("hud_cam_transform: {:?}", hud_cam_transform);

    commands.spawn((
        FOREGROUND,
        Camera3dBundle {
            transform: hud_cam_transform,
            projection: Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical(10.0),
                ..default()
            }),
            camera_3d: Camera3d {
                depth_load_op: Camera3dDepthLoadOp::Load,
                ..default()
            },
            camera: Camera {
                order: 1,
                ..default()
            },

            ..default()
        },
        IgnoreFloatingOrigin,
    ));

    commands.spawn((
        FOREGROUND,
        HookedSceneBundle {
            hook: SceneHook::new(|entity, cmds| {
                match entity.get::<Name>().map(|t| t.as_str()) {
                    _ => cmds.insert(FOREGROUND),
                };
            }),
            scene: SceneBundle {
                scene: scene_assets.nav_ball_orbital_scene.clone(),
                transform: Transform {
                    translation: Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    ..default()
                },
                ..default()
            },
        },
        HUD,
    ));
    commands.spawn((
        FOREGROUND,
        HookedSceneBundle {
            hook: SceneHook::new(|entity, cmds| {
                match entity.get::<Name>().map(|t| t.as_str()) {
                    _ => cmds.insert(FOREGROUND),
                };
            }),
            scene: SceneBundle {
                scene: scene_assets.nav_ring_scene.clone(),
                transform: Transform {
                    translation: Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 2.0,
                    },
                    ..default()
                },
                ..default()
            },
        },
    ));
    commands.spawn((
        FOREGROUND,
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 10_000.0,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 0.0).looking_at(-Vec3::Z, Vec3::Y),
            ..default()
        },
    ));

    commands.spawn((
        BACKGROUND,
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 10_000.0,
                ..default()
            },
            ..default()
        },
    ));

    /* Planet */
    let mesh_handle = meshes.add(Sphere::new(100.0).mesh().ico(32).unwrap());
    let matl_handle = materials.add(StandardMaterial {
        base_color: Color::ORANGE_RED,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    let (planet_cell, planet_pos): (GridCell<i64>, _) =
        space.imprecise_translation_to_grid(Vec3::ZERO);
    let planet_transform = Transform::from_translation(planet_pos);
    debug!("planet_transform: {:?}", planet_transform);

    commands.spawn((
        BACKGROUND,
        Planet,
        RigidBody::Fixed,
        GravityScale(0.0),
        Collider::ball(100.0),
        PbrBundle {
            mesh: mesh_handle.clone(),
            material: matl_handle.clone(),
            transform: planet_transform,
            ..default()
        },
        planet_cell,
    ));

    /* CubeSat */
    let mesh_handle = meshes.add(Cuboid::default());
    let matl_handle = materials.add(StandardMaterial {
        base_color: Color::AQUAMARINE,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    let (cube_sat_cell, cube_sat_pos): (GridCell<i64>, _) =
        space.imprecise_translation_to_grid(Vec3 {
            x: -190.0,
            y: 0.0,
            z: 0.0,
        });
    commands.spawn((
        BACKGROUND,
        RigidBody::Dynamic,
        GravityScale(0.0),
        ExternalForce {
            force: Vec3::ZERO,
            torque: Vec3::ZERO,
        },
        Velocity {
            linvel: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            angvel: Vect {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        },
        PbrBundle {
            mesh: mesh_handle.clone(),
            material: matl_handle.clone(),
            transform: Transform::from_translation(cube_sat_pos),
            ..default()
        },
        cube_sat_cell,
    ));
    let matl_handle = materials.add(StandardMaterial {
        base_color: Color::AZURE,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    commands.spawn((
        BACKGROUND,
        RigidBody::KinematicVelocityBased,
        Collider::cuboid(0.5, 0.5, 0.5),
        GravityScale(0.0),
        Velocity {
            linvel: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            angvel: Vect {
                x: 1.0,
                y: 2.1,
                z: 0.001,
            },
        },
        PbrBundle {
            mesh: mesh_handle.clone(),
            material: matl_handle.clone(),
            transform: Transform::from_translation(cube_sat_pos),
            ..default()
        },
        cube_sat_cell,
    ));

    state.set(AppState::Running);
}

#[derive(Component, Reflect)]
pub struct DebugHudText;

fn ui_setup(
    mut commands: Commands,
    mut state: ResMut<NextState<AppState>>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    commands.spawn((
        FOREGROUND,
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 18.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_text_justify(JustifyText::Left)
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        DebugHudText,
        IgnoreFloatingOrigin,
    ));

    commands.spawn((
        BACKGROUND,
        TextBundle::from_section(
            "No Target",
            TextStyle {
                font_size: 18.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        }),
        TargetDisplay,
    ));

    let (default_gizmo_config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    default_gizmo_config.render_layers = BACKGROUND;
    let (overlay_gizmo_config, _) = config_store.config_mut::<OverlayGizmos>();
    overlay_gizmo_config.render_layers = OVERLAY;

    state.set(AppState::Running);
}

fn overlay_render_setup(mut overlay_gizmos: Gizmos<OverlayGizmos>, time: Res<Time>) {
    let sin = time.elapsed_seconds().sin() * 50.;
    overlay_gizmos.line_2d(Vec2::Y * -sin, Vec2::splat(-80.), Color::RED);
}

#[allow(clippy::type_complexity)]
fn ui_text_system(
    mut debug_text: Query<(&mut Text, &GlobalTransform), With<DebugHudText>>,
    time: Res<Time>,
    origin: Query<GridTransformReadOnly<i64>, With<FloatingOrigin>>,
    camera: Query<&CameraController>,
    objects: Query<(&Transform, &GridCell<i64>), (With<Handle<Mesh>>, Without<Planet>)>,
    reference_frame: Res<RootReferenceFrame<i64>>,
) {
    let origin = origin.single();
    let translation = origin.transform.translation;

    let grid_text = format!(
        "GridCell:\n{}x,\n{}y,\n{}z",
        origin.cell.x, origin.cell.y, origin.cell.z
    );

    let translation_text = format!(
        "Transform:\n{}x,\n{}y,\n{}z",
        translation.x, translation.y, translation.z
    );

    let real_position = reference_frame.grid_position_double(origin.cell, origin.transform);
    let real_position_f64_text = format!(
        "Combined (f64):\n{}x,\n{}y,\n{}z",
        real_position.x, real_position.y, real_position.z
    );
    let real_position_f32_text = format!(
        "Combined (f32):\n{}x,\n{}y,\n{}z",
        real_position.x as f32, real_position.y as f32, real_position.z as f32
    );

    let velocity = camera.single().velocity();
    let speed = velocity.0.length() / time.delta_seconds_f64();
    let camera_text = if speed > 3.0e8 {
        format!("Speed: {:.0e} * speed of light", speed / 3.0e8)
    } else {
        format!("Speed: {:.2e} m/s", speed)
    };

    let mut objects_text = "objects:\n".to_string();
    for (each_object_transform, each_object_grid) in objects.iter() {
        objects_text += &format!(
            "translation:\nx:{:0>10}, y:{:0>10}, z:{:0>10}\ngrid\nx:{:0>10}, y:{:0>10}, z:{:0>10}\n",
            each_object_transform.translation.x,
            each_object_transform.translation.y,
            each_object_transform.translation.z,
            each_object_grid.x,
            each_object_grid.y,
            each_object_grid.z,
        );
    }

    let mut debug_text = debug_text.single_mut();

    debug_text.0.sections[0].value = format!(
        "{grid_text}\n{translation_text}\n\n{real_position_f64_text}\n{real_position_f32_text}\n\n{camera_text}\n\n{objects_text}"
    );
}

fn highlight_nearest_object(
    camera_3d_query: Query<
        (&mut Camera, &mut Transform, &GlobalTransform),
        (With<CameraController>, With<Camera3d>, Without<Camera2d>),
    >,
    cameras: Query<&CameraController>,
    objects: Query<&GlobalTransform, Without<Crosshair>>,
    mut gizmos: Gizmos,
    mut target_display_query: Query<&mut Text, With<TargetDisplay>>,
    mut crosshair_transform_query: Query<
        &mut Transform,
        (With<Crosshair>, Without<Camera3d>, Without<Camera2d>),
    >,
    mut crosshair_visibility_query: Query<&mut Visibility, With<Crosshair>>,
    camera_2d_query: Query<
        (&mut Camera, &mut Transform, &GlobalTransform),
        (With<Camera2d>, Without<Camera3d>),
    >,
) {
    let span = span!(Level::INFO, "highlight_nearest_object()");
    let _enter = span.enter();
    // debug!("start");
    let Some((entity, _)) = cameras.single().nearest_object() else {
        debug!("cameras.single().nearest_object() returned none");
        return;
    };
    let Ok(transform) = objects.get(entity) else {
        debug!("objects.get(entity) did not return ok");
        return;
    };
    let (scale, rotation, translation) = transform.to_scale_rotation_translation();
    gizmos
        .sphere(translation, rotation, scale.x * 1.0, Color::RED)
        .circle_segments(128);

    let (camera_3d, _camera_3d_transform, camera_3d_global_transform) = camera_3d_query.single();

    let mut crosshair_transform = crosshair_transform_query.single_mut();

    let mut crosshair_visibility = crosshair_visibility_query.single_mut();

    let (camera_2d, _camera_2d_transform, camera_2d_global_transform) = camera_2d_query.single();

    let Some(camera_2d_viewport_rect) = camera_2d.logical_viewport_rect() else {
        debug!("camera_2d.logical_viewport_rect() returned none");
        return;
    };

    let target_text;
    let target_text_x = format!("{:>20}", translation.x);
    let target_text_y = format!("{:>20}", translation.y);
    let target_text_z = format!("{:>20}", translation.z);
    let mut overlay_text_x = "".to_string();
    let mut overlay_text_y = "".to_string();

    match target_display_query.get_single_mut() {
        Ok(mut target_display) => {
            match camera_3d.world_to_viewport(camera_3d_global_transform, translation) {
                Some(viewport_position) => {
                    match (
                        camera_2d_viewport_rect.contains(viewport_position),
                        camera_2d
                            .viewport_to_world_2d(camera_2d_global_transform, viewport_position),
                    ) {
                        (true, Some(overlay_position)) => {
                            *crosshair_visibility = Visibility::Visible;
                            crosshair_transform.translation.x = overlay_position.x;
                            crosshair_transform.translation.y = overlay_position.y;

                            target_text = "target onscreen";
                            overlay_text_x = format!("{:>20}", overlay_position.x);
                            overlay_text_y = format!("{:>20}", overlay_position.y);
                        }
                        (false, Some(overlay_position)) => {
                            *crosshair_visibility = Visibility::Hidden;

                            target_text = "target offscreen";
                            overlay_text_x = format!("{:>20}", overlay_position.x);
                            overlay_text_y = format!("{:>20}", overlay_position.y);
                        }
                        (true, None) => {
                            *crosshair_visibility = Visibility::Visible;
                            target_text = "target onscreen";
                        }
                        (false, None) => {
                            *crosshair_visibility = Visibility::Hidden;
                            target_text = "target offscreen";
                        }
                    }
                }
                None => {
                    *crosshair_visibility = Visibility::Hidden;
                    target_text = "target is behind us";
                }
            };

            target_display.sections[0].value = format!(
                "{}\n\n{}\n{}\n{}\n\n{}\n{}",
                target_text,
                target_text_x,
                target_text_y,
                target_text_z,
                overlay_text_x,
                overlay_text_y,
            );
        }
        Err(e) => {
            debug!("{:?}", e)
        }
    };

    // debug!("stop");
}

fn modify_gravity(
    mut rigid_body_query: Query<
        (&Transform, &Collider, &mut ExternalForce),
        (With<RigidBody>, Without<Planet>),
    >,
) {
    for (each_rigid_body_transform, each_rigid_body_collider, mut each_external_force) in
        rigid_body_query.iter_mut()
    {
        let d = (Vec3::ZERO - each_rigid_body_transform.translation)
            .length()
            .abs();
        let m1 = each_rigid_body_collider.raw.mass_properties(1.0).mass();
        let m2 = 10000000000000.0;
        let g = 6.67430e-11;
        let f = (g * m1 * m2) / d.powf(2.0);
        let gfv = -(each_rigid_body_transform.translation.normalize() * f);
        each_external_force.force = gfv;
    }
}

fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    btn: Res<ButtonInput<MouseButton>>,
    cam_grid: Query<GridTransformReadOnly<i64>, With<FloatingOrigin>>,
    camera_controller: Query<&CameraController>,
) {
    let capsule = Capsule3d::new(0.1, 0.2);
    let mesh_handle = meshes.add(capsule);
    let matl_handle = materials.add(StandardMaterial {
        base_color: Color::PURPLE,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });

    let single_cam_grid = cam_grid.single();
    let single_cam_cont = camera_controller.single();
    let spawn_transform = Transform {
        translation: single_cam_grid.transform.translation
            + (single_cam_grid.transform.forward() * 1.0),
        rotation: single_cam_grid.transform.rotation,
        scale: Vec3::ONE,
    };
    if btn.just_pressed(MouseButton::Right) {
        commands.spawn((
            BACKGROUND,
            *single_cam_grid.cell,
            RigidBody::Dynamic,
            Collider::capsule(
                Vec3 {
                    x: 0.0,
                    y: 0.1,
                    z: 0.0,
                },
                Vec3 {
                    x: 0.0,
                    y: -0.1,
                    z: 0.0,
                },
                0.1,
            ),
            GravityScale(0.0),
            Velocity {
                linvel: single_cam_cont.velocity().0.as_vec3()
                    + (single_cam_grid.transform.forward() * 10.0),
                angvel: Vect {
                    x: 1.0,
                    y: 1.1,
                    z: 1.2,
                },
            },
            PbrBundle {
                mesh: mesh_handle,
                material: matl_handle,
                transform: spawn_transform,
                ..default()
            },
            HUD,
        ));
    }
}

fn cursor_grab_system(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut cam: ResMut<CameraInput>,
    btn: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
    mut rapier_configuration: ResMut<RapierConfiguration>,
) {
    let Some(mut window) = windows.get_single_mut().ok() else {
        return;
    };

    if btn.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
        cam.defaults_disabled = false;
    }

    if key.just_pressed(KeyCode::Escape) {
        if window.cursor.grab_mode == CursorGrabMode::None {
            exit.send(AppExit);
        }
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
        cam.defaults_disabled = true;
    }

    if key.just_pressed(KeyCode::Period) {
        match rapier_configuration.timestep_mode {
            TimestepMode::Interpolated {
                dt,
                time_scale,
                substeps,
            } => {
                rapier_configuration.timestep_mode = {
                    let mut new_time_scale = time_scale * 2.0;
                    if new_time_scale > 512.0 {
                        new_time_scale = 512.0
                    }
                    debug!("time_scale: {:?}", new_time_scale);
                    TimestepMode::Interpolated {
                        dt,
                        time_scale: new_time_scale,
                        substeps,
                    }
                }
            }
            _ => {}
        };
    }
    if key.just_pressed(KeyCode::Comma) {
        match rapier_configuration.timestep_mode {
            TimestepMode::Interpolated {
                dt,
                time_scale,
                substeps,
            } => {
                rapier_configuration.timestep_mode = {
                    let mut new_time_scale = time_scale / 2.0;
                    if new_time_scale < 0.001953125 {
                        new_time_scale = 0.001953125
                    }
                    debug!("time_scale: {:?}", new_time_scale);
                    TimestepMode::Interpolated {
                        dt,
                        time_scale: new_time_scale,
                        substeps,
                    }
                }
            }
            _ => {}
        };
    }
    if key.just_pressed(KeyCode::Slash) {
        match rapier_configuration.timestep_mode {
            TimestepMode::Interpolated {
                dt,
                time_scale: _,
                substeps,
            } => {
                rapier_configuration.timestep_mode = {
                    let new_time_scale = 1.0;
                    debug!("time_scale: {:?}", new_time_scale);
                    TimestepMode::Interpolated {
                        dt,
                        time_scale: new_time_scale,
                        substeps,
                    }
                }
            }
            _ => {}
        };
    }
}

fn update_hud(
    mut hud_transform_query: Query<&mut Transform, (With<HUD>, Without<Planet>)>,
    camera_grid_query: Query<GridTransformReadOnly<i64>, (With<FloatingOrigin>, Without<HUD>)>,
    planet_transform_query: Query<&Transform, With<Planet>>,
) {
    let span = span!(Level::INFO, "update_hud()");
    let _enter = span.enter();
    let camera_grid = camera_grid_query.single();
    let mut camera_rotation = camera_grid.transform.rotation;
    let planet_transform = planet_transform_query.single();
    let mut camera_looking_at_planet_rotation = camera_grid
        .transform
        .looking_at(
            planet_transform.translation,
            planet_transform.up().normalize(),
        )
        .rotation
        .inverse();
    camera_rotation.z = -camera_rotation.z;
    camera_looking_at_planet_rotation.z = -camera_looking_at_planet_rotation.z;
    let camera_rotations_combined = camera_rotation * camera_looking_at_planet_rotation;
    for mut each_hud_transform in hud_transform_query.iter_mut() {
        let final_rotation = camera_rotations_combined;
        each_hud_transform.rotation = final_rotation;
    }
}
