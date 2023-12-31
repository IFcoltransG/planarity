use crate::{
    config::Cfg, cursor::TrackCursor, Edge, Endpoint, LevelCleanup, LineIntersects, Node, Velocity,
};
use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use bevy_prototype_lyon::{prelude::*, shapes::Circle};
use petgraph::{
    prelude::*,
    stable_graph::IndexType,
    visit::{IntoEdgeReferences, IntoNodeReferences},
};
use rand::{
    distributions::{Distribution, WeightedIndex},
    thread_rng, Rng,
};
use std::f32::consts::{PI, TAU};

#[derive(Resource, Clone, Debug)]
pub(crate) struct PreviousGraphs<N = Endpoint, E = ()>(pub Vec<StableGraph<N, E, Undirected>>);

pub(crate) fn make_network(
    mut commands: Commands,
    cfg: Res<Cfg>,
    mut previous_graphs: ResMut<PreviousGraphs>,
) {
    let mut rng = thread_rng();

    let graph = make_graph(&mut rng, cfg.num_circles);
    let graph = graph.filter_map(
        |index, _| (graph.neighbors(index).count() > 1).then(|| ()),
        |_, _| Some(()),
    );

    let mut position = || {
        let random_offset = cfg.node_starting_random_offset;
        Vec2::from_angle(rng.gen_range(0.0..TAU))
            * if random_offset != 0. {
                cfg.node_starting_distance + rng.gen_range((-random_offset)..random_offset)
            } else {
                cfg.node_starting_distance
            }
    };
    let mut graph = graph.map(
        |_, _| {
            let pos = position();
            make_endpoint(&mut commands, make_node(pos), pos)
        },
        |_, _| (),
    );

    let previous_graphs = &mut previous_graphs.0;
    previous_graphs.clear();
    previous_graphs.reserve(graph.node_count().saturating_sub(cfg.limit_nodes));

    fn degree_weighted_random(
        graph: &StableGraph<Endpoint, (), Undirected>,
        rng: &mut impl Rng,
        nodes: impl Iterator<Item = NodeIndex>,
    ) -> Option<NodeIndex> {
        let nodes: Vec<_> = nodes.collect();
        let weights: Vec<_> = nodes
            .iter()
            .map(|node| graph.neighbors(*node).count().ilog2())
            .collect();
        let distribution = WeightedIndex::new(weights).unwrap();
        nodes.get(distribution.sample(rng)).copied()
    }

    while cfg.limit_nodes < graph.node_count() {
        eprintln!("{}", graph.node_count());
        previous_graphs.push(graph.clone());
        let node = degree_weighted_random(
            &graph,
            &mut rng,
            graph.node_references().map(|(node, _)| node),
        )
        .unwrap();
        let Some(other) = degree_weighted_random(&graph, &mut rng, graph.neighbors(node)) else {
            graph.remove_node(node).unwrap();
            continue;
        };
        let Endpoint(entity, _) = graph.node_weight(other).unwrap();
        commands
            .entity(*entity)
            .insert(Visibility::Hidden)
            .remove::<Node>();
        merge_nodes(&mut graph, node, other);
    }

    add_edges(commands, graph);
}

fn add_edges(mut commands: Commands, graph: StableGraph<Endpoint, (), Undirected>) {
    for edge in graph.edge_references() {
        let start = &graph[edge.source()];
        let end = &graph[edge.target()];
        make_edge(&mut commands, start, end);
    }
}

pub(crate) fn make_node(position: Vec2) -> impl Bundle {
    let path = ShapePath::build_as(&Circle {
        radius: 10.0,
        center: Vec2::ZERO,
    });
    (
        Node,
        LineIntersects::Unsolved,
        Name::new("Vertex"),
        LevelCleanup,
        ShapeBundle {
            path,
            spatial: SpatialBundle::from_transform(Transform::from_translation(
                position.extend(1.),
            )),
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
        Velocity(None),
    )
}

pub(crate) fn make_edge(
    commands: &mut Commands,
    Endpoint(start, start_pos): &Endpoint,
    Endpoint(end, end_pos): &Endpoint,
) -> Edge {
    let path = GeometryBuilder::build_as(&shapes::Line(*start_pos, *end_pos));
    let edge = Edge(*start, *end);
    commands.spawn((
        edge,
        Name::new("Edge"),
        LevelCleanup,
        ShapeBundle { path, ..default() },
        Stroke::color(Color::RED),
    ));
    edge
}

pub(crate) fn make_endpoint(
    commands: &mut Commands,
    bundle: impl Bundle,
    position: Vec2,
) -> Endpoint {
    Endpoint(commands.spawn(bundle).id(), position)
}

fn make_graph(mut rng: impl Rng, num_circles: usize) -> StableUnGraph<(), ()> {
    fn sort_pair(x: usize, y: usize) -> (usize, usize) {
        (x.min(y), x.max(y))
    }
    let num_nodes = num_circles * (num_circles - 1);
    // TODO: this is wrong number
    let num_edges = num_circles;
    let circles = (0..num_circles)
        .map(|_| rng.gen::<[f32; 2]>().into())
        .collect::<Vec<Vec2>>();
    let mut scratch_circles = circles.iter().enumerate().collect::<Vec<_>>();
    let mut graph = UnGraphMap::with_capacity(num_nodes, num_edges);
    // for each circle-circle pair, connect it to each of its neighbours when
    // sorting each circle's intersections by how far round the circle they are;
    // that is, connect intersections that are connected by an arc if there's no
    // other intersection on that arc, including 0,0 the distance along each
    // circle with midpoint A, of circle with midpoint B is proportional to
    // angle OAB, so that's a proxy for sorting
    for (i, circle) in circles.iter().enumerate() {
        let arc_dist = |other_centre| circle.angle_between(other_centre - *circle).rem_euclid(PI);
        // TODO: Check they're in general position
        scratch_circles.sort_by(|(_, &x), (_, &y)| arc_dist(x).total_cmp(&arc_dist(y)));
        // add adjacent intersections between circle i and other circles
        // to the graph as edges
        for [(a, a_vec), (b, b_vec)] in scratch_circles.array_windows() {
            if a_vec == b_vec {
                panic!("shouldn't have two intersections at the same point, {a} and {b} at {a_vec} and {b_vec}");
            }
            let a = sort_pair(*a, i);
            let b = sort_pair(*b, i);
            graph.add_edge(a, b, ());
        }
    }
    // then map graph into Entities...?
    graph
        .into_graph()
        .map(
            |_node_index, _node_weight| (),
            |_edge_index, _edge_weight| (),
        )
        .into()
}

fn merge_nodes<N, E, Ix: IndexType>(
    graph: &mut StableGraph<N, E, Undirected, Ix>,
    target: NodeIndex<Ix>,
    other: NodeIndex<Ix>,
) {
    while let Some((edge_index, node_index)) = graph.neighbors(other).detach().next(&graph) {
        let edge_weight = graph.remove_edge(edge_index).unwrap();
        if target == node_index {
            continue;
        }
        graph.add_edge(node_index, target, edge_weight);
    }
    graph.remove_node(other).unwrap();
}

fn replace_graph(
    mut commands: Commands,
    graph: StableGraph<Endpoint, (), Undirected>,
    edges: Query<Entity, With<Edge>>,
) {
    for edge in &edges {
        commands.entity(edge).despawn();
    }
    for Endpoint(entity, _) in graph.node_weights() {
        commands
            .entity(*entity)
            .insert((Node, LineIntersects::Unsolved, Visibility::Inherited));
    }
    add_edges(commands, graph);
}

pub(crate) fn bigger_graph(
    commands: Commands,
    mut previous: ResMut<PreviousGraphs>,
    edges: Query<Entity, With<Edge>>,
) {
    if let Some(graph) = previous.0.pop() {
        replace_graph(commands, graph, edges);
    }
}
