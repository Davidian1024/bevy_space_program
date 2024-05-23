use std::f32::consts::PI;

use bevy::{
    app::AppExit,
    core_pipeline::bloom::BloomSettings,
    math::DVec3,
    prelude::*,
    render::{camera::Exposure, view::RenderLayers},
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::{CursorGrabMode, PresentMode, PrimaryWindow, WindowMode},
};
use big_space::{
    camera::{CameraController, CameraInput},
    reference_frame::RootReferenceFrame,
    FloatingOrigin, GridCell, IgnoreFloatingOrigin,
};
use rand::Rng;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<TransformPlugin>(),
            big_space::FloatingOriginPlugin::<i64>::default(),
            big_space::debug::FloatingOriginDebugPlugin::<i64>::default(),
            big_space::camera::CameraControllerPlugin::<i64>::default(),
            bevy_framepace::FramepacePlugin,
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 100.0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (input_handling, update_targeting_overlay))
        // .add_systems(Update, (input_handling))
        .run()
}

const BACKGROUND: RenderLayers = RenderLayers::layer(1);
const OVERLAY: RenderLayers = RenderLayers::layer(2);

#[derive(Component)]
pub struct ValidTarget;

#[derive(Component)]
pub struct CursorNearestReticle;

#[derive(Component)]
pub struct TargetObjectReticle;

#[derive(Resource, Debug)]
pub struct TargetResource {
    target: Option<Entity>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    space: Res<RootReferenceFrame<i64>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut cam: ResMut<CameraInput>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut gizmo_config_store: ResMut<GizmoConfigStore>,
) {
    /* User Interface Setup */
    let Some(mut window) = windows.get_single_mut().ok() else {
        return;
    };
    window.mode = WindowMode::BorderlessFullscreen;
    window.present_mode = PresentMode::Fifo;
    window.cursor.grab_mode = CursorGrabMode::None;
    window.cursor.visible = true;
    cam.defaults_disabled = true;

    /* Ensure gizmos will be rendered to the background layer */
    let (default_gizmo_config, _) = gizmo_config_store.config_mut::<DefaultGizmoConfigGroup>();
    default_gizmo_config.render_layers = BACKGROUND;

    /* Overlay Camera */
    commands.spawn((
        OVERLAY,
        IgnoreFloatingOrigin,
        Camera2dBundle {
            camera: Camera {
                order: 2,
                // viewport: todo!(),
                // is_active: todo!(),
                // computed: todo!(),
                // target: todo!(),
                hdr: true,
                // output_mode: CameraOutputMode::Write {
                //     blend_state: None,
                //     color_attachment_load_op: LoadOp::Load,
                // },
                // msaa_writeback: todo!(),
                // clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            // camera_render_graph: todo!(),
            // projection: todo!(),
            // visible_entities: todo!(),
            // frustum: todo!(),
            // transform: todo!(),
            // global_transform: todo!(),
            camera_2d: Camera2d,
            // tonemapping: todo!(),
            // deband_dither: todo!(),
            // main_texture_usages: todo!(),
            ..default()
        },
    ));

    /* CursorNearestReticle */
    let small_triangle = Mesh2dHandle(meshes.add(Triangle2d::new(
        Vec2::ZERO,
        Vec2 { x: 10.0, y: 0.0 },
        Vec2 { x: 0.0, y: 10.0 },
    )));
    let camera_reticle_color = match Color::hex("B2AFC2") {
        Ok(c) => c,
        Err(_) => Color::rgb(1.0, 1.0, 1.0),
    };
    commands
        .spawn((
            OVERLAY,
            IgnoreFloatingOrigin,
            CursorNearestReticle,
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Hidden,
            InheritedVisibility::HIDDEN,
        ))
        .with_children(|parent| {
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: small_triangle.clone(),
                    material: color_materials.add(camera_reticle_color),
                    transform: Transform {
                        translation: Vec3 {
                            x: 0.0,
                            y: 10.0,
                            z: 0.0,
                        },
                        rotation: Quat::from_rotation_z(PI / 4.0),
                        ..default()
                    },
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: small_triangle.clone(),
                    material: color_materials.add(camera_reticle_color),
                    transform: Transform {
                        translation: Vec3 {
                            x: -10.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        rotation: Quat::from_rotation_z((PI / 4.0) + (PI / 2.0)),
                        ..default()
                    },
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: small_triangle.clone(),
                    material: color_materials.add(camera_reticle_color),
                    transform: Transform {
                        translation: Vec3 {
                            x: 0.0,
                            y: -10.0,
                            z: 0.0,
                        },
                        rotation: Quat::from_rotation_z((PI / 4.0) + PI),
                        ..default()
                    },
                    ..default()
                },
            ));
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    mesh: small_triangle.clone(),
                    material: color_materials.add(camera_reticle_color),
                    transform: Transform {
                        translation: Vec3 {
                            x: 10.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        rotation: Quat::from_rotation_z(-(PI / 4.0)),
                        ..default()
                    },
                    ..default()
                },
            ));
        });

    /* Crosshair */
    let crosshair_color = match Color::hex("FE9F00") {
        Ok(c) => c,
        Err(_) => Color::rgb(1.0, 1.0, 1.0),
    };
    let long_horizontal = Mesh2dHandle(meshes.add(Rectangle::new(2000.0, 1.0)));
    let long_vertical = Mesh2dHandle(meshes.add(Rectangle::new(1.0, 2000.0)));
    commands
        .spawn((
            OVERLAY,
            IgnoreFloatingOrigin,
            TargetObjectReticle,
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::VISIBLE,
        ))
        .with_children(|parent| {
            parent.spawn((
                OVERLAY,
                MaterialMesh2dBundle {
                    // visibility: Visibility::Inherited,
                    // inherited_visibility: InheritedVisibility::HIDDEN,
                    mesh: long_horizontal.clone(),
                    transform: Transform {
                        translation: Vec3 {
                            x: -1100.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        ..default()
                    },
                    material: color_materials.add(crosshair_color),
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
                    material: color_materials.add(crosshair_color),
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
                    material: color_materials.add(crosshair_color),
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
                    material: color_materials.add(crosshair_color),
                    ..default()
                },
            ));
        });

    /* Spawn the Sun at (0,0,0) */
    let sun_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: Color::rgb_linear(10000000., 10000000., 10000000.),
        ..default()
    });
    let sun_radius_m = 695_508_000.0;
    let sun_mesh = meshes.add(Sphere::new(sun_radius_m).mesh().ico(32).unwrap());
    commands
        .spawn((
            ValidTarget,
            BACKGROUND,
            GridCell::<i64>::ZERO,
            PointLightBundle {
                point_light: PointLight {
                    intensity: 35.73e27,
                    range: 1e20,
                    radius: sun_radius_m,
                    shadows_enabled: true,
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|builder| {
            builder.spawn((
                BACKGROUND,
                PbrBundle {
                    mesh: sun_mesh,
                    material: sun_mat,
                    ..default()
                },
            ));
        });

    /* Spawn the user controlled camera */
    let (cam_cell, cam_pos): (GridCell<i64>, _) = space.translation_to_grid(DVec3 {
        x: 0.0,
        y: 0.0,
        z: (sun_radius_m as f64 * 20.0) + 1000.0,
    });
    let cam_hud_mat = materials.add(StandardMaterial {
        base_color: Color::GRAY,
        perceptual_roughness: 1.0,
        reflectance: 0.5,
        ..default()
    });
    let (_cam_hud_cell, cam_hud_pos): (GridCell<i64>, _) = space.translation_to_grid(DVec3 {
        z: (sun_radius_m as f64 * 20.0) + 635.0,
        ..default()
    });
    let cam_hud_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0)).clone();
    commands
        .spawn((
            BACKGROUND,
            Camera3dBundle {
                transform: Transform::from_translation(cam_pos),
                camera: Camera {
                    order: 1,
                    hdr: true,
                    ..default()
                },
                exposure: Exposure::SUNLIGHT,
                ..default()
            },
            BloomSettings::default(),
            cam_cell,
            FloatingOrigin, // Important: marks the floating origin entity for rendering.
            CameraController::default() // Built-in camera controller
                .with_speed_bounds([10e-18, 10e35])
                .with_smoothness(0.9, 0.8)
                .with_speed(1.0),
        ))
        .with_children(|builder| {
            builder.spawn((
                BACKGROUND,
                PbrBundle {
                    mesh: cam_hud_mesh.clone(),
                    material: cam_hud_mat.clone(),
                    transform: Transform::from_translation(
                        cam_hud_pos
                            + Vec3 {
                                x: 3.5,
                                y: 2.0,
                                z: 0.0,
                            },
                    ),
                    ..default()
                },
            ));
            builder.spawn((
                BACKGROUND,
                PbrBundle {
                    mesh: cam_hud_mesh.clone(),
                    material: cam_hud_mat.clone(),
                    transform: Transform::from_translation(
                        cam_hud_pos
                            + Vec3 {
                                x: -3.5,
                                y: 2.0,
                                z: 0.0,
                            },
                    ),
                    ..default()
                },
            ));
            builder.spawn((
                BACKGROUND,
                PbrBundle {
                    mesh: cam_hud_mesh.clone(),
                    material: cam_hud_mat.clone(),
                    transform: Transform::from_translation(
                        cam_hud_pos
                            + Vec3 {
                                x: 3.5,
                                y: -2.0,
                                z: 0.0,
                            },
                    ),
                    ..default()
                },
            ));
            builder.spawn((
                BACKGROUND,
                PbrBundle {
                    mesh: cam_hud_mesh.clone(),
                    material: cam_hud_mat.clone(),
                    transform: Transform::from_translation(
                        cam_hud_pos
                            + Vec3 {
                                x: -3.5,
                                y: -2.0,
                                z: 0.0,
                            },
                    ),
                    ..default()
                },
            ));
        });

    /* Spawn a purple ball with a radius of 1.0 */
    let (ball_cell, ball_pos): (GridCell<i64>, _) = space.translation_to_grid(Vec3 {
        x: 0.0,
        y: 0.0,
        z: sun_radius_m * 20.0,
    });
    let ball_mat = materials.add(StandardMaterial {
        base_color: Color::PURPLE,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    let purple_ball_mesh = meshes.add(Sphere::new(1.0).mesh().ico(32).unwrap());
    let purple_ball_entity = commands
        .spawn((
            ValidTarget,
            BACKGROUND,
            PbrBundle {
                mesh: purple_ball_mesh,
                material: ball_mat,
                transform: Transform::from_translation(ball_pos),
                ..default()
            },
            ball_cell,
        ))
        .id();

    commands.insert_resource(TargetResource {
        target: Some(purple_ball_entity),
    });

    /* Spawn a star field */
    let star_mesh = meshes.add(Sphere::new(1e10).mesh().ico(32).unwrap());
    let star_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: Color::rgb_linear(100000., 100000., 100000.),
        ..default()
    });
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        commands.spawn((
            BACKGROUND,
            GridCell::<i64>::new(
                ((rng.gen::<f32>() - 0.5) * 1e10) as i64,
                ((rng.gen::<f32>() - 0.5) * 1e10) as i64,
                ((rng.gen::<f32>() - 0.5) * 1e10) as i64,
            ),
            PbrBundle {
                mesh: star_mesh.clone(),
                material: star_mat.clone(),
                ..default()
            },
        ));
    }
}

fn update_targeting_overlay(
    camera_3d_query: Query<
        (&mut Camera, &mut Transform, &GlobalTransform),
        (With<CameraController>, With<Camera3d>, Without<Camera2d>),
    >,
    camera_2d_query: Query<
        (&mut Camera, &mut Transform, &GlobalTransform),
        (With<Camera2d>, Without<Camera3d>),
    >,
    valid_targets_query: Query<(&GlobalTransform, Entity), With<ValidTarget>>,
    mut cursor_nearest_reticle_transform_query: Query<
        &mut Transform,
        (
            With<CursorNearestReticle>,
            Without<Camera3d>,
            Without<Camera2d>,
        ),
    >,
    mut cursor_nearest_reticle_visibility_query: Query<&mut Visibility, With<CursorNearestReticle>>,
    key: Res<ButtonInput<KeyCode>>,
    mut target_resource: ResMut<TargetResource>,
    mut target_object_reticle_transform_query: Query<
        &mut Transform,
        (
            With<TargetObjectReticle>,
            Without<CursorNearestReticle>,
            Without<Camera3d>,
            Without<Camera2d>,
        ),
    >,
    mut target_object_reticle_visibility_query: Query<
        &mut Visibility,
        (With<TargetObjectReticle>, Without<CursorNearestReticle>),
    >,
    objects: Query<&GlobalTransform>,
) {
    let (camera_3d, _camera_3d_transform, camera_3d_global_transform) = camera_3d_query.single();
    let (camera_2d, _camera_2d_transform, camera_2d_global_transform) = camera_2d_query.single();

    /* Highlight object nearest to cursor (center of screen) with small reticle */
    let mut cursor_nearest_reticle_transform = cursor_nearest_reticle_transform_query.single_mut();
    let mut cursor_nearest_reticle_visibility =
        cursor_nearest_reticle_visibility_query.single_mut();
    let mut cursor_nearest_entity = None;
    let mut cursor_target_onscreen = false;
    let mut cursor_nearest = Vec2 {
        x: 10000000.0,
        y: 10000000.0,
    };
    for (_index, (each_valid_target_transform, each_valid_target_entity)) in
        valid_targets_query.iter().enumerate()
    {
        match camera_3d.world_to_viewport(
            camera_3d_global_transform,
            each_valid_target_transform.translation(),
        ) {
            Some(each_object_3d_viewport_position) => {
                match camera_2d.viewport_to_world_2d(
                    camera_2d_global_transform,
                    each_object_3d_viewport_position,
                ) {
                    Some(each_object_2d_viewport_position) => {
                        if each_object_2d_viewport_position.length() < cursor_nearest.length() {
                            cursor_target_onscreen = true;
                            cursor_nearest = each_object_2d_viewport_position;
                            cursor_nearest_entity = Some(each_valid_target_entity);
                        }
                    }
                    None => {}
                }
            }
            None => {}
        }
    }
    if cursor_target_onscreen {
        *cursor_nearest_reticle_visibility = Visibility::Visible;
        cursor_nearest_reticle_transform.translation.x = cursor_nearest.x;
        cursor_nearest_reticle_transform.translation.y = cursor_nearest.y;
    }

    let mut target_object_reticle_transform = target_object_reticle_transform_query.single_mut();

    let mut target_object_reticle_visibility = target_object_reticle_visibility_query.single_mut();

    let Some(camera_2d_viewport_rect) = camera_2d.logical_viewport_rect() else {
        return;
    };

    /* Highlight target with crosshair reticle */
    match target_resource.target {
        Some(target) => match objects.get(target) {
            Ok(target_object) => {
                let (_target_object_scale, _target_object_rotation, target_object_translation) =
                    target_object.to_scale_rotation_translation();
                match camera_3d
                    .world_to_viewport(camera_3d_global_transform, target_object_translation)
                {
                    Some(target_object_viewport_position) => {
                        match (
                            camera_2d_viewport_rect.contains(target_object_viewport_position),
                            camera_2d.viewport_to_world_2d(
                                camera_2d_global_transform,
                                target_object_viewport_position,
                            ),
                        ) {
                            (true, Some(target_object_overlay_position)) => {
                                *target_object_reticle_visibility = Visibility::Visible;
                                target_object_reticle_transform.translation.x =
                                    target_object_overlay_position.x;
                                target_object_reticle_transform.translation.y =
                                    target_object_overlay_position.y;
                            }
                            (false, Some(_target_object_overlay_position)) => {
                                *target_object_reticle_visibility = Visibility::Hidden;
                            }
                            (true, None) => {
                                *target_object_reticle_visibility = Visibility::Hidden;
                            }
                            (false, None) => {
                                *target_object_reticle_visibility = Visibility::Hidden;
                            }
                        }
                    }
                    None => {
                        *target_object_reticle_visibility = Visibility::Hidden;
                    }
                }
            }
            Err(e) => error!("{:?}", e),
        },
        None => {}
    }

    if key.just_pressed(KeyCode::Enter) {
        target_resource.target = cursor_nearest_entity;
        debug!("target_resource.target: {:?}", target_resource.target);
    }
}

fn input_handling(
    mut cam: ResMut<CameraInput>,
    btn: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut exit: EventWriter<AppExit>,
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
}
