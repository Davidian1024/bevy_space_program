use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::view::RenderLayers,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use big_space::IgnoreFloatingOrigin;

#[derive(Component)]
pub enum CrosshairType {
    SmallSquareCorners,
    SmallTriangleArrows45s,
    SmallTriangleArrows90s,
}

impl Default for CrosshairType {
    fn default() -> Self {
        debug!("default crosshair");
        CrosshairType::SmallSquareCorners
    }
}

pub fn spawn_crosshair(
    commands: &mut Commands,
    crosshair_type: CrosshairType,
    meshes: &mut ResMut<Assets<Mesh>>,
    color_materials: &mut ResMut<Assets<ColorMaterial>>,
    render_layers: RenderLayers,
) -> Entity {
    match crosshair_type {
        CrosshairType::SmallSquareCorners => {
            let short_horizontal = Mesh2dHandle(meshes.add(Rectangle::new(10.0, 0.25)));
            let short_vertical = Mesh2dHandle(meshes.add(Rectangle::new(0.25, 10.0)));
            let crosshair_color = color_materials.add(match Color::hex("FE9F00") {
                Ok(c) => c,
                Err(_) => Color::rgb(1.0, 1.0, 1.0),
            });

            commands
                .spawn((
                    CrosshairType::SmallSquareCorners,
                    Transform::default(),
                    GlobalTransform::default(),
                    // Visibility::Hidden,
                    // InheritedVisibility::HIDDEN,
                    IgnoreFloatingOrigin,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        render_layers,
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
                            material: crosshair_color.clone(),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        render_layers,
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
                            material: crosshair_color.clone(),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        render_layers,
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
                            material: crosshair_color.clone(),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        render_layers,
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
                            material: crosshair_color.clone(),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        render_layers,
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
                            material: crosshair_color.clone(),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        render_layers,
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
                            material: crosshair_color.clone(),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        render_layers,
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
                            material: crosshair_color.clone(),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        render_layers,
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
                            material: crosshair_color.clone(),
                            ..default()
                        },
                    ));
                })
                .id()
        }

        CrosshairType::SmallTriangleArrows45s => {
            let small_triangle = Mesh2dHandle(meshes.add(Triangle2d::new(
                Vec2::ZERO,
                Vec2 { x: 10.0, y: 0.0 },
                Vec2 { x: 0.0, y: 10.0 },
            )));
            let camera_reticle_color = color_materials.add(match Color::hex("B2AFC2") {
                Ok(c) => c,
                Err(_) => Color::rgb(1.0, 1.0, 1.0),
            });

            commands
                .spawn((
                    Transform::default(),
                    GlobalTransform::default(),
                    IgnoreFloatingOrigin,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        render_layers,
                        MaterialMesh2dBundle {
                            mesh: small_triangle.clone(),
                            material: camera_reticle_color.clone(),
                            transform: Transform {
                                translation: Vec3 {
                                    x: 10.0,
                                    y: 10.0,
                                    z: 0.0,
                                },
                                ..default()
                            },
                            ..default()
                        },
                    ));
                    parent.spawn((
                        render_layers,
                        MaterialMesh2dBundle {
                            mesh: small_triangle.clone(),
                            material: camera_reticle_color.clone(),
                            transform: Transform {
                                translation: Vec3 {
                                    x: -10.0,
                                    y: 10.0,
                                    z: 0.0,
                                },
                                rotation: Quat::from_rotation_z(PI / 2.0),
                                ..default()
                            },
                            ..default()
                        },
                    ));
                    parent.spawn((
                        render_layers,
                        MaterialMesh2dBundle {
                            mesh: small_triangle.clone(),
                            material: camera_reticle_color.clone(),
                            transform: Transform {
                                translation: Vec3 {
                                    x: -10.0,
                                    y: -10.0,
                                    z: 0.0,
                                },
                                rotation: Quat::from_rotation_z(PI),
                                ..default()
                            },
                            ..default()
                        },
                    ));
                    parent.spawn((
                        render_layers,
                        MaterialMesh2dBundle {
                            mesh: small_triangle.clone(),
                            material: camera_reticle_color.clone(),
                            transform: Transform {
                                translation: Vec3 {
                                    x: 10.0,
                                    y: -10.0,
                                    z: 0.0,
                                },
                                rotation: Quat::from_rotation_z(-PI / 2.0),
                                ..default()
                            },
                            ..default()
                        },
                    ));
                })
                .id()
        }
        CrosshairType::SmallTriangleArrows90s => {
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
                    render_layers,
                    IgnoreFloatingOrigin,
                    // CursorNearestReticle,
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::Hidden,
                    InheritedVisibility::HIDDEN,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        render_layers,
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
                        render_layers,
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
                        render_layers,
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
                        render_layers,
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
                })
                .id()
        }
    }
}
