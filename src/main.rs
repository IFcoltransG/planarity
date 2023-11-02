use std::f32::consts::TAU;

use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    sprite::{Material2d, MaterialMesh2dBundle, Mesh2dHandle},
};
use bevy_mod_picking::prelude::*;
use bevy_prototype_lyon::{prelude::*, shapes::Circle};
use cursor::{update_cursor, CursorWorldCoords};
use geo::{prelude::*, Coord};
use itertools::Itertools;
use petgraph::prelude::*;
use rand::{seq::SliceRandom, thread_rng, Rng};

mod cursor;

#[derive(Debug, Component)]
struct Node;

#[derive(Debug, Component, PartialEq, Eq, Hash, Clone, Copy)]
struct Edge(Entity, Entity);

#[derive(Debug, Component)]
struct TrackCursor(bool);

#[derive(Debug)]
struct Endpoint(Entity, Vec2);

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((DefaultPlugins, ShapePlugin, DefaultPickingPlugins))
        .add_systems(Startup, (cursor::setup_camera, make_network))
        .add_systems(
            Update,
            (
                (update_cursor, track_cursor, move_line).chain(),
                highlight_edges,
            ),
        )
        .run();
}

fn make_network(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let num_nodes = 5;
    let num_edges = 10;

    let mut rng = thread_rng();

    let positions = (0..num_nodes).map(|_| Vec2::from_angle(rng.gen_range(0.0..TAU)) * 100.);
    let endpoints: Vec<_> = positions
        .map(|pos| make_endpoint(&mut commands, make_node(pos), pos))
        .collect();

    let mut graph = DiGraphMap::with_capacity(num_nodes, num_edges);
    for endpoint in &endpoints {
        graph.add_node(endpoint.0);
    }

    for _ in 0..num_edges {
        if let Some((start, end)) = endpoints.choose_multiple(&mut rng, 2).collect_tuple() {
            if graph.add_edge(start.0, end.0, ()).is_none() {
                make_edge(&mut commands, &start, &end);
            }
        }
    }
}

fn make_endpoint(commands: &mut Commands, bundle: impl Bundle, position: Vec2) -> Endpoint {
    Endpoint(commands.spawn(bundle).id(), position)
}

fn make_edge(
    commands: &mut Commands,
    Endpoint(start, start_pos): &Endpoint,
    Endpoint(end, end_pos): &Endpoint,
) -> Edge {
    let path = GeometryBuilder::build_as(&shapes::Line(*start_pos, *end_pos));
    let edge = Edge(*start, *end);
    commands.spawn((
        edge,
        ShapeBundle { path, ..default() },
        Stroke::color(Color::RED),
    ));
    edge
}

fn highlight_edges(
    mut edges: Query<(&mut Stroke, &Edge)>,
    mut nodes: Query<&mut Fill, With<Node>>,
    translations: Query<&Transform, With<Node>>,
) {
    for (mut stroke, _) in &mut edges {
        *stroke = Stroke::color(Color::DARK_GREEN)
    }

    let mut edge_combos = edges.iter_combinations_mut();
    while let Some([(mut x_stroke, x_edge), (mut y_stroke, y_edge)]) = edge_combos.fetch_next() {
        // skip if edges are adjacent
        if x_edge.0 == y_edge.0
            || x_edge.0 == y_edge.1
            || x_edge.1 == y_edge.0
            || x_edge.1 == y_edge.1
        {
            continue;
        }

        let [Ok(point_x_0), Ok(point_x_1), Ok(point_y_0), Ok(point_y_1)] = [
            translations.get(y_edge.0),
            translations.get(y_edge.1),
            translations.get(x_edge.0),
            translations.get(x_edge.1),
        ] else {
            continue;
        };

        if intersects(
            [point_x_0.translation.xy(), point_x_1.translation.xy()],
            [point_y_0.translation.xy(), point_y_1.translation.xy()],
        ) {
            *x_stroke = Stroke::new(Color::RED, 3.0);
            *y_stroke = Stroke::new(Color::RED, 3.0);
        }
    }
}

fn intersects([p1, p2]: [Vec2; 2], [q1, q2]: [Vec2; 2]) -> bool {
    fn coord(v: Vec2) -> Coord<f32> {
        Into::<[f32; 2]>::into(v).into()
    }
    let p = geo::Line::new(coord(p1), coord(p2));
    let q = geo::Line::new(coord(q1), coord(q2));

    p.intersects(&q)
}

fn make_node(position: Vec2) -> impl Bundle {
    let path = ShapePath::build_as(&Circle {
        radius: 10.0,
        center: Vec2::ZERO,
    });
    (
        Node,
        ShapeBundle {
            path,
            transform: Transform::from_translation(position.extend(1.)),
            ..default()
        },
        Fill::color(Color::PURPLE),
        On::<Pointer<DragStart>>::target_component_mut::<TrackCursor>(
            |_drag_start, TrackCursor(track)| {
                // toggle whether it's selected (will be undone by the click event at end of
                // drag)
                *track = !*track;
            },
        ),
        On::<Pointer<DragEnd>>::target_component_mut::<TrackCursor>(
            |drag_end, TrackCursor(track)| {
                // only register as a drag if distance high enough
                if drag_end.distance.length_squared() < 1000. {
                    // undo the drag start, treat as not dragged at all
                    // but let the click event accompanying the drag still fire
                    *track = !*track;
                }
            },
        ),
        On::<Pointer<Click>>::target_component_mut::<TrackCursor>(|_click, TrackCursor(track)| {
            // toggle whether it's selected
            *track = !*track;
        }),
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
        *path = GeometryBuilder::build_as(&shapes::Line(start, end));
    }
}
