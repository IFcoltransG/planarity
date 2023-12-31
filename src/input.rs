use crate::{
    cmp_vec,
    config::Cfg,
    fields::Field,
    generate::{bigger_graph, make_network, PreviousGraphs},
    Edge, LevelCleanup, Node, Velocity,
};
use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_prototype_lyon::prelude::*;
use leafwing_input_manager::prelude::*;

#[derive(Debug, Actionlike, Reflect, Clone, Event, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum Action {
    Reset,
    Bigger,
    MoveOutwards,
    Size(usize, usize),
}

pub(crate) fn keyboard_action_events(
    actions: Res<ActionState<Action>>,
    mut action_events: EventWriter<Action>,
) {
    for action in actions.get_just_pressed() {
        action_events.send(action)
    }
}

pub(crate) fn move_points_outwards(
    actions: Res<ActionState<Action>>,
    mut points: Query<(Entity, &mut Velocity), With<Node>>,
    field: Field<Entity, ()>,
    time: Res<Time>,
    cfg: Res<Cfg>,
) {
    // fn target(vec: Vec2, length: f32) -> Vec2 {
    //     with_length(vec, vec.length() - length)
    // }
    // fn invert(vec: Vec2) -> Vec2 {
    //     with_length(vec, 10. / vec.length().max(1.))
    // }
    fn invert(value: f32) -> f32 {
        10. / value.max(1.)
    }
    fn with_length(vec: Vec2, length: f32) -> Vec2 {
        vec.normalize_or_zero() * length
    }
    fn distance_to_circle(vec: Vec2, radius: f32) -> f32 {
        radius - vec.length()
    }
    if actions.pressed(Action::MoveOutwards) {
        let delta_time = time.delta_seconds().min(0.1);
        for (entity, mut velocity) in &mut points {
            let (transform, _) = field.nodes.get(entity).unwrap();
            let point = transform.translation.xy();

            let to_nearest_point = field.points_strength_except(entity, point, cfg.field_base);
            let to_nearest_line = field.lines_strength_except(entity, point, cfg.field_base);
            let to_centre = field.boundary_strength(point);
            // let target_centre = target(to_centre, cfg.target_centre_length);
            let direction = [
                with_length(
                    to_centre,
                    invert(distance_to_circle(to_centre, cfg.target_centre_length)),
                ),
                with_length(-to_nearest_point, invert(to_nearest_point.length() / 2.)),
                with_length(-to_nearest_line, invert(to_nearest_line.length())),
            ]
            .into_iter()
            .max_by(cmp_vec)
            .unwrap()
            .clamp_length_max(3.0);
            let new_velocity = delta_time * cfg.move_speed * direction;

            *velocity = Velocity(Some(new_velocity));
        }
    }
}

pub(crate) fn reset_network(
    mut commands: Commands,
    mut actions: EventReader<Action>,
    level: Query<Entity, With<LevelCleanup>>,
    cfg: Res<Cfg>,
    previous_graphs: ResMut<PreviousGraphs>,
) {
    for action in actions.read() {
        match action {
            Action::Reset => {
                for entity in &level {
                    commands.get_entity(entity).unwrap().despawn();
                }
                return make_network(commands, cfg, previous_graphs);
            }
            _ => {}
        };
    }
}

pub(crate) fn bigger_network(
    commands: Commands,
    mut actions: EventReader<Action>,
    previous_graphs: ResMut<PreviousGraphs>,
    edges: Query<Entity, With<Edge>>,
) {
    for action in actions.read() {
        match action {
            Action::Bigger => return bigger_graph(commands, previous_graphs, edges),
            _ => {}
        };
    }
}

pub(crate) fn network_size(
    mut actions: EventReader<Action>,
    mut cfg: ResMut<Cfg>,
) {
    for action in actions.read() {
        match action {
            Action::Size(graph_size, number_of_circles) => {
                cfg.num_circles = *number_of_circles;
                cfg.limit_nodes = *graph_size;
                return;
            }
            _ => {}
        };
    }
}

pub(crate) fn move_line(
    nodes: Query<&Transform, With<Node>>,
    mut edges: Query<(&mut Path, &Edge)>,
) {
    for (mut path, Edge(start, end)) in &mut edges {
        let start = {
            let Ok(start) = nodes.get(*start) else {
                continue;
            };
            start.translation.xy()
        };
        let end = {
            let Ok(end) = nodes.get(*end) else { continue };
            end.translation.xy()
        };
        *path = GeometryBuilder::build_as(&shapes::Line(start, end));
    }
}
