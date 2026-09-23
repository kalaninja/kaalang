//! Converts the verified arrangement's corridors into pixels and checks
//! RFC 0002 §8. A failed realization is a renderer defect.

use kaalang_compiler::geometry::{
    bundle_meetings, compatible, overlaps_itself, straighten, turns_downward,
};
use kaalang_compiler::topology::{Destination, ExitId, NodeId, Source, Vertex};
use kaalang_compiler::{SemanticModel, Side};

use super::{Connection, Point, Rows, Scene};

pub(super) fn emit(scene: &Scene, rows: &Rows) -> Vec<Connection> {
    scene
        .topology
        .connections
        .iter()
        .enumerate()
        .map(|(index, wire)| {
            let route = &scene.arrangement.routes[index];
            let start = match wire.source {
                Source::Exit(exit) => exit_anchor(scene, exit, wire.destination),
                Source::Junction(junction) => junction_point(scene, rows, junction),
            };
            let end = match wire.destination {
                Vertex::Node(node) => scene.top_anchor(node),
                Vertex::Junction(junction) => junction_point(scene, rows, junction),
            };

            // A side exit reaches its branch column horizontally before it
            // descends into the row gaps. The brancher's footprint reserves
            // this space beside the node.
            let mut points = route.corners(start, |c| scene.column_x(c), |l| rows.line_y(l));
            points.push(end);

            Connection {
                source: wire.source,
                destination: wire.destination,
                points: straighten(points),
            }
        })
        .collect()
}

pub(super) fn exit_anchor(scene: &Scene, exit: ExitId, destination: Destination) -> Point {
    match destination {
        Destination::Node(NodeId::Case { choice, branch })
            if exit.node == NodeId::Block(choice) =>
        {
            super::choice::exit_anchor(scene.node(exit.node), branch)
        }
        _ => scene.exit_anchor(exit),
    }
}

/// Realizes every iteration back edge: out of its tail, up the lane the arrangement
/// chose beside its body, and horizontally into its entry junction
/// (RFC 0002 §8). The lane sits just outside the body it climbs past.
pub(super) fn back_edges(scene: &Scene, model: &SemanticModel, rows: &Rows) -> Vec<Connection> {
    let mut drawn: Vec<(usize, i32)> = Vec::new();
    let mut connections = Vec::new();
    // Innermost first, so a nested rail is stroked before the one that encloses
    // it, as RFC 0002 §7 asks.
    for (index, loop_) in scene.topology.loops.iter().enumerate().rev() {
        let from = junction_point(scene, rows, loop_.tail);
        let end = junction_point(scene, rows, loop_.entry);
        // The body still stands where it was numbered, so the recorded column
        // is realized through the same map every node and route uses.
        if !scene.arrangement.back_routes.is_empty() {
            connections.push(Connection {
                source: Source::Junction(loop_.tail),
                destination: Destination::Junction(loop_.entry),
                points: bent_back_edge(scene, rows, index, from, end),
            });
            continue;
        }
        let anchor = contour_anchor(scene, index);
        let aside = contour_x(scene, model, index, from, end, &drawn, anchor);
        drawn.push((loop_.header, aside));
        connections.push(Connection {
            source: Source::Junction(loop_.tail),
            destination: Destination::Junction(loop_.entry),
            points: straighten(back_edge_points(from, aside, end)),
        });
    }
    connections
}

/// A common column map for every climb in a drawing with routing bends.
/// It preserves the abstract order of both sides of every column, leaving
/// node boxes and their labels inside the space between a column and a rail.
pub(super) fn back_edge_x(scene: &Scene, index: usize, column: i32) -> i32 {
    let contour = scene.arrangement.contours[index];
    let offset =
        super::NODE_WIDTH / 2 + super::label::LABEL_WIDTH + (contour.lane as i32 + 1) * super::LANE;
    scene.column_x(column)
        + match contour.side {
            Side::Left => -offset,
            Side::Right => offset,
        }
}

fn bent_back_edge(scene: &Scene, rows: &Rows, index: usize, from: Point, end: Point) -> Vec<Point> {
    let x = |column| back_edge_x(scene, index, column);
    let Some(route) = scene.arrangement.back_routes.get(&index) else {
        return back_edge_points(from, x(scene.arrangement.contours[index].column), end);
    };
    let mut points = route.corners(end, x, |l| rows.line_y(l));
    points.push(Point {
        x: x(route.arrival),
        y: from.y,
    });
    points.push(from);
    points.reverse();
    straighten(points)
}

/// The climb of one iteration back edge: out of its tail, up `aside`, and into its
/// entry. Each caller smooths the result its own way.
pub(super) fn back_edge_points(from: Point, aside: i32, end: Point) -> Vec<Point> {
    vec![
        from,
        Point {
            x: aside,
            y: from.y,
        },
        Point { x: aside, y: end.y },
        end,
    ]
}

/// The recorded contour column after the scene's horizontal translation.
pub(super) fn contour_anchor(scene: &Scene, index: usize) -> i32 {
    let origin =
        scene.node(NodeId::Start).x - scene.column_x(scene.column(Vertex::Node(NodeId::Start)));
    origin + scene.column_x(scene.arrangement.contours[index].column)
}

/// Realizes the recorded contour side, column, and lane. Measured body bounds
/// may push the rail outward, never inward past `anchor` or into another corridor.
/// Body membership is independent of rank; continuations are excluded.
pub(super) fn contour_x(
    scene: &Scene,
    model: &SemanticModel,
    index: usize,
    from: Point,
    end: Point,
    drawn: &[(usize, i32)],
    anchor: i32,
) -> i32 {
    let loop_ = scene.topology.loops[index];
    let contour = scene.arrangement.contours[index];
    let (left, right) = body_extent(scene, index).unwrap_or((from.x.min(end.x), from.x.max(end.x)));
    let (left, right) = (left.min(from.x).min(end.x), right.max(from.x).max(end.x));
    let (left, right) = (left.min(anchor), right.max(anchor));
    let step = (contour.lane as i32 + 1) * super::LANE;
    // A rail already drawn beside a nested body is part of this body too, and
    // one lane past it is enough: the arrangement put this contour outside
    // that one, so the lane step above is measured from the body and this only
    // holds the two apart.
    let body = model.analysis.flow.blocks[loop_.header]
        .loop_end
        .expect("a loop owns a body");
    let nested = drawn
        .iter()
        .filter(|(header, _)| (loop_.header..body).contains(header))
        .map(|&(_, rail)| rail);
    match contour.side {
        Side::Left => nested.fold(left - step, |aside, rail| aside.min(rail - super::LANE)),
        Side::Right => nested.fold(right + step, |aside, rail| aside.max(rail + super::LANE)),
    }
}

/// Horizontal extent of the model-defined body, including junctions and
/// vertices below the tail. The caller adds entry and tail route endpoints.
fn body_extent(scene: &Scene, index: usize) -> Option<(i32, i32)> {
    let loop_ = scene.topology.loops[index];
    let ends = [Vertex::Junction(loop_.entry), Vertex::Junction(loop_.tail)];
    scene.bodies[index]
        .iter()
        .copied()
        .filter(|vertex| !ends.contains(vertex))
        .map(|vertex| match vertex {
            Vertex::Node(id) => {
                let (left, _, right, _) = Scene::bounds(scene.node(id));
                (left, right)
            }
            Vertex::Junction(junction) => {
                let at = scene
                    .junction_at(junction)
                    .expect("a body junction has an incident route")
                    .x;
                (at, at)
            }
        })
        .reduce(|(left, right), (edge, beyond)| (left.min(edge), right.max(beyond)))
}

fn junction_point(scene: &Scene, rows: &Rows, junction: usize) -> Point {
    let row = scene.rank(Vertex::Junction(junction));
    Point {
        x: scene.column_x(scene.column(Vertex::Junction(junction))),
        y: rows.line_y(kaalang_compiler::RunLine::Rank(row)),
    }
}

/// Checks emitted back edges against the recorded side and the whole body's
/// horizontal extent, independently of rank (RFC 0002 §8).
fn verify_back_edges(scene: &Scene) -> Option<String> {
    let climbs = (0..scene.topology.loops.len())
        .map(|index| {
            let edge = scene.back_edge(index)?;
            if !edge.points.windows(2).any(|pair| pair[1].y < pair[0].y)
                || edge.points.windows(2).any(|pair| pair[1].y > pair[0].y)
                || edge.points.len() < 4
            {
                return None;
            }
            let points = &edge.points[1..edge.points.len() - 1];
            Some((
                points.iter().map(|p| p.x).min()?,
                points.iter().map(|p| p.x).max()?,
            ))
        })
        .collect::<Vec<_>>();
    for (index, loop_) in scene.topology.loops.iter().enumerate() {
        let at = format!(
            "the iteration back edge of the cycle at block {}",
            loop_.header + 1
        );
        let Some(climb) = climbs[index] else {
            return Some(format!("{at} is not drawn, or does not climb"));
        };
        let side = scene.arrangement.contours[index].side;
        let clear = |other: i32| match side {
            Side::Left => climb.1 < other,
            Side::Right => climb.0 > other,
        };
        // Nested contours must stay inside this one even when row spans do not overlap.
        if let Some(nested) = (0..climbs.len())
            .filter(|&other| other != index)
            .filter(|&other| {
                scene.bodies[index].contains(&Vertex::Junction(scene.topology.loops[other].entry))
            })
            .find_map(|other| {
                climbs[other]
                    .map(|(left, right)| match side {
                        Side::Left => left,
                        Side::Right => right,
                    })
                    .filter(|&rail| !clear(rail))
            })
        {
            return Some(format!(
                "{at} climbs inside the back edge nested in its body at {nested}"
            ));
        }
        // Its own entry and tail are part of the body too; use their drawn ends.
        let edge = scene.back_edge(index).expect("the climb above has an edge");
        let endpoints = [edge.points[0].x, edge.points[edge.points.len() - 1].x];
        let (left, right) = body_extent(scene, index).unwrap_or((endpoints[0], endpoints[0]));
        let left = left.min(endpoints[0]).min(endpoints[1]);
        let right = right.max(endpoints[0]).max(endpoints[1]);
        if !clear(match side {
            Side::Left => left,
            Side::Right => right,
        }) {
            return Some(format!("{at} climbs inside its body"));
        }
    }
    None
}

/// Reports the first RFC 0002 §8 rule the emitted routes break, if any.
pub(super) fn verify(scene: &Scene) -> Option<String> {
    if let Some(reason) = super::choice::verify(scene) {
        return Some(reason);
    }
    if let Some(reason) = super::end::verify(scene) {
        return Some(reason);
    }
    if let Some(reason) = verify_back_edges(scene) {
        return Some(reason);
    }
    for connection in &scene.connections {
        if connection.points.len() < 2 {
            return Some("a connection has no route".to_owned());
        }
        for segment in connection.points.windows(2) {
            let (a, b) = (segment[0], segment[1]);
            if a.x != b.x && a.y != b.y {
                return Some("a connection bends other than at a right angle".to_owned());
            }
            if b.y < a.y && !scene.is_back_edge(connection) {
                return Some("a connection moves upward".to_owned());
            }
            if scene
                .nodes
                .iter()
                .filter(|node| !touches(connection, node.id))
                .any(|node| enters(a, b, Scene::bounds(node)))
            {
                return Some("a connection passes through a node".to_owned());
            }
            if scene
                .parameters
                .as_ref()
                .is_some_and(|parameters| enters(a, b, Scene::parameter_bounds(parameters)))
            {
                return Some("a connection passes through the parameter panel".to_owned());
            }
        }
        if overlaps_itself(&connection.points) {
            return Some("a connection overlaps itself".to_owned());
        }
        if matches!(connection.destination, Destination::Junction(_))
            && turns_downward(&connection.points)
        {
            return Some("a merge side route turns downward before its endpoint".to_owned());
        }
    }

    for (index, connection) in scene.connections.iter().enumerate() {
        for other in &scene.connections[index + 1..] {
            if collapsed(connection) || collapsed(other) {
                continue;
            }
            let shared =
                connection.source == other.source || connection.destination == other.destination;
            let meetings = if shared {
                bundle_meetings(&connection.points, &other.points)
            } else {
                common_ends(scene, connection, other)
            };
            if !compatible(&connection.points, &other.points, shared, &meetings) {
                return Some(format!(
                    "{} crosses {}",
                    name(scene, connection),
                    name(scene, other)
                ));
            }
        }
    }

    None
}

/// A structural hop compacted to one point has no ink to intersect.
fn collapsed(connection: &Connection) -> bool {
    connection
        .points
        .windows(2)
        .all(|segment| segment[0] == segment[1])
}

/// Shared endpoint positions, including junctions joined by collapsed hops.
/// Node ports on different box edges do not coincide; shared runs are checked separately.
fn common_ends(scene: &Scene, left: &Connection, right: &Connection) -> Vec<Point> {
    let ends = |connection: &Connection| {
        [
            (
                Vertex::from(connection.source),
                connection.points.first().copied(),
            ),
            (connection.destination, connection.points.last().copied()),
        ]
    };
    let mut meetings = Vec::new();
    for (vertex, point) in ends(left) {
        for (other, at) in ends(right) {
            if point != at {
                continue;
            }
            let collapsed = matches!((vertex, other), (Vertex::Junction(_), Vertex::Junction(_)))
                && scene.connections.iter().any(|edge| {
                    let endpoints = (Vertex::from(edge.source), edge.destination);
                    (endpoints == (vertex, other) || endpoints == (other, vertex))
                        && collapsed(edge)
                        && edge.points.first().copied() == point
                });
            if vertex == other || collapsed {
                meetings.extend(point);
            }
        }
    }
    meetings
}

/// Names one connection by the ends it joins, for a diagnostic.
fn name(scene: &Scene, connection: &Connection) -> String {
    let end = |vertex: Vertex| match vertex {
        Vertex::Node(node) => match scene
            .topology
            .nodes
            .iter()
            .find(|projected| projected.id == node)
        {
            Some(projected) => {
                format!(
                    "{:?}({})",
                    projected.kind,
                    scene.captions.label(node).as_ref().trim()
                )
            }
            None => format!("{node:?}"),
        },
        Vertex::Junction(junction) => {
            let wires = scene.captions.junction_wires(junction);
            if wires.is_empty() {
                format!("junction {}", junction + 1)
            } else {
                format!("merge({})", wires.join("+"))
            }
        }
    };
    let branch = match connection.source {
        Source::Exit(ExitId {
            branch: Some(branch),
            ..
        }) => format!(" branch {}", branch + 1),
        _ => String::new(),
    };
    format!(
        "[{from}{branch} -> {to}]",
        from = end(Vertex::from(connection.source)),
        to = end(connection.destination)
    )
}

fn touches(connection: &Connection, node: NodeId) -> bool {
    matches!(connection.source, Source::Exit(exit) if exit.node == node)
        || connection.destination == Destination::Node(node)
}

/// Whether any segment of a route passes through a rectangle.
pub(super) fn crosses(points: &[Point], bounds: (i32, i32, i32, i32)) -> bool {
    points
        .windows(2)
        .any(|segment| enters(segment[0], segment[1], bounds))
}

/// Whether a segment passes through a rectangle rather than merely touching it.
/// The model checks its own boundaries with this same definition.
pub(super) use kaalang_compiler::geometry::enters;

#[cfg(test)]
mod tests {
    use super::*;
    use kaalang_compiler::topology::{ExitId, Topology};

    fn routes(paths: &[&[(i32, i32)]]) -> Scene {
        let connections = paths
            .iter()
            .enumerate()
            .map(|(index, points)| Connection {
                source: Source::Exit(ExitId::of(NodeId::Block(0))),
                destination: Destination::Node(NodeId::Block(index + 2)),
                points: points.iter().map(|&(x, y)| Point { x, y }).collect(),
            })
            .collect();
        Scene {
            narrow: false,
            reach: std::collections::BTreeMap::new(),
            slack: 0,
            bodies: Vec::new(),
            region_bodies: Vec::new(),
            width: 10,
            height: 10,
            topology: Topology::default(),
            arrangement: kaalang_compiler::Arrangement::default(),
            captions: std::rc::Rc::default(),
            nodes: vec![],
            parameters: None,
            connections,
            labels: vec![],
            loop_regions: Vec::new(),
        }
    }

    #[test]
    fn bundled_routes_may_share_a_rail_but_must_not_cross() {
        let rail: &[&[(i32, i32)]] = &[
            &[(0, 0), (0, 2), (4, 2), (4, 8)],
            &[(0, 0), (0, 2), (8, 2), (8, 8)],
        ];
        let crossing: &[&[(i32, i32)]] = &[
            &[(0, 0), (0, 2), (6, 2), (6, 8)],
            &[(0, 0), (4, 0), (4, 4), (8, 4), (8, 8)],
        ];
        for (paths, crosses) in [(rail, false), (crossing, true)] {
            let mut scene = routes(paths);
            assert_eq!(verify(&scene).is_some(), crosses);
            // Reversing time turns a distributor into a merge with the same geometry.
            for (index, route) in scene.connections.iter_mut().enumerate() {
                route.source = Source::Exit(ExitId::of(NodeId::Block(index)));
                route.destination = Destination::Node(NodeId::Block(2));
                route.points.reverse();
                for point in &mut route.points {
                    point.y = 8 - point.y;
                }
            }
            assert_eq!(verify(&scene).is_some(), crosses);
        }
    }

    #[test]
    fn invalid_routes_are_rejected_before_serialization() {
        for (path, reason) in [
            (vec![(0, 0)], "a connection has no route"),
            (
                vec![(0, 0), (4, 4)],
                "a connection bends other than at a right angle",
            ),
            (vec![(0, 4), (0, 0)], "a connection moves upward"),
            (
                vec![(0, 0), (0, 2), (4, 2), (2, 2)],
                "a connection overlaps itself",
            ),
        ] {
            assert_eq!(verify(&routes(&[&path])).as_deref(), Some(reason));
        }
        let mut scene = routes(&[&[(0, 0), (0, 8)]]);
        scene.nodes.push(super::super::Node {
            id: NodeId::Block(1),
            x: 0,
            y: 4,
            width: 2,
            height: 2,
            lines: vec![],
        });
        assert_eq!(
            verify(&scene).as_deref(),
            Some("a connection passes through a node")
        );

        let mut scene = routes(&[&[(0, 0), (0, 8)], &[(0, 0), (0, 4), (4, 4), (4, 8)]]);
        assert_eq!(verify(&scene), None);
        scene.connections[1].source = Source::Exit(ExitId::of(NodeId::Block(1)));
        assert!(
            verify(&scene).is_some_and(|reason| reason.contains("crosses")),
            "two routes from unrelated exits must not share a run"
        );
    }

    #[test]
    fn a_branch_may_turn_into_the_merge_column_at_its_source() {
        let mut scene = routes(&[&[(0, 0), (4, 0), (4, 8)]]);
        scene.connections[0].destination = Destination::Junction(0);
        assert_eq!(verify(&scene), None);

        scene.connections[0].points = [(0, 0), (0, 4), (4, 4), (4, 8)]
            .into_iter()
            .map(|(x, y)| Point { x, y })
            .collect();
        assert_eq!(
            verify(&scene).as_deref(),
            Some("a merge side route turns downward before its endpoint")
        );
    }
}
