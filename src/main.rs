use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    sprite::MaterialMesh2dBundle,
    window::{Cursor, PrimaryWindow},
};
use bevy_mod_picking::prelude::*;
use bevy_prototype_lyon::prelude::*;
use cursor::{update_cursor, CursorWorldCoords};

mod cursor;

#[derive(Debug, Component)]
struct Node;

#[derive(Debug, Component)]
struct Edge(Entity, Entity);

#[derive(Debug, Component)]
struct TrackCursor(bool);

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((DefaultPlugins, ShapePlugin, DefaultPickingPlugins))
        .add_systems(Startup, (cursor::setup_camera, make_network))
        .add_systems(Update, (update_cursor, track_cursor, move_line).chain())
        .run();
}

fn make_network(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let start_pos = Vec3::new(-150., 0., 1.);
    let end_pos = Vec3::new(150., 0., 1.);

    let start = commands
        .spawn(make_node(start_pos, &mut meshes, &mut materials))
        .id();

    let end = commands
        .spawn(make_node(end_pos, &mut meshes, &mut materials))
        .id();

    let path = line_path(start_pos.xy(), end_pos.xy());
    commands.spawn((
        Edge(start, end),
        ShapeBundle { path, ..default() },
        Stroke::color(Color::RED),
    ));
}

fn make_node(
    position: Vec3,
    meshes: &mut ResMut<'_, Assets<Mesh>>,
    materials: &mut ResMut<'_, Assets<ColorMaterial>>,
) -> impl Bundle {
    (
        Node,
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(20.).into()).into(),
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            transform: Transform::from_translation(position),
            ..default()
        },
        On::<Pointer<Drag>>::target_component_mut::<Transform>(|drag, transform| {
            // drag and drop
            let Vec2 { x, y } = drag.delta;
            // drag events are y-inverted
            transform.translation += Vec3::new(x, -y, 0.);
        }),
        On::<Pointer<DragEnd>>::target_component_mut::<TrackCursor>(
            |drag_end, TrackCursor(track)| {
                // reset whether it's selected
                if drag_end.distance.length_squared() > 1000. {
                    *track = false;
                }
            },
        ),
        On::<Pointer<Click>>::target_component_mut::<TrackCursor>(|_click, TrackCursor(track)| {
            *track = !*track;
        }),
        // PickableBundle::default(),
        TrackCursor(false),
    )
}

fn track_cursor(
    pointer: Res<CursorWorldCoords>,
    mut entities: Query<(&mut Transform, &TrackCursor)>,
) {
    for (mut transform, TrackCursor(track)) in &mut entities {
        if *track {
            let Vec2 { x, y } = pointer.0;
            transform.translation.x = x;
            transform.translation.y = y;
        }
    }
}

fn move_line(nodes: Query<&Transform, With<Node>>, mut edges: Query<(&mut Path, &Edge)>) {
    for (mut path, Edge(start, end)) in &mut edges {
        let start = {
            let start = if let Ok(start) = nodes.get(*start) {
                start
            } else {
                continue;
            };
            start.translation.xy()
        };
        let end = {
            let end = if let Ok(end) = nodes.get(*end) {
                end
            } else {
                continue;
            };
            end.translation.xy()
        };
        *path = line_path(start, end)
    }
}

fn line_path(start: Vec2, end: Vec2) -> Path {
    let mut path_builder = PathBuilder::new();
    path_builder.move_to(start);
    path_builder.line_to(end);
    path_builder.build()
}
