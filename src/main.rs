#![feature(array_windows)]

use std::cmp::Ordering;

use crate::{
    config::Cfg,
    cursor::CursorWorldCoords,
    generate::PreviousGraphs,
    input::Action,
    story::{story_assets, story_assets::InkAssetPlugin, Tag},
};
use bevy::{
    math::{DMat2, Vec3Swizzles},
    prelude::*,
};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::{
    inspector_options::ReflectInspectorOptions,
    quick::{ResourceInspectorPlugin, WorldInspectorPlugin},
    InspectorOptions,
};
use bevy_mod_picking::prelude::*;
use bevy_prototype_lyon::prelude::*;
use geo::{prelude::*, Coord};
use iyes_progress::prelude::*;
use leafwing_input_manager::prelude::*;

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

#[derive(Debug, Component, PartialEq)]
enum LineIntersects {
    Unsolved,
    Solved,
    Intersecting,
}

#[derive(Resource, Default, Reflect, Clone, Debug, InspectorOptions)]
#[reflect(Resource, InspectorOptions, Default)]
struct IntersectionsCount(u32);

#[derive(Debug, Component, PartialEq, Eq, Hash, Clone, Copy)]
struct Edge(Entity, Entity);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Velocity(Option<Vec2>);

#[derive(Debug, Clone)]
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
            (KeyCode::B, Action::Bigger),
            (KeyCode::F, Action::MoveOutwards),
        ]))
        .insert_resource(PreviousGraphs::<Endpoint>(Vec::new()))
        .init_resource::<ActionState<Action>>()
        .init_resource::<IntersectionsCount>()
        .add_event::<Action>()
        .add_event::<Tag>()
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                mode: AssetMode::Processed,
                ..default()
            }),
            EguiPlugin,
            ShapePlugin,
            DefaultPickingPlugins,
            InputManagerPlugin::<Action>::default(),
            ProgressPlugin::new(AppState::Loading)
                .continue_to(AppState::Running)
                .track_assets(),
            InkAssetPlugin,
            WorldInspectorPlugin::new(),
            ResourceInspectorPlugin::<Cfg>::default(),
            ResourceInspectorPlugin::<IntersectionsCount>::default(),
        ))
        .add_systems(Update, story_assets::setup_story_asset)
        .add_systems(Startup, (cursor::setup_camera, generate::make_network))
        .add_systems(Update, story::setup_story.run_if(story::story_needs_reload))
        // .add_systems(OnEnter(AppState::Running), story::setup_story)
        .add_state::<AppState>()
        // .add_systems(Update, story::reload_story)
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
                (input::network_size, input::reset_network).chain(),
                input::bigger_network,
                fields::show_strength,
                fields::debug_field,
                (story::show_story, story::tag_actions).chain(),
                story::log_tags,
                story::update_intersections,
            )
                .run_if(in_state(AppState::Running)),
        )
        .run();
}

fn highlight_edges(
    mut edges: Query<(&mut Stroke, &Edge)>,
    mut node_fills: Query<(&mut Fill, &mut LineIntersects), With<Node>>,
    mut intersections_count: ResMut<IntersectionsCount>,
    translations: Query<&Transform, With<Node>>,
) {
    for (mut stroke, _) in &mut edges {
        *stroke = Stroke::color(Color::DARK_GREEN);
    }

    for (mut fill, mut intersects) in &mut node_fills {
        let color = match *intersects {
            LineIntersects::Solved => Color::MIDNIGHT_BLUE,
            _ => Color::WHITE,
        };
        if *intersects == LineIntersects::Intersecting {
            *intersects = LineIntersects::Solved
        }
        *fill = Fill::color(color);
    }

    let mut edge_combos = edges.iter_combinations_mut();
    intersections_count.0 = 0;
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
                if let Ok((mut fill, mut intersects)) = node_fills.get_mut(entity) {
                    if *intersects != LineIntersects::Unsolved {
                        *fill = Fill::color(Color::PURPLE);
                        *intersects = LineIntersects::Intersecting
                    }
                }
            }
            intersections_count.0 += 1;
        }
    }
    for (fill, mut intersects) in &mut node_fills {
        if fill.color == Color::WHITE {
            *intersects = LineIntersects::Solved;
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

fn cmp_vec(x: &Vec2, y: &Vec2) -> Ordering {
    x.length_squared().total_cmp(&y.length_squared())
}
