#![feature(array_windows)]

use std::cmp::Ordering;

use crate::{config::Cfg, input::Action};
use ::bevy::{
    math::{DMat2, Vec3Swizzles},
    prelude::*,
};
use ::bevy_egui::EguiPlugin;
use ::bevy_mod_picking::prelude::*;
use ::bevy_prototype_lyon::prelude::*;
use ::geo::{prelude::*, Coord};
use ::iyes_progress::prelude::*;
use ::leafwing_input_manager::prelude::*;
use cursor::CursorWorldCoords;
use story::{JsonStoryAsset, JsonStoryLoader};

mod config;
mod cursor;
mod fields;
mod generate;
mod input;
mod story;

/// Tags entities that will be deleted when resetting the level
#[derive(Debug, Component)]
struct LevelCleanup;

/// Graph nodes
#[derive(Debug, Component)]
struct Node;

#[derive(Debug, Component, PartialEq, Eq, Hash, Clone, Copy)]
struct Edge(Entity, Entity);

#[derive(Component)]
struct Velocity(Option<Vec2>);

#[derive(Debug)]
struct Endpoint(Entity, Vec2);
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Loading,
    Running,
}

fn main() {
    App::new()
        // .insert_resource(Msaa::Sample4)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Cfg::default())
        .insert_resource(InputMap::new([
            (KeyCode::Space, Action::Reset),
            (KeyCode::F, Action::MoveOutwards),
            ]))
            .init_resource::<ActionState<Action>>()
            .add_event::<Action>()
            .add_plugins((
                DefaultPlugins,
                EguiPlugin,
                ShapePlugin,
                DefaultPickingPlugins,
                InputManagerPlugin::<Action>::default(),
                ProgressPlugin::new(AppState::Loading)
                    .continue_to(AppState::Running)
                    .track_assets()
            ))
        .init_asset_loader::<JsonStoryLoader>()
        .init_asset::<JsonStoryAsset>()
        .add_state::<AppState>()
        .add_systems(Startup, (cursor::setup_camera, generate::make_network, story::setup_story_asset))
        .add_systems(OnExit(AppState::Loading), story::setup_story)
        .add_systems(
            Update,
            (
                (
                    (cursor::update_cursor, input::move_points_outwards).chain(),
                    cursor::track_cursor,
                    apply_velocity,
                    input::move_line,
                )
                    .chain(),
                highlight_edges,
                input::keyboard_action_events,
                input::reset_network,
                fields::show_strength,
                fields::debug_field,
                config::settings,
                story::show_story,
            ).run_if(in_state(AppState::Running)),
        )
        .run();
}

fn highlight_edges(
    mut edges: Query<(&mut Stroke, &Edge)>,
    mut node_fills: Query<&mut Fill, With<Node>>,
    translations: Query<&Transform, With<Node>>,
) {
    for (mut stroke, _) in &mut edges {
        *stroke = Stroke::color(Color::DARK_GREEN);
    }

    for mut fill in &mut node_fills {
        *fill = Fill::color(Color::MIDNIGHT_BLUE);
    }

    let mut edge_combos = edges.iter_combinations_mut();
    while let Some([(mut x_stroke, x_edge), (mut y_stroke, y_edge)]) = edge_combos.fetch_next() {
        // skip if edges are adjacent; compare all endpoints
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
            for entity in [x_edge.0, x_edge.1, y_edge.0, y_edge.1] {
                if let Ok(mut fill) = node_fills.get_mut(entity) {
                    *fill = Fill::color(Color::PURPLE)
                }
            }
        }
    }
}

fn apply_velocity(
    mut points: Query<(&mut Transform, &mut Velocity), (With<Node>, Changed<Velocity>)>,
    cursor: Res<CursorWorldCoords>,
) {
    for (mut point, mut velocity) in &mut points {
        if let Some(velocity) = velocity.0.take() {
            let distance_to_cursor = (cursor.0 - point.translation.xy()).length_squared();
            let speed = (distance_to_cursor / 10000.).clamp(0., 1.);
            point.translation += velocity.extend(0.) * speed
        }
    }
}

fn intersects([p1, p2]: [Vec2; 2], [q1, q2]: [Vec2; 2]) -> bool {
    fn coord(v: Vec2) -> Coord<f32> {
        Into::<[f32; 2]>::into(v).into()
    }
    let p_line = geo::Line::new(coord(p1), coord(p2));
    let q_line = geo::Line::new(coord(q1), coord(q2));

    p_line.intersects(&q_line)
}

fn intersection_scalars(a_vec: Vec2, b_vec: Vec2, starting_difference: Vec2) -> Option<Vec2> {
    let matrix = DMat2::from_cols(a_vec.as_dvec2(), b_vec.as_dvec2());
    let determinant = matrix.determinant();
    if determinant == 0. {
        return None;
    }
    let inverted = matrix.inverse();
    // vec of [a_coefficient, b_coefficient] to roughly add to difference
    Some((inverted * starting_difference.as_dvec2()).as_vec2())
}

// #[test]
// fn test() {
//     use rand::Rng;
//     for _ in 0..10 {
//         let a: Vec2 = thread_rng().gen::<[f32; 2]>().into();
//         let b: Vec2 = thread_rng().gen::<[f32; 2]>().into();
//         let diff: Vec2 = thread_rng().gen::<[f32; 2]>().into();
//         let result = intersection_scalars(a, b, diff).unwrap();
//         assert_eq!(a * result.x + b * result.y, diff);
//     }
//     let solution: Vec2 =
//         intersection_scalars(Vec2::new(1., 1.), Vec2::new(-2., 0.),
// Vec2::new(-1., 1.)).unwrap();     assert_eq!(solution, Vec2::new(1., 1.))
// }

fn cmp_vec(x: &Vec2, y: &Vec2) -> Ordering {
    x.length_squared().total_cmp(&y.length_squared())
}
