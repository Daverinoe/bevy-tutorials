use bevy::input::common_conditions::input_just_released;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowFocused};
use std::f32::consts::PI;

const PLAYER_SPEED: f32 = 50.;
const MOUSE_SENSITIVITY: f32 = 0.01;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, (spawn_camera, spawn_map));
    app.add_systems(
        Update,
        (
            player_look,
            player_move.after(player_look),
            focus_events,
            toggle_grab.run_if(input_just_released(KeyCode::Escape)),
            shoot_ball.before(spawn_ball).before(focus_events),
            spawn_ball,
        ),
    );
    app.add_observer(apply_grab);
    app.add_event::<BallSpawn>();
    app.run();
}

#[derive(Component)]
struct Player;

#[derive(Event, Deref)]
struct GrabEvent(bool);

#[derive(Event)]
struct BallSpawn {
    position: Vec3,
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Player));
}

fn spawn_map(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(DirectionalLight::default());
    let ball_mesh = mesh_assets.add(Sphere::new(1.));

    for h in 0..16 {
        let color = Color::hsl((h as f32 / 16.) * 360., 1., 0.5);
        let ball_material = material_assets.add(StandardMaterial {
            base_color: color,
            ..Default::default()
        });
        commands.spawn((
            Transform::from_translation(Vec3::new((-8. + h as f32) * 2., 0., -50.)),
            Mesh3d(ball_mesh.clone()),
            MeshMaterial3d(ball_material),
        ));
    }
}

fn player_look(
    mut player: Single<&mut Transform, With<Player>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    if !window.focused {
        return;
    }
    let sensitivity = 100. / window.width().min(window.height()) * MOUSE_SENSITIVITY;

    use EulerRot::YXZ;
    let (mut yaw, mut pitch, _) = player.rotation.to_euler(YXZ);

    pitch -= mouse_motion.delta.y * sensitivity;
    pitch = pitch.clamp(-PI / 2., PI / 2.);
    yaw -= mouse_motion.delta.x * sensitivity;

    player.rotation = Quat::from_euler(YXZ, yaw, pitch, 0.);
}

fn player_move(
    mut player: Single<&mut Transform, With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let mut direction = Vec3::ZERO;

    if keys.pressed(KeyCode::KeyA) {
        direction.x -= 1.;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction.x += 1.;
    }
    if keys.pressed(KeyCode::KeyW) {
        direction.z += 1.;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction.z -= 1.;
    }

    let forward = player.forward().as_vec3() * direction.z;
    let right = player.right().as_vec3() * direction.x;

    let mut to_move = forward + right;
    to_move.y = 0.;
    to_move = to_move.normalize_or_zero();

    player.translation += to_move * time.delta_secs() * PLAYER_SPEED;
}

fn apply_grab(grab: Trigger<GrabEvent>, mut window: Single<&mut Window, With<PrimaryWindow>>) {
    use bevy::window::CursorGrabMode;
    if **grab {
        window.cursor_options.visible = false;
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
    } else {
        window.cursor_options.visible = true;
        window.cursor_options.grab_mode = CursorGrabMode::None;
    }
}

fn focus_events(mut events: EventReader<WindowFocused>, mut commands: Commands) {
    if let Some(event) = events.read().last() {
        commands.trigger(GrabEvent(event.focused));
    }
}

fn toggle_grab(mut window: Single<&mut Window, With<PrimaryWindow>>, mut commands: Commands) {
    window.focused = !window.focused;
    commands.trigger(GrabEvent(window.focused));
}

fn spawn_ball(
    mut events: EventReader<BallSpawn>,
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    for spawn in events.read() {
        commands.spawn((
            Transform::from_translation(spawn.position),
            Mesh3d(mesh_assets.add(Sphere::new(1.))),
            MeshMaterial3d(material_assets.add(StandardMaterial::default())),
        ));
    }
}

fn shoot_ball(
    inputs: Res<ButtonInput<MouseButton>>,
    player: Single<&Transform, With<Player>>,
    mut spawner: EventWriter<BallSpawn>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    if window.cursor_options.visible {
        return;
    }
    if !inputs.just_pressed(MouseButton::Left) {
        return;
    }

    spawner.write(BallSpawn {
        position: player.translation,
    });
}
