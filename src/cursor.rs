use bevy::{prelude::*, window::PrimaryWindow};
/// We will store the world position of the mouse cursor here.
#[derive(Resource, Default)]

pub(super) struct CursorWorldCoords(pub Vec2);

/// Used to help identify our main camera
#[derive(Component)]
pub(super) struct MainCamera;


pub(super) fn setup_camera(mut commands: Commands) {
    commands.init_resource::<CursorWorldCoords>();
    // Make sure to add the marker component when you set up your camera
    commands.spawn((
        Camera2dBundle::default(),
        MainCamera,
    ));
}

pub(super) fn update_cursor(
    mut cursor_coords: ResMut<CursorWorldCoords>,
    // query to get the window (so we can read the current cursor position)
    q_window: Query<&Window, With<PrimaryWindow>>,
    // query to get camera transform
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so Query::single() is OK
    let (camera, camera_transform) = q_camera.single();

    // There is only one primary window, so we can similarly get it from the query:
    let window = q_window.single();

    // check if the cursor is inside the window and get its position
    // then, ask bevy to convert into world coordinates, and truncate to discard Z
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        cursor_coords.0 = world_position;
        // eprintln!("World coords: {}/{}", world_position.x, world_position.y);
    }
}
