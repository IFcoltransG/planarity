use crate::{
    cmp_vec, config::Cfg, fields::Field, generate::make_network, Edge, LevelCleanup, Node, Velocity,
};
use ::bevy::{math::Vec3Swizzles, prelude::*};
use ::bevy_prototype_lyon::prelude::*;
use ::leafwing_input_manager::prelude::*;

#[derive(Debug, Actionlike, Reflect, Clone, Event, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum Action {
    Reset,
    MoveOutwards,
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
    fn target(vec: Vec2, length: f32) -> Vec2 {
        vec.normalize_or_zero() * (vec.length() - length)
    }
    if actions.pressed(Action::MoveOutwards) {
        let delta_time = time.delta_seconds();
        for (entity, mut velocity) in &mut points {
            let (transform, _) = field.nodes.get(entity).unwrap();
            let point = transform.translation.xy();

            let to_nearest_point = field.points_strength_except(entity, point);
            let to_nearest_line = field.lines_strength_except(entity, point);
            let to_centre = field.boundary_strength(point);
            let deflect = match &(to_nearest_line.length_squared() - 100.).max(0.) {
                x if ((0.)..0.1).contains(x) => 256.,
                x if ((0.)..0.5).contains(x) => 64.,
                x if ((0.)..1.).contains(x) => 16.,
                x if ((0.)..10.).contains(x) => 8.,
                x if ((0.)..100.).contains(x) => 4.,
                x if ((0.)..1000.).contains(x) => 2.,
                _ => 1.,
            };
            let direction = [
                target(to_centre, cfg.target_centre_length),
                target(-to_nearest_point, -cfg.target_point_length),
                -to_nearest_line * deflect,
            ]
            .into_iter()
            .max_by(cmp_vec)
            .unwrap()
            .clamp_length_max(1.);
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
) {
    for action in actions.read() {
        match action {
            Action::Reset => {
                for entity in &level {
                    commands.get_entity(entity).unwrap().despawn();
                }
                return make_network(commands, cfg);
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
