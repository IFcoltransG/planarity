use bevy::{
    ecs::{query::WorldQuery, system::SystemParam},
    math::Vec3Swizzles,
    prelude::*,
};
use geo::{Closest, ClosestPoint, Line, Point};
use itertools::Itertools;

use crate::{config::Cfg, cursor::CursorWorldCoords, Edge, Node};

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
        base: f64,
    ) -> Vec2 {
        let nodes = nodes
            .into_iter()
            .map(|transform| transform.translation.xy() - point)
            .collect_vec();
        softargmin(&nodes, base)
            .iter()
            .map(|(weight, vec)| *weight as f32 * *vec)
            .sum()
    }

    pub(crate) fn lines_strength<'a>(
        &self,
        edges: impl IntoIterator<Item = &'a Edge>,
        point: Vec2,
        base: f64,
    ) -> Vec2 {
        fn to_point(v: Vec2) -> Point<f32> {
            Into::<[f32; 2]>::into(v).into()
        }
        fn to_vec(v: Point<f32>) -> Vec2 {
            Into::<[f32; 2]>::into(v).into()
        }
        let geo_point = to_point(point);
        let result = edges
            .into_iter()
            .map(|Edge(start, end)| {
                let (start, end) = (
                    to_point(self.nodes.get(*start).unwrap().0.translation.xy()),
                    to_point(self.nodes.get(*end).unwrap().0.translation.xy()),
                );
                let closest = match <Line<f32>>::new(start, end).closest_point(&geo_point) {
                    Closest::Intersection(closest) | Closest::SinglePoint(closest) => closest,
                    Closest::Indeterminate => return Vec2::ZERO,
                };
                to_vec(closest) - point
            })
            .collect_vec();

        let result = softargmin(&result, base)
            .iter()
            .map(|(weight, vec)| *weight as f32 * *vec)
            .sum();
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
    pub(crate) fn points_strength_except(&self, entity: Entity, point: Vec2, base: f64) -> Vec2 {
        self.points_strength(
            self.nodes
                .iter()
                .filter(|(_, other_entity)| *other_entity != entity)
                .map(|(edge, _)| edge),
            point,
            base,
        )
    }

    pub(crate) fn lines_strength_except(&self, entity: Entity, point: Vec2, base: f64) -> Vec2 {
        self.lines_strength(
            self.edges
                .iter()
                .filter(|(Edge(start, end), _)| *start != entity && *end != entity)
                .map(|(edge, _)| edge),
            point,
            base,
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
            .points_strength((&field.nodes).iter().map(|x| x.0), cursor, cfg.field_base)
            .length();
        let red = field
            .lines_strength((&field.edges).iter().map(|x| x.0), cursor, cfg.field_base)
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
            let nodes = field.points_strength_except(entity, point, cfg.field_base);
            let edges = field.lines_strength_except(entity, point, cfg.field_base);
            let bounds = field.boundary_strength(point);
            gizmos.line_gradient_2d(point, point + nodes, Color::RED, Color::WHITE);
            gizmos.line_gradient_2d(point, point + edges, Color::BLUE, Color::WHITE);
            gizmos.line_gradient_2d(point, point + bounds, Color::GREEN, Color::WHITE);
        }
    }
}

pub(crate) fn softargmin(vecs: &[Vec2], sigma: f64) -> Vec<(f64, Vec2)> {
    let mut vecs = vecs
        .iter()
        .map(|vec| (sigma.powf(-vec.length() as f64), *vec))
        .collect_vec();
    let sum: f64 = vecs.iter().map(|(weight, _)| weight).sum();
    vecs.iter_mut().for_each(|(weight, _)| {
        *weight /= sum;
    });
    vecs
}
