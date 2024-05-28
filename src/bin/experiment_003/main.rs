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
use bevy_space_program::crosshair::{spawn_crosshair, CrosshairType};
use big_space::{
    camera::{CameraController, CameraInput},
    reference_frame::{ReferenceFrame, RootReferenceFrame},
    world_query::GridTransformReadOnly,
    FloatingOrigin, GridCell, IgnoreFloatingOrigin,
};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum AutomationState {
    Idle,
    FocusingOnTarget,
}

#[derive(Default, Reflect, GizmoConfigGroup)]
struct OverlayGizmos {}

fn main() {
    App::new()
        .insert_state(AutomationState::FocusingOnTarget)
        .add_plugins((
            DefaultPlugins.build().disable::<TransformPlugin>(),
            big_space::FloatingOriginPlugin::<i64>::default(),
            big_space::debug::FloatingOriginDebugPlugin::<i64>::default(),
            big_space::camera::CameraControllerPlugin::<i64>::default(),
            bevy_framepace::FramepacePlugin,
        ))
        .init_gizmo_group::<OverlayGizmos>()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa::Sample8)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 100.0,
        })
        .add_systems(Startup, (setup, ui_text_setup))
        .add_systems(
            Update,
            (
                ui_text_update,
                input_handling,
                update_targeting_overlay,
                rotate,
            ),
        )
        .add_systems(
            Update,
            focus_on_target.run_if(in_state(AutomationState::FocusingOnTarget)),
        )
        .add_systems(
            PostUpdate,
            (update_valid_target_gizmos, update_orbit_gizmos),
        )
        .run()
}

const BACKGROUND: RenderLayers = RenderLayers::layer(1);
const OVERLAY: RenderLayers = RenderLayers::layer(2);

#[derive(Component)]
pub struct ValidTarget;

#[derive(Component)]
pub struct Orbit {
    radius: f32,
    base_color: Color,
}

#[derive(Component)]
pub struct CursorNearestReticle;

#[derive(Component)]
pub struct TargetObjectReticle;

#[derive(Component)]
pub struct HUD;

#[derive(Component)]
pub struct TargetLabel;

#[derive(Component)]
pub struct ComponentInfo {
    name: String,
    size: f32,
}

#[derive(Component)]
struct Rotates(Vec3);

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
    default_gizmo_config.line_width = 2.0;

    let (overlay_gizmo_config, _) = gizmo_config_store.config_mut::<OverlayGizmos>();
    overlay_gizmo_config.render_layers = OVERLAY;
    overlay_gizmo_config.line_width = 0.25;

    /* Overlay Camera */
    commands.spawn((
        OVERLAY,
        IgnoreFloatingOrigin,
        Camera2dBundle {
            camera: Camera {
                order: 2,
                hdr: true,
                ..default()
            },
            camera_2d: Camera2d,
            ..default()
        },
    ));

    spawn_crosshair(
        &mut commands,
        CrosshairType::SmallTriangleArrows45s,
        &mut meshes,
        &mut color_materials,
        OVERLAY,
    );

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
    let long_horizontal = Mesh2dHandle(meshes.add(Rectangle::new(2000.0, 0.25)));
    let long_vertical = Mesh2dHandle(meshes.add(Rectangle::new(0.25, 2000.0)));
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

    let initial_target_entity: Option<Entity>;

    /* Spawn the Sun at (0,0,0) */
    let sun_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: Color::rgb_linear(10000000., 10000000., 10000000.),
        ..default()
    });
    let sun_radius_m = 695_508_000.0;
    let sun_mesh = meshes.add(Sphere::new(sun_radius_m).mesh().ico(16).unwrap());

    commands
        .spawn((
            BACKGROUND,
            GridCell::<i64>::ZERO,
            PointLightBundle {
                point_light: PointLight {
                    intensity: 35.73e28,
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
                ComponentInfo {
                    name: "Sun".to_string(),
                    size: sun_radius_m,
                },
                ValidTarget,
                PbrBundle {
                    mesh: sun_mesh,
                    material: sun_mat,
                    ..default()
                },
            ));
        });

    let mercury_mat = materials.add(StandardMaterial {
        base_color: Color::DARK_GRAY,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    let mercury_radius_m = 2.4397e6;
    let mercury_orbit_radius_m = 57.91e9;
    let mercury_mesh = meshes.add(Sphere::new(mercury_radius_m).mesh().ico(16).unwrap());
    let (mercury_cell, mercury_pos): (GridCell<i64>, _) =
        space.imprecise_translation_to_grid(Vec3::Z * mercury_orbit_radius_m);
    commands.spawn((
        ComponentInfo {
            name: "Mercury".to_string(),
            size: mercury_radius_m,
        },
        BACKGROUND,
        ValidTarget,
        PbrBundle {
            mesh: mercury_mesh,
            material: mercury_mat,
            transform: Transform::from_translation(mercury_pos),
            ..default()
        },
        mercury_cell,
    ));

    commands.spawn((
        BACKGROUND,
        Orbit {
            radius: mercury_orbit_radius_m,
            base_color: Color::DARK_GRAY,
        },
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        GridCell::<i64>::ZERO,
    ));

    let venus_mat = materials.add(StandardMaterial {
        base_color: Color::ORANGE,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    let venus_radius_m = 6.0518e6;
    let venus_orbit_radius_m = 108.21e9;
    let venus_mesh = meshes.add(Sphere::new(venus_radius_m).mesh().ico(16).unwrap());
    let (venus_cell, venus_pos): (GridCell<i64>, _) =
        space.imprecise_translation_to_grid(Vec3::Z * venus_orbit_radius_m);
    commands.spawn((
        ComponentInfo {
            name: "Venus".to_string(),
            size: venus_radius_m,
        },
        BACKGROUND,
        ValidTarget,
        PbrBundle {
            mesh: venus_mesh,
            material: venus_mat,
            transform: Transform::from_translation(venus_pos),
            ..default()
        },
        venus_cell,
    ));
    commands.spawn((
        BACKGROUND,
        Orbit {
            radius: venus_orbit_radius_m,
            base_color: Color::ORANGE,
        },
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        GridCell::<i64>::ZERO,
    ));

    let earth_mat = materials.add(StandardMaterial {
        base_color: Color::BLUE,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    let earth_radius_m = 6.371e6;
    let earth_orbit_radius_m = 149.60e9;
    let earth_mesh = meshes.add(Sphere::new(earth_radius_m).mesh().ico(16).unwrap());
    let (earth_cell, earth_pos): (GridCell<i64>, _) =
        space.imprecise_translation_to_grid(Vec3::Z * earth_orbit_radius_m);
    commands.spawn((
        ComponentInfo {
            name: "Earth".to_string(),
            size: earth_radius_m,
        },
        BACKGROUND,
        ValidTarget,
        PbrBundle {
            mesh: earth_mesh,
            material: earth_mat,
            transform: Transform::from_translation(earth_pos),
            ..default()
        },
        earth_cell,
    ));
    commands.spawn((
        BACKGROUND,
        Orbit {
            radius: earth_orbit_radius_m,
            base_color: Color::BLUE,
        },
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        GridCell::<i64>::ZERO,
    ));

    let mars_mat = materials.add(StandardMaterial {
        base_color: Color::RED,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    let mars_radius_m = 3.3962e6;
    let mars_orbit_radius_m = 228.6e9;
    let mars_mesh = meshes.add(Sphere::new(mars_radius_m).mesh().ico(16).unwrap());
    let (mars_cell, mars_pos): (GridCell<i64>, _) =
        space.imprecise_translation_to_grid(Vec3::Z * mars_orbit_radius_m);
    commands.spawn((
        ComponentInfo {
            name: "Mars".to_string(),
            size: mars_radius_m,
        },
        BACKGROUND,
        ValidTarget,
        PbrBundle {
            mesh: mars_mesh,
            material: mars_mat,
            transform: Transform::from_translation(mars_pos),
            ..default()
        },
        mars_cell,
    ));
    commands.spawn((
        BACKGROUND,
        Orbit {
            radius: mars_orbit_radius_m,
            base_color: Color::RED,
        },
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        GridCell::<i64>::ZERO,
    ));

    let jupiter_mat = materials.add(StandardMaterial {
        base_color: Color::BEIGE,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    let jupiter_radius_m = 71.492e6;
    let jupiter_orbit_radius_m = 778.479e9;
    let jupiter_mesh = meshes.add(Sphere::new(jupiter_radius_m).mesh().ico(16).unwrap());
    let (jupiter_cell, jupiter_pos): (GridCell<i64>, _) =
        space.imprecise_translation_to_grid(Vec3::Z * jupiter_orbit_radius_m);
    commands.spawn((
        ComponentInfo {
            name: "Jupiter".to_string(),
            size: jupiter_radius_m,
        },
        BACKGROUND,
        ValidTarget,
        PbrBundle {
            mesh: jupiter_mesh,
            material: jupiter_mat,
            transform: Transform::from_translation(jupiter_pos),
            ..default()
        },
        jupiter_cell,
    ));
    commands.spawn((
        BACKGROUND,
        Orbit {
            radius: jupiter_orbit_radius_m,
            base_color: Color::BEIGE,
        },
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        GridCell::<i64>::ZERO,
    ));

    let saturn_mat = materials.add(StandardMaterial {
        base_color: Color::BEIGE,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    let saturn_radius_m = 58.232e6;
    let saturn_orbit_radius_m = 1433.525e9;
    let saturn_mesh = meshes.add(Sphere::new(saturn_radius_m).mesh().ico(16).unwrap());
    let (saturn_cell, saturn_pos): (GridCell<i64>, _) =
        space.imprecise_translation_to_grid(Vec3::Z * saturn_orbit_radius_m);
    initial_target_entity = Some(
        commands
            .spawn((
                ComponentInfo {
                    name: "Saturn".to_string(),
                    size: saturn_radius_m,
                },
                BACKGROUND,
                ValidTarget,
                PbrBundle {
                    mesh: saturn_mesh,
                    material: saturn_mat,
                    transform: Transform::from_translation(saturn_pos),
                    ..default()
                },
                saturn_cell,
            ))
            .id(),
    );
    let saturn_rings_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        cull_mode: None,
        ..default()
    });
    let saturn_rings_radius_m = 100e6;
    let saturn_rings_mesh = meshes.add(Circle::new(saturn_rings_radius_m).mesh().resolution(128));
    commands.spawn((
        BACKGROUND,
        PbrBundle {
            mesh: saturn_rings_mesh.clone(),
            material: saturn_rings_mat.clone(),
            transform: Transform::from_translation(saturn_pos)
                .with_rotation(Quat::from_rotation_y((PI / 4.0) - PI)),
            ..default()
        },
        saturn_cell,
    ));
    commands.spawn((
        BACKGROUND,
        Orbit {
            radius: saturn_orbit_radius_m,
            base_color: Color::BEIGE,
        },
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        GridCell::<i64>::ZERO,
    ));

    let uranus_mat = materials.add(StandardMaterial {
        base_color: Color::CYAN,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    let uranus_radius_m = 25.559e6;
    let uranus_orbit_radius_m = 2870.975e9;
    let uranus_mesh = meshes.add(Sphere::new(uranus_radius_m).mesh().ico(16).unwrap());
    let (uranus_cell, uranus_pos): (GridCell<i64>, _) =
        space.imprecise_translation_to_grid(Vec3::Z * uranus_orbit_radius_m);
    commands.spawn((
        ComponentInfo {
            name: "Uranus".to_string(),
            size: uranus_radius_m,
        },
        BACKGROUND,
        ValidTarget,
        PbrBundle {
            mesh: uranus_mesh,
            material: uranus_mat,
            transform: Transform::from_translation(uranus_pos),
            ..default()
        },
        uranus_cell,
    ));
    commands.spawn((
        BACKGROUND,
        Orbit {
            radius: uranus_orbit_radius_m,
            base_color: Color::CYAN,
        },
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        GridCell::<i64>::ZERO,
    ));

    let neptune_mat = materials.add(StandardMaterial {
        base_color: Color::BLUE,
        perceptual_roughness: 0.8,
        reflectance: 1.0,
        ..default()
    });
    let neptune_radius_m = 24.764e6;
    let neptune_orbit_radius_m = 4500e9;
    let neptune_mesh = meshes.add(Sphere::new(neptune_radius_m).mesh().ico(16).unwrap());
    let (neptune_cell, neptune_pos): (GridCell<i64>, _) =
        space.imprecise_translation_to_grid(Vec3::Z * neptune_orbit_radius_m);
    commands.spawn((
        ComponentInfo {
            name: "Neptune".to_string(),
            size: neptune_radius_m,
        },
        BACKGROUND,
        ValidTarget,
        PbrBundle {
            mesh: neptune_mesh,
            material: neptune_mat,
            transform: Transform::from_translation(neptune_pos),
            ..default()
        },
        neptune_cell,
    ));
    commands.spawn((
        BACKGROUND,
        Orbit {
            radius: neptune_orbit_radius_m,
            base_color: Color::BLUE,
        },
        Transform::IDENTITY,
        GlobalTransform::IDENTITY,
        GridCell::<i64>::ZERO,
    ));

    /* Spawn the user controlled camera */
    let (cam_cell, cam_pos): (GridCell<i64>, _) = space.translation_to_grid(DVec3 {
        x: (sun_radius_m as f64 * 20.0) + 1000.0,
        y: (sun_radius_m as f64 * 20.0) + 1000.0,
        z: (sun_radius_m as f64 * 20.0) + 1000.0,
    });
    commands.spawn((
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
    ));

    let home_object_mat = materials.add(StandardMaterial {
        base_color: Color::PURPLE,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    let home_object_size_m = 1.0;
    let home_object_distance_m = sun_radius_m * 20.0;
    let (home_object_cell, _home_object_pos): (GridCell<i64>, _) =
        space.translation_to_grid(DVec3::splat(home_object_distance_m as f64));
    let home_object_mesh = meshes.add(Cuboid::new(
        home_object_size_m,
        home_object_size_m,
        home_object_size_m,
    ));
    commands.spawn((
        ComponentInfo {
            name: "Home".to_string(),
            size: home_object_size_m,
        },
        ValidTarget,
        BACKGROUND,
        PbrBundle {
            mesh: home_object_mesh,
            material: home_object_mat,
            transform: Transform::IDENTITY,
            ..default()
        },
        home_object_cell,
        ReferenceFrame::<i64>::default(),
        Rotates(Vec3 {
            x: 0.0201,
            y: 0.021,
            z: 0.0210001,
        }),
    ));

    commands.insert_resource(TargetResource {
        target: initial_target_entity,
    });

    /* Proxima Centauri 4.017 × 10^16 m */
    let proxima_centauri_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: Color::rgb_linear(10000000., 10000000., 10000000.),
        ..default()
    });
    let proxima_centauri_radius_m = sun_radius_m * 0.1542;
    let proxima_centauri_distance_m = 4.017e16;
    let proxima_centauri_mesh = meshes.add(
        Sphere::new(proxima_centauri_radius_m)
            .mesh()
            .ico(16)
            .unwrap(),
    );
    let (proxima_centauri_grid_cell, proxima_centauri_grid_pos): (GridCell<i64>, _) =
        space.translation_to_grid(Vec3::Z * proxima_centauri_distance_m);
    commands.spawn((
        BACKGROUND,
        proxima_centauri_grid_cell,
        PointLightBundle {
            transform: Transform::from_translation(proxima_centauri_grid_pos),
            point_light: PointLight {
                intensity: 35.73e28,
                range: 1e20,
                radius: proxima_centauri_radius_m,
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
    ));
    commands.spawn((
        BACKGROUND,
        proxima_centauri_grid_cell,
        ComponentInfo {
            name: "Proxima Centauri".to_string(),
            size: proxima_centauri_radius_m,
        },
        ValidTarget,
        PbrBundle {
            transform: Transform::from_translation(proxima_centauri_grid_pos),
            mesh: proxima_centauri_mesh,
            material: proxima_centauri_mat,
            ..default()
        },
    ));
}

fn ui_text_setup(mut commands: Commands) {
    commands.spawn((
        BACKGROUND,
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
        HUD,
        IgnoreFloatingOrigin,
    ));

    commands.spawn((
        BACKGROUND,
        TargetLabel,
        TextBundle {
            visibility: Visibility::Hidden,
            inherited_visibility: InheritedVisibility::HIDDEN,
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(100.0),
                left: Val::Px(10.0),
                ..default()
            },
            text: Text {
                sections: vec![TextSection {
                    value: "Test Label".to_string(),
                    style: TextStyle {
                        font_size: 18.0,
                        color: Color::ORANGE,
                        ..default()
                    },
                }],
                justify: JustifyText::Left,
                ..default()
            },
            ..default()
        },
        IgnoreFloatingOrigin,
    ));
}

fn ui_text_update(
    floating_origin_grid_transform_query: Query<
        (&Transform, GridTransformReadOnly<i64>),
        With<FloatingOrigin>,
    >,
    camera_controller_query: Query<&CameraController>,
    mut hud_text_query: Query<&mut Text, With<HUD>>,
    time: Res<Time>,
    target_resource: ResMut<TargetResource>,
    component_info_query: Query<&ComponentInfo>,
) {
    let (camera_3d_transform, floating_origin_grid_transform) =
        floating_origin_grid_transform_query.single();
    let grid_text = format!(
        "X:{:_>15} Y:{:_>15} Z:{:_>15}",
        floating_origin_grid_transform.cell.x,
        floating_origin_grid_transform.cell.y,
        floating_origin_grid_transform.cell.z
    );

    let mut target_entity_name = "none";
    match target_resource.target {
        Some(target_entity) => match component_info_query.get(target_entity) {
            Ok(target_entity_component_info) => {
                target_entity_name = &target_entity_component_info.name;
            }
            Err(e) => error!("match component_info_query.get(target_entity) {:?}", e),
        },
        None => {}
    }

    let camera_coordinates = camera_3d_transform.translation;
    let camera_controller = camera_controller_query.single();
    let (velocity, _) = camera_controller.velocity();
    let speed = velocity.length() / time.delta_seconds_f64();
    let speed_text = if speed > 3.0e8 {
        format!("{:.0e} * speed of light", speed / 3.0e8)
    } else {
        format!("{:.2e} m/s", speed)
    };
    let mut hud_text = hud_text_query.single_mut();
    let hud_text_string = format!(
        "Speed: {}\nGrid Coordinates: {}\nCell Coordinates: X:{:_>15} Y:{:_>15} Z:{:_>15}\nTracking: {}",
        speed_text,
        grid_text,
        camera_coordinates.x,
        camera_coordinates.y,
        camera_coordinates.z,
        target_entity_name
    );
    hud_text.sections[0].value = hud_text_string.clone();
}

fn update_valid_target_gizmos(
    global_transform_query: Query<&GlobalTransform>,
    valid_target_entity_query: Query<Entity, With<ValidTarget>>,
    mut overlay_gizmos: Gizmos<OverlayGizmos>,
    camera_3d_query: Query<(&mut Camera, &GlobalTransform), (With<Camera3d>, Without<Camera2d>)>,
    camera_2d_query: Query<(&mut Camera, &GlobalTransform), (With<Camera2d>, Without<Camera3d>)>,
) {
    for each_valid_target_entity in valid_target_entity_query.iter() {
        let Ok(transform) = global_transform_query.get(each_valid_target_entity) else {
            return;
        };
        let (_scale, _rotationn, translation) = transform.to_scale_rotation_translation();

        let (camera_3d, camera_3d_global_transform) = camera_3d_query.single();
        let (camera_2d, camera_2d_global_transform) = camera_2d_query.single();
        match camera_3d.world_to_viewport(camera_3d_global_transform, translation) {
            Some(each_valid_target_viewport_position) => {
                match camera_2d.viewport_to_world_2d(
                    camera_2d_global_transform,
                    each_valid_target_viewport_position,
                ) {
                    Some(each_valid_target_world_2d_position) => {
                        let color = match Color::hex("FE9F00") {
                            Ok(c) => c,
                            Err(_) => Color::rgb(1.0, 1.0, 1.0),
                        };
                        overlay_gizmos.linestrip_2d(
                            vec![
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x + 25.0,
                                    y: each_valid_target_world_2d_position.y + 30.0,
                                },
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x + 30.0,
                                    y: each_valid_target_world_2d_position.y + 30.0,
                                },
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x + 30.0,
                                    y: each_valid_target_world_2d_position.y + 25.0,
                                },
                            ],
                            color,
                        );
                        overlay_gizmos.linestrip_2d(
                            vec![
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x + 30.0,
                                    y: each_valid_target_world_2d_position.y - 25.0,
                                },
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x + 30.0,
                                    y: each_valid_target_world_2d_position.y - 30.0,
                                },
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x + 25.0,
                                    y: each_valid_target_world_2d_position.y - 30.0,
                                },
                            ],
                            color,
                        );
                        overlay_gizmos.linestrip_2d(
                            vec![
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x - 25.0,
                                    y: each_valid_target_world_2d_position.y + 30.0,
                                },
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x - 30.0,
                                    y: each_valid_target_world_2d_position.y + 30.0,
                                },
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x - 30.0,
                                    y: each_valid_target_world_2d_position.y + 25.0,
                                },
                            ],
                            color,
                        );
                        overlay_gizmos.linestrip_2d(
                            vec![
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x - 30.0,
                                    y: each_valid_target_world_2d_position.y - 25.0,
                                },
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x - 30.0,
                                    y: each_valid_target_world_2d_position.y - 30.0,
                                },
                                Vec2 {
                                    x: each_valid_target_world_2d_position.x - 25.0,
                                    y: each_valid_target_world_2d_position.y - 30.0,
                                },
                            ],
                            color,
                        );
                    }
                    None => {}
                }
            }
            None => {}
        }
    }
}

fn update_orbit_gizmos(
    global_transform_query: Query<&GlobalTransform>,
    orbit_entity_query: Query<(Entity, &Orbit)>,
    mut default_gizmos: Gizmos,
) {
    for (each_entity, each_orbit) in orbit_entity_query.iter() {
        let Ok(transform) = global_transform_query.get(each_entity) else {
            return;
        };
        let (_scale, _rotationn, translation) = transform.to_scale_rotation_translation();
        match Direction3d::from_xyz(transform.up().x, transform.up().y, transform.up().z) {
            Ok(d) => {
                default_gizmos
                    .circle(translation, d, each_orbit.radius, each_orbit.base_color)
                    .segments(64);
            }
            Err(e) => error!("{:?}", e),
        }
    }
}

fn update_targeting_overlay(
    camera_3d_query: Query<(&mut Camera, &GlobalTransform), (With<Camera3d>, Without<Camera2d>)>,
    camera_2d_query: Query<(&mut Camera, &GlobalTransform), (With<Camera2d>, Without<Camera3d>)>,
    valid_targets_query: Query<(&GlobalTransform, Entity, &ComponentInfo), With<ValidTarget>>,
    mut target_resource: ResMut<TargetResource>,
    mut cursor_nearest_reticle_transform_query: Query<
        &mut Transform,
        (
            With<CursorNearestReticle>,
            Without<Camera3d>,
            Without<Camera2d>,
            Without<TargetLabel>,
            Without<CrosshairType>,
        ),
    >,
    mut target_object_reticle_transform_query: Query<
        &mut Transform,
        (
            With<TargetObjectReticle>,
            Without<CursorNearestReticle>,
            Without<Camera3d>,
            Without<Camera2d>,
            Without<TargetLabel>,
            Without<CrosshairType>,
        ),
    >,
    mut target_label_style_query: Query<(&mut Style, &mut Text), With<TargetLabel>>,
    cursor_nearest_entity_query: Query<Entity, With<CursorNearestReticle>>,
    target_object_reticle_entity_query: Query<Entity, With<TargetObjectReticle>>,
    target_label_entity_query: Query<Entity, With<TargetLabel>>,
    global_transform_query: Query<&GlobalTransform>,
    mut visibility_query: Query<&mut Visibility>,
    key: Res<ButtonInput<KeyCode>>,
) {
    let cursor_nearest_entity = cursor_nearest_entity_query.single();
    let target_object_reticle_entity = target_object_reticle_entity_query.single();
    let target_label_entity = target_label_entity_query.single();

    let (camera_3d, camera_3d_global_transform) = camera_3d_query.single();
    let (camera_2d, camera_2d_global_transform) = camera_2d_query.single();

    /* Highlight object nearest to cursor (center of screen) with small reticle */
    let mut cursor_nearest_reticle_transform = cursor_nearest_reticle_transform_query.single_mut();
    let visibility_entity_results = visibility_query.get_many_mut([
        cursor_nearest_entity,
        target_object_reticle_entity,
        target_label_entity,
    ]);
    match visibility_entity_results {
        Ok(mut visibility_entities) => {
            let first_visibility_entities_split = visibility_entities.split_at_mut(1);
            let cursor_nearest_reticle_visibility = first_visibility_entities_split.0;
            let second_visibility_entities_split =
                first_visibility_entities_split.1.split_at_mut(1);
            let target_object_reticle_visibility = second_visibility_entities_split.0;
            let target_label_visibility = second_visibility_entities_split.1;

            let mut cursor_nearest_entity = None;
            let mut cursor_target_onscreen = false;
            let mut cursor_nearest_size = 0.0;
            let mut cursor_nearest = Vec2 {
                x: 10000000.0,
                y: 10000000.0,
            };
            for (
                _index,
                (each_valid_target_transform, each_valid_target_entity, each_valid_target_info),
            ) in valid_targets_query.iter().enumerate()
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
                                trace!(
                                    "{:?} {:?}",
                                    each_valid_target_info.name,
                                    each_object_2d_viewport_position
                                );

                                let length_difference = each_object_2d_viewport_position.length()
                                    - cursor_nearest.length();
                                if length_difference < 0.0 {
                                    if length_difference > -3.0 {
                                        if each_valid_target_info.size > cursor_nearest_size {
                                            cursor_target_onscreen = true;
                                            cursor_nearest = each_object_2d_viewport_position;
                                            cursor_nearest_entity = Some(each_valid_target_entity);
                                            cursor_nearest_size = each_valid_target_info.size;
                                        }
                                    } else {
                                        cursor_target_onscreen = true;
                                        cursor_nearest = each_object_2d_viewport_position;
                                        cursor_nearest_entity = Some(each_valid_target_entity);
                                        cursor_nearest_size = each_valid_target_info.size;
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                    None => {}
                }
            }
            if cursor_target_onscreen {
                *cursor_nearest_reticle_visibility[0] = Visibility::Visible;
                cursor_nearest_reticle_transform.translation.x = cursor_nearest.x;
                cursor_nearest_reticle_transform.translation.y = cursor_nearest.y;
            }

            let mut target_object_reticle_transform =
                target_object_reticle_transform_query.single_mut();

            let Some(camera_2d_viewport_rect) = camera_2d.logical_viewport_rect() else {
                return;
            };

            match target_resource.target {
                Some(target) => match global_transform_query.get(target) {
                    Ok(target_object) => {
                        let (
                            _target_object_scale,
                            _target_object_rotation,
                            target_object_translation,
                        ) = target_object.to_scale_rotation_translation();
                        match camera_3d.world_to_viewport(
                            camera_3d_global_transform,
                            target_object_translation,
                        ) {
                            Some(target_object_viewport_position) => {
                                match (
                                    camera_2d_viewport_rect
                                        .contains(target_object_viewport_position),
                                    camera_2d.viewport_to_world_2d(
                                        camera_2d_global_transform,
                                        target_object_viewport_position,
                                    ),
                                ) {
                                    (true, Some(target_object_overlay_position)) => {
                                        *target_object_reticle_visibility[0] = Visibility::Visible;
                                        target_object_reticle_transform.translation.x =
                                            target_object_overlay_position.x;
                                        target_object_reticle_transform.translation.y =
                                            target_object_overlay_position.y;

                                        *target_label_visibility[0] = Visibility::Visible;
                                        let (mut target_label_style, mut target_label_text) =
                                            target_label_style_query.single_mut();

                                        target_label_style.top =
                                            Val::Px(target_object_viewport_position.y + 30.0);
                                        target_label_style.left =
                                            Val::Px(target_object_viewport_position.x + 30.0);

                                        match valid_targets_query.get(target) {
                                            Ok((_, _, target_component_info)) => {
                                                target_label_text.sections[0].value =
                                                    target_component_info.name.to_string();
                                            }
                                            Err(e) => error!(
                                                "match valid_targets_query.get(target) {:?}",
                                                e
                                            ),
                                        }
                                    }
                                    (false, Some(_target_object_overlay_position)) => {
                                        *target_object_reticle_visibility[0] = Visibility::Hidden;
                                        *target_label_visibility[0] = Visibility::Hidden;
                                    }
                                    (true, None) => {
                                        *target_object_reticle_visibility[0] = Visibility::Hidden;
                                        *target_label_visibility[0] = Visibility::Hidden;
                                    }
                                    (false, None) => {
                                        *target_object_reticle_visibility[0] = Visibility::Hidden;
                                        *target_label_visibility[0] = Visibility::Hidden;
                                    }
                                }
                            }
                            None => {
                                *target_object_reticle_visibility[0] = Visibility::Hidden;
                                *target_label_visibility[0] = Visibility::Hidden;
                            }
                        }
                    }
                    Err(e) => error!("{:?}", e),
                },
                None => {}
            }

            /* Highlight target with crosshair reticle */
            if key.just_pressed(KeyCode::Enter) {
                target_resource.target = cursor_nearest_entity;
            }
        }
        Err(e) => error!("match visibility_entity_results {:?}", e),
    }
}

fn rotate(mut rotate_query: Query<(&mut Transform, &Rotates)>) {
    for (mut transform, rotates) in rotate_query.iter_mut() {
        transform.rotate_x(rotates.0.x);
        transform.rotate_y(rotates.0.y);
        transform.rotate_z(rotates.0.z);
    }
}

fn input_handling(
    mut cam: ResMut<CameraInput>,
    btn: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut exit: EventWriter<AppExit>,
    current_state: Res<State<AutomationState>>,
    mut state: ResMut<NextState<AutomationState>>,
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

    if key.just_pressed(KeyCode::KeyF) {
        debug!("auto focus:");
        match current_state.get() {
            AutomationState::Idle => {
                debug!("enabled");
                state.set(AutomationState::FocusingOnTarget);
            }
            AutomationState::FocusingOnTarget => {
                debug!("disabled");
                state.set(AutomationState::Idle);
            }
        }
    }
}

fn focus_on_target(
    mut camera_3d_query: Query<
        &mut Transform,
        (With<CameraController>, With<Camera3d>, Without<Camera2d>),
    >,
    target_resource: ResMut<TargetResource>,
    global_transform_query: Query<&GlobalTransform>,
    mut state: ResMut<NextState<AutomationState>>,
) {
    let mut camera_3d_transform = camera_3d_query.single_mut();
    match target_resource.target {
        Some(target) => match global_transform_query.get(target) {
            Ok(target_object) => {
                let target_rotation = camera_3d_transform
                    .looking_at(
                        target_object.translation(),
                        camera_3d_transform.up().normalize(),
                    )
                    .rotation;
                let rotation_difference = target_rotation * camera_3d_transform.rotation.inverse();
                let (rotation_axis, mut rotation_angle) = rotation_difference.to_axis_angle();
                if rotation_angle > PI {
                    rotation_angle -= PI * 2.0;
                };
                trace!("rotation_axis: {:?}", rotation_axis);
                trace!("rotation_angle: {:?}", rotation_angle);
                let mut new_transform = camera_3d_transform.clone();
                new_transform.rotate_axis(rotation_axis, 0.01 * rotation_angle.signum());
                let angle_between = target_rotation
                    .normalize()
                    .angle_between(camera_3d_transform.rotation.normalize());
                trace!("angle_between: {:?}", angle_between);
                if angle_between < 0.01 {
                    camera_3d_transform.rotation = target_rotation;
                    debug!("target aligned");
                    state.set(AutomationState::Idle);
                } else {
                    camera_3d_transform.rotation = new_transform.rotation;
                }
            }
            Err(e) => error!("match global_transform_query.get(target) {:?}", e),
        },
        None => {}
    }
}
