use bevy::input::common_conditions::input_just_released;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowFocused};
use rand::SeedableRng;
use std::f32::consts::PI;

const PLAYER_SPEED: f32 = 50.;
const MOUSE_SENSITIVITY: f32 = 0.01;
const SHOT_VELOCITY: f32 = 10.;
const GRAVITY: Vec3 = Vec3::new(0., -9.8, 0.);
const POWER_MIN: f32 = 1.;
const POWER_MAX: f32 = 6.;

const NOT_CHARGING: Color = Color::linear_rgb(0.2, 0.2, 0.2);
const MIN_FILL: f32 = 29.75 / POWER_MAX;
const EMPTY_SPACE: f32 = 29.75 - MIN_FILL;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, (spawn_camera, spawn_map));
    app.insert_resource(Time::<Fixed>::from_hz(60.));
    app.add_systems(
        FixedUpdate,
        (
            apply_velocity,
            apply_gravity.before(apply_velocity),
            bounce.after(apply_velocity),
        ),
    );
    app.add_systems(
        Update,
        (
            player_look,
            player_move.after(player_look),
            focus_events,
            toggle_grab.run_if(input_just_released(KeyCode::Escape)),
            shoot_ball.before(spawn_ball).before(focus_events),
            spawn_ball,
            update_power_bar,
        ),
    );
    app.add_observer(apply_grab);
    app.add_event::<BallSpawn>();
    app.init_resource::<BallData>();
    app.insert_resource(Power {
        charging: false,
        current: 0.,
    });
    app.run();
}

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec3);

#[derive(Component)]
struct Player;

#[derive(Event, Deref)]
struct GrabEvent(bool);

#[derive(Event)]
struct BallSpawn {
    position: Vec3,
    velocity: Vec3,
    power: f32,
}

#[derive(Resource)]
struct BallData {
    mesh: Handle<Mesh>,
    materials: Vec<Handle<StandardMaterial>>,
    rng: std::sync::Mutex<rand::rngs::StdRng>,
}

impl BallData {
    fn mesh(&self) -> Handle<Mesh> {
        self.mesh.clone()
    }
    fn material(&self) -> Handle<StandardMaterial> {
        use rand::seq::IndexedRandom;
        let mut rng = self.rng.lock().unwrap();
        self.materials.choose(&mut rng).unwrap().clone()
    }
}

impl FromWorld for BallData {
    fn from_world(world: &mut World) -> Self {
        let mesh = world.resource_mut::<Assets<Mesh>>().add(Sphere::new(1.));
        let mut materials = Vec::new();
        let mut mat_assets = world.resource_mut::<Assets<StandardMaterial>>();
        for i in 0..36 {
            let color = Color::hsl((i * 10) as f32, 1., 0.5);
            materials.push(mat_assets.add(StandardMaterial {
                base_color: color,
                ..Default::default()
            }));
        }
        let seed = *b"DaverinoeIsC00lDaverinoeIsC00l22";
        BallData {
            mesh,
            materials,
            rng: std::sync::Mutex::new(rand::rngs::StdRng::from_seed(seed)),
        }
    }
}

fn apply_gravity(mut objects: Query<&mut Velocity>, time: Res<Time>) {
    let g = GRAVITY * time.delta_secs();
    for mut v in &mut objects {
        **v += g;
    }
}

fn bounce(mut balls: Query<(&Transform, &mut Velocity)>) {
    for (transform, mut velocity) in &mut balls {
        if transform.translation.y < 0. && velocity.y < 0. {
            velocity.y *= -1.;
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Player));
}

fn spawn_map(mut commands: Commands, ball_data: Res<BallData>) {
    commands.spawn(DirectionalLight::default());

    for h in 0..ball_data.materials.len() {
        let ball_material = ball_data.materials[h].clone();
        commands.spawn((
            Transform::from_translation(Vec3::new((-8. + h as f32) * 2., 0., -50.)),
            Mesh3d(ball_data.mesh()),
            MeshMaterial3d(ball_material),
        ));
    }
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::VMax(30.),
                height: Val::VMax(5.),
                bottom: Val::Px(20.),
                left: Val::Px(20.),
                ..Default::default()
            },
            BackgroundColor(Color::linear_rgb(0.5, 0.5, 0.5)),
            BorderRadius::all(Val::VMax(5.)),
        ))
        .with_child((
            Node {
                position_type: PositionType::Absolute,
                min_width: Val::VMax(MIN_FILL),
                height: Val::Percent(95.),
                margin: UiRect::all(Val::VMax(0.125)),
                ..Default::default()
            },
            BackgroundColor(NOT_CHARGING),
            BorderRadius::all(Val::VMax(5.)),
            PowerBar {
                min: POWER_MIN,
                max: POWER_MAX,
            },
        ));
}

fn update_power_bar(
    mut bars: Query<(&mut Node, &PowerBar, &mut BackgroundColor)>,
    power: Res<Power>,
) {
    for (mut bar, config, mut bg) in &mut bars {
        if !power.charging {
            bg.0 = NOT_CHARGING;
            bar.width = Val::VMax(MIN_FILL);
        } else {
            let percent = (power.current - config.min) / (config.max - config.min);
            bg.0 = Color::linear_rgb(1. - percent, percent, 0.);
            bar.width = Val::VMax(MIN_FILL + percent * EMPTY_SPACE);
        }
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
    ball_data: Res<BallData>,
) {
    for spawn in events.read() {
        commands.spawn((
            Transform::from_translation(spawn.position),
            Mesh3d(ball_data.mesh()),
            MeshMaterial3d(ball_data.material()),
            Velocity(spawn.velocity * spawn.power * SHOT_VELOCITY),
        ));
    }
}

#[derive(Resource)]
struct Power {
    charging: bool,
    current: f32,
}

fn shoot_ball(
    inputs: Res<ButtonInput<MouseButton>>,
    player: Single<&Transform, With<Player>>,
    mut spawner: EventWriter<BallSpawn>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut power: ResMut<Power>,
    time: Res<Time>,
) {
    if window.cursor_options.visible {
        return;
    }

    if power.charging {
        if inputs.just_released(MouseButton::Left) {
            spawner.write(BallSpawn {
                position: player.translation,
                velocity: player.forward().as_vec3() * SHOT_VELOCITY,
                power: power.current,
            });
            power.charging = false;
            power.current = 1.;
        }
        if inputs.pressed(MouseButton::Left) {
            power.current += time.delta_secs();
            power.current = power.current.clamp(POWER_MIN, POWER_MAX);
        }
    }
    if inputs.just_pressed(MouseButton::Left) {
        power.charging = true;
    }
}

fn apply_velocity(mut objects: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut objects {
        transform.translation += velocity.0 * time.delta_secs();
    }
}

#[derive(Component)]
struct PowerBar {
    min: f32,
    max: f32,
}
