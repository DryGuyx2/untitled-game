use avian2d::prelude::*;
use bevy::{
    color::palettes::css::GRAY,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureFormat::Bgra8UnormSrgb, TextureUsages,
        },
        view::RenderLayers,
    },
    window::{PrimaryWindow, WindowResized},
};

const RES_HEIGHT: u32 = 80;
const RES_WIDTH: u32 = 128;

const PIXEL_PERFECT_LAYER: RenderLayers = RenderLayers::layer(0);
const HIGH_RES_LAYER: RenderLayers = RenderLayers::layer(1);

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (RES_WIDTH as f32 * 10., RES_HEIGHT as f32 * 10.).into(),
                    title: "Untitled Game".into(),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .set(ImagePlugin::default_nearest()),
        PhysicsPlugins::default(),
        PhysicsDebugPlugin::default(),
    ));
    app.add_systems(Startup, setup);
    app.add_systems(Update, fit_canvas);
    app.add_systems(
        Update,
        (
            move_player,
            update_mouse_world_pos,
            rotate_to_mouse,
            spawn_flares,
        ),
    );
    app.insert_resource(MouseWorldPos(Vec2::new(0., 0.)));
    app.run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    let canvas_size = Extent3d {
        width: RES_WIDTH,
        height: RES_HEIGHT,
        depth_or_array_layers: 1,
    };

    let mut canvas_texture = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            size: canvas_size,
            dimension: bevy::render::render_resource::TextureDimension::D2,
            format: Bgra8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..Default::default()
    };

    canvas_texture.resize(canvas_size);
    let image_handle = images.add(canvas_texture);

    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            target: RenderTarget::Image(image_handle.clone().into()),
            clear_color: ClearColorConfig::Custom(GRAY.into()),
            ..Default::default()
        },
        PIXEL_PERFECT_LAYER,
    ));

    commands.spawn((Sprite::from_image(image_handle), Canvas, HIGH_RES_LAYER));
    commands.spawn((Camera2d, Msaa::Off, HIGH_RES_LAYER, MainCamera));

    commands.spawn((
        Transform::from_xyz(0., 0., 0.).with_scale(Vec3::splat(1.)),
        Sprite::from_image(asset_server.load("player.png")),
        Player,
        RotateToMouse,
        RigidBody::Dynamic,
        Collider::circle(9.),
        DebugRender::default().with_collider_color(Color::srgb(1.0, 0.0, 0.0)),
        PIXEL_PERFECT_LAYER,
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
        MaxLinearSpeed(400.),
    ));

    commands.spawn((
        Transform::from_xyz(30., 0., 0.).with_scale(Vec3::splat(1.)),
        Sprite::from_image(asset_server.load("player.png")),
        RigidBody::Kinematic,
        Collider::circle(9.),
        DebugRender::default().with_collider_color(Color::srgb(1.0, 1.0, 0.0)),
        PIXEL_PERFECT_LAYER,
    ));
}

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Canvas;

fn fit_canvas(
    mut resize_events: EventReader<WindowResized>,
    mut canvas_transform: Single<&mut Transform, With<Canvas>>,
) {
    for event in resize_events.read() {
        let scale_x = event.width / RES_WIDTH as f32;
        let scale_y = event.height / RES_HEIGHT as f32;
        let scale = scale_x.min(scale_y).floor();

        canvas_transform.scale = Vec3::splat(scale);
    }
}

#[derive(Resource, Debug)]
struct MouseWorldPos(Vec2);

fn update_mouse_world_pos(
    mut mouse_world_pos: ResMut<MouseWorldPos>,
    camera_q: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let (camera, camera_pos) = *camera_q;

    let cursor_pos = match window.cursor_position() {
        Some(pos) => pos,
        None => return,
    };

    let cursor_world_pos = match camera.viewport_to_world_2d(camera_pos, cursor_pos) {
        Ok(pos) => pos,
        Err(_) => return,
    };

    let cursor_ndc_world_pos = match camera.world_to_ndc(camera_pos, cursor_world_pos.extend(0.)) {
        Some(pos) => pos,
        None => return,
    };

    let scaled_ndc_world_pos = Vec2::new(
        cursor_ndc_world_pos.x * RES_WIDTH as f32,
        cursor_ndc_world_pos.y * RES_HEIGHT as f32,
    );

    mouse_world_pos.0 = scaled_ndc_world_pos;
}

#[derive(Component)]
struct RotateToMouse;

#[derive(Component)]
struct Player;

fn move_player(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    player_velocity: Single<&mut LinearVelocity, With<Player>>,
) {
    let mut direction = Vec2::ZERO;
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.;
    };
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction.x += 1.;
    };
    if keyboard_input.pressed(KeyCode::KeyW) {
        direction.y += 1.;
    };
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.;
    };

    let speed = 100.;

    direction = direction.normalize_or_zero() * speed;

    let mut velocity = player_velocity.into_inner();
    velocity.0 = direction;
}

fn rotate_to_mouse(
    mouse_world_pos: Res<MouseWorldPos>,
    mut transform_q: Query<&mut Transform, With<RotateToMouse>>,
) {
    for mut transform in transform_q.iter_mut() {
        let direction = mouse_world_pos.0 - transform.translation.truncate();
        let angle = direction.y.atan2(direction.x);
        transform.rotation = Quat::from_rotation_z(angle);
    }
}

#[derive(Component)]
struct Flare;

fn spawn_flares(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    player_transform: Single<&Transform, With<Player>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyF) {
        commands.spawn((
            Flare,
            Transform::from_xyz(30., 0., 0.).with_scale(Vec3::splat(1.)),
            Sprite::from_image(asset_server.load("flare.png")),
            //RigidBody::Dynamic,
            //Collider::circle(9.),
            //DebugRender::default().with_collider_color(Color::srgb(1.0, 1.0, 0.0)),
            PIXEL_PERFECT_LAYER,
        ));
        info!("Flare spawned");
    }
}
