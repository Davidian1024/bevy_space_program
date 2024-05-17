const NAME: &str = "Bevy Space Program";

const EARTH_RADIUS: f32 = 6_371.0;

const CAMERA_STRAFE_SPEED: f32 = 0.1;
const CAMERA_MOVEMENT_SPEED: f32 = 0.1;
const CAMERA_ROLL_SPEED: f32 = 0.1;
const CAMERA_ZOOM_SPEED: f32 = 1.1;
const CAMERA_ZOOM_MINIMUM:f32 = PI/2.0;
const CAMERA_ZOOM_MAXIMUM:f32 = PI/1000.0;

use std::f32::consts::PI;

use bevy::{app::AppExit, input::mouse::{MouseMotion, MouseWheel}, log::Level, prelude::*, utils::tracing::span};
use bevy_rapier3d::prelude::*;
use rand::Rng;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Loading,
    Generating,
    Spawning,
    Running,
}

fn main() {
    println!("main() start");
    App::new()
        .insert_state(AppState::Loading)
        .insert_resource(ClearColor(Color::rgb(0.1, 0.0, 0.15)))
        .insert_resource(AmbientLight {
            color: Color::default(),
            brightness: 100.0,
        })
        .insert_resource(Msaa::Sample8)
        .add_plugins(DefaultPlugins.set(
            WindowPlugin {
                primary_window: Some(
                    Window {
                        title: String::from(NAME),
                        name: Some(String::from(NAME)),
                        ..default()
                    }
                ),
                ..default()
            }
        ))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin {
            enabled: false,
            style: DebugRenderStyle {
                ..default()
            },
            mode: DebugRenderMode::default(),
        })
        .add_systems(Startup, initiate_asset_loading)
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, app_loading.run_if(in_state(AppState::Loading)))
        .add_systems(Update, generate_resources.run_if(in_state(AppState::Generating)))
        .add_systems(Update, initiate_spawning.run_if(in_state(AppState::Spawning)))
        .add_systems(Update, run_app.run_if(in_state(AppState::Running)))
        .add_systems(Update, camera_controls.run_if(in_state(AppState::Running)))        
        .add_systems(Update, state_controls.run_if(in_state(AppState::Running)))        
        .add_systems(Update, app_controls)
        .run();
    println!("main() stop");
}

#[derive(Resource, Debug, Default)]
pub struct MeshAssets {
    pub torus_mesh: Handle<Mesh>,
    pub command_pod_mesh: Handle<Mesh>,
    pub earth_mesh: Handle<Mesh>,
}

#[derive(Resource, Debug, Default)]
pub struct SceneAssets {
    pub torus_scene: Handle<Scene>,
    pub command_pod_scene: Handle<Scene>,
    pub earth_scene: Handle<Scene>,
}

#[derive(Resource, Debug, Default)]
pub struct ColliderAssets {
    pub torus_collider: Collider,
    pub command_pod_collider: Collider,
    pub earth_collider: Collider,
}

#[derive(Component)]
pub struct Torus;

#[derive(Component)]
pub struct CommandPod;

#[derive(Component)]
pub struct EarthPod;

#[derive(Component)]
pub struct TheCamera;

fn initiate_asset_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let span = span!(Level::INFO, "initiate_asset_loading()");
    let _enter = span.enter();
    debug!("start");
    commands.insert_resource(MeshAssets {
        command_pod_mesh: asset_server.load("experiment_001/command_pod.glb#Mesh0/Primitive0"),
        torus_mesh: asset_server.load("experiment_001/torus.glb#Mesh0/Primitive0"),
        earth_mesh: asset_server.load("experiment_001/earth.glb#Mesh0/Primitive0"),
    });
    commands.insert_resource(SceneAssets {
        command_pod_scene: asset_server.load("experiment_001/command_pod.glb#Scene0"),
        torus_scene: asset_server.load("experiment_001/torus.glb#Scene0"),
        earth_scene: asset_server.load("experiment_001/earth.glb#Scene0"),
    });
    debug!("stop");
}

fn app_loading(
    meshes: Res<Assets<Mesh>>,
    mesh_assets: Res<MeshAssets>,
    scenes: Res<Assets<Scene>>,
    scene_assets: Res<SceneAssets>,
    mut state: ResMut<NextState<AppState>>,
) {
    let span = span!(Level::INFO, "app_loading()");
    let _enter = span.enter();
    debug!("start");
    let command_pod_scene_option = scenes.get(&scene_assets.command_pod_scene);
    let torus_scene_option = scenes.get(&scene_assets.torus_scene);
    let earth_scene_option = scenes.get(&scene_assets.earth_scene);
    let command_pod_mesh_option = meshes.get(&mesh_assets.command_pod_mesh);
    let torus_mesh_option = meshes.get(&mesh_assets.torus_mesh);
    let earth_mesh_option = meshes.get(&mesh_assets.earth_mesh);
    let mut scenes_loaded = false;
    match (command_pod_scene_option, torus_scene_option, earth_scene_option) {
        (Some(_), Some(_), Some(_)) => {
            debug!("scenes loaded");
            scenes_loaded = true;
        },
        _ => print!("."),
    }
    let mut meshes_loaded = false;
    match (command_pod_mesh_option, torus_mesh_option, earth_mesh_option) {
        (Some(_), Some(_), Some(_)) => {
            debug!("meshes loaded");
            meshes_loaded = true;
        },
        _ => print!("."),
    }
    if scenes_loaded && meshes_loaded {
        debug!("loading complete");
        state.set(AppState::Generating);
    }
    debug!("stop");
}

fn generate_resources(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    mesh_assets: Res<MeshAssets>,
    mut state: ResMut<NextState<AppState>>,
) {
    let span = span!(Level::INFO, "generate_resources()");
    let _enter = span.enter();
    debug!("start");
    let command_pod_mesh = meshes.get(&mesh_assets.command_pod_mesh);
    debug!("got command pod mesh");
    let torus_mesh = meshes.get(&mesh_assets.torus_mesh);
    debug!("got torus mesh");
    let earth_mesh = meshes.get(&mesh_assets.earth_mesh);
    debug!("got earth mesh");
    let command_pod_collider = Collider::from_bevy_mesh(command_pod_mesh.expect("command_pod_mesh"), &ComputedColliderShape::ConvexDecomposition(VHACDParameters::default()));
    debug!("generated command pod collider");
    let torus_collider = Collider::from_bevy_mesh(torus_mesh.expect("torus_mesh"), &ComputedColliderShape::ConvexDecomposition(VHACDParameters::default()));
    debug!("generated torus collider");
    let earth_collider = Collider::from_bevy_mesh(earth_mesh.expect("earth_mesh"), &ComputedColliderShape::ConvexDecomposition(VHACDParameters::default()));
    debug!("generated earth collider");
    match (
        command_pod_collider,
        torus_collider,
        earth_collider,
    ) {
        (Some(cp), Some(t), Some(e)) => {
            commands.insert_resource(
                ColliderAssets { 
                    command_pod_collider: cp,
                    torus_collider: t,
                    earth_collider: e,
                }
            );
        },
        _ => {},
    }
    state.set(AppState::Spawning);
    debug!("stop");
}

fn spawn_camera(
    mut commands: Commands
) {
    let span = span!(Level::INFO, "spawn_camera()");
    let _enter = span.enter();
    debug!("start");
    commands
        .spawn((
            Camera3dBundle {
                transform:
                    Transform::from_xyz(20.0, EARTH_RADIUS + 2.0, 0.0).looking_at(Vec3 { x: 0.0, y: EARTH_RADIUS + 2.0, z: 0.0 }, Vec3::Y),
                camera: Camera {
                    ..default()
                },
                projection: Projection::Perspective(PerspectiveProjection {
                    near: 0.01,
                    ..default()
                }),
                camera_3d: Camera3d {
                    ..default()
                },
                ..default()
            },
        ))
        .insert(TheCamera);
    debug!("stop");
}

fn initiate_spawning(
    mut commands: Commands,
    scene_assets: Res<SceneAssets>,
    collider_assets: Res<ColliderAssets>,
    mut state: ResMut<NextState<AppState>>,
) {
    let span = span!(Level::INFO, "initiate_spawning()");
    let _enter = span.enter();
    debug!("start");

    /* Let there be light. */
    commands
        .spawn(
            DirectionalLightBundle {
                directional_light: DirectionalLight {
                    illuminance: 1000.0,
                    shadows_enabled: true,
                    ..default()
                },
                ..default()
            }
        )
        .insert(TransformBundle::from(Transform::from_rotation(Quat::from_rotation_x(-(PI/2.0 + PI/4.0)))))
        ;

    /* Create the Earth. */
    commands
        .spawn((
            collider_assets.earth_collider.clone(),
        ))
        .insert((
            SceneBundle { scene: scene_assets.earth_scene.clone(), ..default() },
        ))
        .insert(Restitution::coefficient(0.1))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 0.0, 0.0)));


    /* Create the command pod. */
    commands
        .spawn((
            CommandPod,
            SceneBundle {
                scene: scene_assets.command_pod_scene.clone(),
                ..default()
            },
            RigidBody::Dynamic,
            collider_assets.command_pod_collider.clone(),
        ))
        .insert(
            TransformBundle::from_transform(
                Transform::from_xyz(0.0, EARTH_RADIUS + 2.0, 0.0)
                // * Transform::from_scale(Vec3 { x: 100.0, y: 100.0, z: 100.0 })
            )
        );

    /* Create a chain. */
    for i in 0..100 {
        commands
            .spawn((
                RigidBody::Dynamic,
            ))
            .insert((
                SceneBundle { scene: scene_assets.torus_scene.clone(), ..default() },
            ))
            .insert(collider_assets.torus_collider.clone())
            .insert(Restitution::coefficient(0.01))
            .insert(Friction::coefficient(4.0))
            .insert(Velocity {
                linvel: Vec3 {
                    x: (rand::thread_rng().gen_range(0..100) as f32) / 100.0,
                    y: (rand::thread_rng().gen_range(0..100) as f32) / 100.0,
                    z: (rand::thread_rng().gen_range(0..100) as f32) / 100.0,
                },
                angvel: Vec3 { x: 0.0, y: 2.0, z: 0.0 },
            })
            .insert(Torus)
            .insert(TransformBundle::from(
                Transform::from_xyz(0.0, EARTH_RADIUS + 100.0 - ((i as f32) / 1.9), 0.0)
                * Transform::from_rotation(Quat::from_rotation_y(PI/2.0 * (i as f32)))
            ));
    }

    state.set(AppState::Running);
    debug!("stop");
}

fn run_app(
    positions: Query<&Transform, With<RigidBody>>,
) {
    let span = span!(Level::INFO, "run_app()");
    let _enter = span.enter();
    debug!("start");
    for transform in positions.iter() {
        debug!("Altitude: {}", transform.translation.y);
    }
    debug!("stop");
}

fn camera_controls(
    mut camera_transform_query: Query<&mut Transform, (With<TheCamera>, Without<CommandPod>)>,
    mut camera_projection_query: Query<&mut Projection, (With<TheCamera>, Without<CommandPod>)>,
    pod_transform_query: Query<&Transform, (With<CommandPod>, Without<TheCamera>)>,
    keyboard_button_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_event_reader: EventReader<MouseMotion>,
    mut mouse_wheel_event_reader: EventReader<MouseWheel>,
    time: Res<Time>,
) {
    let span = span!(Level::DEBUG, "camera_controls()");
    let _enter = span.enter();
    debug!("start");

    let Ok(mut camera_transform) = camera_transform_query.get_single_mut() else {
        error!("query failed to return camera transform?");
        return;
    };

    let Projection::Perspective(camera_perspective) = camera_projection_query.single_mut().into_inner() else {
        return;
    };

    let mut strafe = 0.0;
    let mut roll = 0.0;
    let mut thrust = 0.0;
    let mut rise = 0.0;

    if mouse_button_input.pressed(MouseButton::Right) {
        for each_mouse_motion_event in mouse_motion_event_reader.read() {
            camera_transform.rotate_local_y((each_mouse_motion_event.delta.x / 10.0) * time.delta_seconds());
            camera_transform.rotate_local_x((each_mouse_motion_event.delta.y / 10.0) * time.delta_seconds());
        }
    }

    if mouse_button_input.pressed(MouseButton::Middle) {
        camera_perspective.fov = PI/4.0;
    }

    for each_mouse_wheel_event in mouse_wheel_event_reader.read() {
        info!("mouse wheel event: {:?}", each_mouse_wheel_event);
        let current_fov = camera_perspective.fov;
        let mut desired_fov = current_fov;
        info!("fov before: {:?}", current_fov);
        if each_mouse_wheel_event.y > 0.0 { desired_fov /= CAMERA_ZOOM_SPEED; };
        if each_mouse_wheel_event.y < 0.0 { desired_fov *= CAMERA_ZOOM_SPEED; };
        if desired_fov > CAMERA_ZOOM_MINIMUM { desired_fov = CAMERA_ZOOM_MINIMUM; }
        if desired_fov < CAMERA_ZOOM_MAXIMUM { desired_fov = CAMERA_ZOOM_MAXIMUM; }
        camera_perspective.fov = desired_fov;
        info!("fov after: {:?}", desired_fov);
    }

    if keyboard_button_input.pressed(KeyCode::KeyD) {
        strafe = -CAMERA_STRAFE_SPEED * time.delta_seconds();
    } else if keyboard_button_input.pressed(KeyCode::KeyA) {
        strafe = CAMERA_STRAFE_SPEED * time.delta_seconds();
    }

    if keyboard_button_input.pressed(KeyCode::KeyS) {
        thrust = -CAMERA_MOVEMENT_SPEED * time.delta_seconds();
    } else if keyboard_button_input.pressed(KeyCode::KeyW) {
        thrust = CAMERA_MOVEMENT_SPEED * time.delta_seconds();
    }

    if keyboard_button_input.pressed(KeyCode::ControlLeft) {
        rise = -CAMERA_MOVEMENT_SPEED * time.delta_seconds();
    } else if keyboard_button_input.pressed(KeyCode::ShiftLeft) {
        rise = CAMERA_MOVEMENT_SPEED * time.delta_seconds();
    }

    if keyboard_button_input.pressed(KeyCode::KeyQ) {
        roll = -CAMERA_ROLL_SPEED * time.delta_seconds();
    } else if keyboard_button_input.pressed(KeyCode::KeyE) {
        roll = CAMERA_ROLL_SPEED * time.delta_seconds();
    }

    if keyboard_button_input.pressed(KeyCode::Home) {
        for transform in pod_transform_query.iter() {
            camera_transform.look_at(transform.translation, Vec3::Y);
        }    
    }

    let strafe_movement = camera_transform.left() * strafe;
    camera_transform.translation += strafe_movement;

    let thrust_movement = camera_transform.forward() * thrust;
    camera_transform.translation += thrust_movement;

    let rise_movement = camera_transform.up() * rise;
    camera_transform.translation += rise_movement;

    camera_transform.rotate_local_z(roll);
    debug!("stop");
}

fn state_controls(
    mut commands: Commands,
    scene_assets: Res<SceneAssets>,
    collider_assets: Res<ColliderAssets>,
    keyboard_button_input: Res<ButtonInput<KeyCode>>,
    mut pod_query: Query<(&mut Transform, &mut Velocity), With<CommandPod>>,
) {
    let span = span!(Level::DEBUG, "camera_controls()");
    let _enter = span.enter();
    debug!("start");
    if keyboard_button_input.pressed(KeyCode::KeyR) {
        for (mut pod_transform, mut pod_velocity) in pod_query.iter_mut() {
            pod_transform.translation = Vec3 { x: 0.0, y: 40.0, z: 0.0 };
            pod_transform.rotation = Quat::from_rotation_x(0.0);
            pod_velocity.linvel = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
            pod_velocity.angvel = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
        }
    }

    if keyboard_button_input.just_pressed(KeyCode::KeyI) {
        commands
        .spawn(RigidBody::Dynamic)
        .insert(SceneBundle {
            scene: scene_assets.command_pod_scene.clone(),
            ..default()
        })
        .insert(collider_assets.command_pod_collider.clone())
        .insert(Restitution::coefficient(0.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 40.0, 0.0)))
        .insert(Velocity {
            linvel: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
            angvel: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
        })
        .insert(CommandPod);
    }
    debug!("stop");
}

fn app_controls(
    keyboard_button_input: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
) {
    if keyboard_button_input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit);
    }
}