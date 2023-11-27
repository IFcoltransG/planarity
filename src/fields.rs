use std::cmp::min_by;

use bevy::{
    ecs::{query::WorldQuery, system::SystemParam},
    math::Vec3Swizzles,
    prelude::*,
};
use geo::{Closest, ClosestPoint, Line, Point};

use crate::{cmp_vec, config::Cfg, cursor::CursorWorldCoords, Edge, Node};

#[derive(SystemParam)]
pub(crate) struct Field<'w, 's, NodeData, EdgeData>
where
    NodeData: 'static + WorldQuery,
    EdgeData: 'static + WorldQuery,
{
    pub nodes: Query<'w, 's, (&'static Transform, NodeData), With<Node>>,
    pub edges: Query<'w, 's, (&'static Edge, EdgeData)>,
}

impl<NodeData, EdgeData> Field<'_, '_, NodeData, EdgeData>
where
    NodeData: WorldQuery,
    EdgeData: WorldQuery,
{
    pub(crate) fn points_strength<'a>(
        &self,
        nodes: impl IntoIterator<Item = &'a Transform>,
        point: Vec2,
    ) -> Vec2 {
        let mut result = Vec2::INFINITY;
        for position in nodes {
            let to_nearest = position.translation.xy() - point;
            result = min_by(result, to_nearest, cmp_vec);
        }
        result
    }

    pub(crate) fn lines_strength<'a>(
        &self,
        edges: impl IntoIterator<Item = &'a Edge>,
        point: Vec2,
    ) -> Vec2 {
        fn to_point(v: Vec2) -> Point<f32> {
            Into::<[f32; 2]>::into(v).into()
        }
        fn to_vec(v: Point<f32>) -> Vec2 {
            Into::<[f32; 2]>::into(v).into()
        }
        let geo_point = to_point(point);
        let mut result = Vec2::INFINITY;
        for Edge(start, end) in edges {
            let (start, end) = (
                to_point(self.nodes.get(*start).unwrap().0.translation.xy()),
                to_point(self.nodes.get(*end).unwrap().0.translation.xy()),
            );
            let closest = match <Line<f32>>::new(start, end).closest_point(&geo_point) {
                Closest::Intersection(closest) | Closest::SinglePoint(closest) => closest,
                Closest::Indeterminate => continue,
            };
            let vec = to_vec(closest) - point;

            result = min_by(result, vec, cmp_vec);
        }
        result
    }

    pub(crate) fn boundary_strength(&self, point: Vec2) -> Vec2 {
        -point
    }
}

impl<EdgeData> Field<'_, '_, Entity, EdgeData>
where
    EdgeData: WorldQuery,
{
    pub(crate) fn points_strength_except(&self, entity: Entity, point: Vec2) -> Vec2 {
        self.points_strength(
            self.nodes
                .iter()
                .filter(|(_, other_entity)| *other_entity != entity)
                .map(|(edge, _)| edge),
            point,
        )
    }

    pub(crate) fn lines_strength_except(&self, entity: Entity, point: Vec2) -> Vec2 {
        self.lines_strength(
            self.edges
                .iter()
                .filter(|(Edge(start, end), _)| *start != entity && *end != entity)
                .map(|(edge, _)| edge),
            point,
        )
    }
}

pub(crate) fn show_strength(
    pointer: Res<CursorWorldCoords>,
    // mut clear: ResMut<ClearColor>,
    field: Field<(), ()>,
    cfg: Res<Cfg>,
) {
    if cfg.debug_print {
        let cursor = pointer.0;
        let blue = field
            .points_strength((&field.nodes).iter().map(|x| x.0), cursor)
            .length();
        let red = field
            .lines_strength((&field.edges).iter().map(|x| x.0), cursor)
            .length();
        let green = field.boundary_strength(cursor).length();
        eprintln!("{red} {blue} {green}");
        // clear.0 = Color::rgb(10. / red, 1. / green, 100000. / blue);
    }
}

pub(crate) fn debug_field(field: Field<Entity, Entity>, mut gizmos: Gizmos, cfg: Res<Cfg>) {
    if cfg.debug_vecs {
        for (point, entity) in &field.nodes {
            let point = point.translation.xy();
            let nodes = field.points_strength_except(entity, point);
            let edges = field.lines_strength_except(entity, point);
            let bounds = field.boundary_strength(point);
            gizmos.line_gradient_2d(point, point + nodes, Color::RED, Color::WHITE);
            gizmos.line_gradient_2d(point, point + edges, Color::BLUE, Color::WHITE);
            gizmos.line_gradient_2d(point, point + bounds, Color::GREEN, Color::WHITE);
        }
    }
}
