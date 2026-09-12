//! Realizes the checked arrangement as geometry, and then checks the result
//! against RFC 0002 §8.
//!
//! The arrangement already fixed which column every connection descends in,
//! which rank gaps it turns in, and the order of the runs sharing one gap. This
//! module only turns those into pixels and holds the emitted routes to the
//! spatial contract, so a disagreement between the two is a compiler bug rather
//! than another arrangement to try.

use kaalang_model::geometry::{
    bundle_meetings, compatible, overlaps_itself, straighten, turns_downward,
};
use kaalang_model::topology::{Destination, NodeId, Source, Vertex};
use kaalang_model::{SemanticModel, Side};

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
                Source::Exit(exit) => scene.exit_anchor(exit),
                Source::Junction(junction) => junction_point(scene, rows, junction),
            };
            let end = match wire.destination {
                Vertex::Node(node) => scene.top_anchor(node),
                Vertex::Junction(junction) => junction_point(scene, rows, junction),
            };

            // A side exit reaches its branch column horizontally before it
            // descends into the row gaps. The brancher's footprint reserves
            // this space beside the node.
            let mut points = vec![
                start,
                Point {
                    x: scene.column_x(route.departure),
                    y: start.y,
                },
            ];
            for run in &route.runs {
                let y = rows.lane_y(run.gap, run.lane, rows.lanes_in(run.gap));
                points.push(Point {
                    x: scene.column_x(run.enter),
                    y,
                });
                points.push(Point {
                    x: scene.column_x(run.exit),
                    y,
                });
            }
            points.push(end);

            Connection {
                source: wire.source,
                destination: wire.destination,
                points: straighten(points),
            }
        })
        .collect()
}

/// Realizes every loop return: out of its tail, up the lane the arrangement
/// chose beside its body, and horizontally into its entry junction
/// (RFC 0002 §8). The lane sits just outside the body it climbs past.
pub(super) fn returns(scene: &Scene, model: &SemanticModel, rows: &Rows) -> Vec<Connection> {
    let mut drawn: Vec<(usize, i32)> = Vec::new();
    let mut connections = Vec::new();
    // Innermost first, so a nested rail is stroked before the one that encloses
    // it, as RFC 0002 §7 asks.
    for (index, loop_) in scene.topology.loops.iter().enumerate().rev() {
        let from = junction_point(scene, rows, loop_.tail);
        let end = junction_point(scene, rows, loop_.entry);
        let aside = contour_x(scene, model, index, from, end, &drawn);
        drawn.push((loop_.header, aside));
        connections.push(Connection {
            source: Source::Junction(loop_.tail),
            destination: Destination::Junction(loop_.entry),
            points: straighten(return_points(from, aside, end)),
        });
    }
    connections
}

/// The climb of one loop return: out of its tail, up `aside`, and into its
/// entry. Each caller smooths the result its own way.
pub(super) fn return_points(from: Point, aside: i32, end: Point) -> Vec<Point> {
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

/// Where one return climbs: on the side and in the lane the arrangement chose,
/// just past everything its whole body draws, independently of its ranks.
///
/// A presentation may turn the return upward early, but the body it has to
/// clear does not shrink with it. Continuation vertices after leaving the
/// body are excluded by membership, not by their position below the tail.
///
/// The column the arrangement names is not read here, and cannot be: RFC 0003
/// §2 lets a presentation turn a return upward at a side exit and compact a
/// sole arrival, so a vertex's pixel position is not `column_x` of its abstract
/// column. What carries over is the side and the lane — the two choices that
/// decide whether the return can be drawn at all — while the pixels those land
/// on come from the boxes the body actually fills. `verify` then holds the
/// result to the same rules the arrangement was checked against.
pub(super) fn contour_x(
    scene: &Scene,
    model: &SemanticModel,
    index: usize,
    from: Point,
    end: Point,
    drawn: &[(usize, i32)],
) -> i32 {
    let loop_ = scene.topology.loops[index];
    let contour = scene.arrangement.contours[index];
    let (left, right) = body_extent(scene, index).unwrap_or((from.x.min(end.x), from.x.max(end.x)));
    let (left, right) = (left.min(from.x).min(end.x), right.max(from.x).max(end.x));
    let step = (contour.lane as i32 + 1) * super::LANE;
    // A rail already drawn beside a nested body is part of this body too, and
    // one lane past it is enough: the arrangement put this contour outside
    // that one, so the lane step above is measured from the body and this only
    // holds the two apart.
    let body = model.flow.blocks[loop_.header]
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

/// How far left and right one loop's whole body reaches.
///
/// The body is the model's own: every vertex it counts when it places the
/// return, junctions included. A wire merge or a break inside the body draws
/// no node and still fills a column, so measuring the boxes alone would leave
/// the climb a column short of clear. Membership does not depend on ranks:
/// placing part of the body below the tail does not remove it from the body.
///
/// The loop's own entry and tail are left out here and folded in by the
/// caller, which has them as drawn rather than as numbered: a compaction may
/// bring the tail's arrival in from the column it was given, and the climb
/// leaves from where it actually ends.
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
                    .connections
                    .iter()
                    .find_map(|edge| {
                        if edge.source == Source::Junction(junction) {
                            edge.points.first()
                        } else if edge.destination == vertex {
                            edge.points.last()
                        } else {
                            None
                        }
                    })
                    .expect("a body junction has an incident route")
                    .x;
                (at, at)
            }
        })
        .reduce(|(left, right), (edge, beyond)| (left.min(edge), right.max(beyond)))
}

fn junction_point(scene: &Scene, rows: &Rows, junction: usize) -> Point {
    let row = scene.rank(Vertex::Junction(junction));
    let gap = row - 1;
    let lane = scene
        .arrangement
        .deepest_lane(&scene.topology, junction, gap);
    Point {
        x: scene.column_x(scene.column(Vertex::Junction(junction))),
        y: lane.map_or_else(
            || rows.junction_y(row),
            |lane| rows.lane_y(gap, lane, rows.lanes_in(gap)),
        ),
    }
}

/// Each return climbs on the side the arrangement chose, clear of every column
/// its whole body fills, independently of its ranks (RFC 0002 §8).
///
/// The arrangement settles that on its abstract grid; this reads the emitted
/// climb, so it holds whatever the compactions left behind and does not take
/// the side and the lane on trust.
fn verify_returns(scene: &Scene) -> Option<String> {
    let climbs = scene
        .topology
        .loops
        .iter()
        .map(|loop_| {
            let edge = scene
                .connections
                .iter()
                .find(|edge| edge.source == Source::Junction(loop_.tail))?;
            edge.points
                .windows(2)
                .find(|pair| pair[1].y < pair[0].y)
                .map(|climb| climb[0].x)
        })
        .collect::<Vec<_>>();
    for (index, loop_) in scene.topology.loops.iter().enumerate() {
        let at = format!("the return of the loop at block {}", loop_.header + 1);
        let Some(climb) = climbs[index] else {
            return Some(format!("{at} is not drawn, or does not climb"));
        };
        let side = scene.arrangement.contours[index].side;
        let clear = |other: i32| match side {
            Side::Left => climb < other,
            Side::Right => climb > other,
        };
        // A return nested in this body climbs beside it too, and the two need
        // not share a single row — so no crossing check compares them, and
        // this is the only place the enclosing one is held outside the nested
        // one (RFC 0002 §8).
        if let Some(nested) = (0..climbs.len())
            .filter(|&other| other != index)
            .filter(|&other| {
                scene.bodies[index].contains(&Vertex::Junction(scene.topology.loops[other].entry))
            })
            .find_map(|other| climbs[other].filter(|&rail| !clear(rail)))
        {
            return Some(format!(
                "{at} climbs inside the return nested in its body at {nested}"
            ));
        }
        // A loop with an empty body has nothing for the climb to be inside.
        let Some((left, right)) = body_extent(scene, index) else {
            continue;
        };
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
    if let Some(reason) = super::end::verify(scene) {
        return Some(reason);
    }
    if let Some(reason) = verify_returns(scene) {
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
            let shared =
                connection.source == other.source || connection.destination == other.destination;
            let meetings = if shared {
                bundle_meetings(&connection.points, &other.points)
            } else {
                common_ends(connection, other)
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

/// Two routes that end at one vertex may meet where they both reach it, and
/// nowhere else: sharing any length still counts as an overlap.
///
/// The point has to coincide for both, which is what keeps this honest in
/// pixels. A junction is a single point, so its incoming and outgoing routes do
/// meet there; a node is a box, so two routes reaching it attach to different
/// parts of its boundary and never touch at all.
fn common_ends(left: &Connection, right: &Connection) -> Vec<Point> {
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
            if vertex == other && point == at {
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
                    scene.captions.label(node).trim()
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
        Source::Exit(kaalang_model::topology::ExitId {
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

/// Whether a segment passes through a rectangle rather than merely touching it.
pub(super) fn enters(a: Point, b: Point, bounds: (i32, i32, i32, i32)) -> bool {
    let (left, top, right, bottom) = bounds;
    if a.x == b.x {
        a.x > left && a.x < right && a.y.min(b.y) < bottom && a.y.max(b.y) > top
    } else {
        a.y > top && a.y < bottom && a.x.min(b.x) < right && a.x.max(b.x) > left
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaalang_model::topology::{ExitId, Topology};

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
            reach: (0, 0),
            slack: 0,
            bodies: Vec::new(),
            width: 10,
            height: 10,
            topology: Topology::default(),
            arrangement: kaalang_model::Arrangement::default(),
            captions: crate::captions::Captions::default(),
            nodes: vec![],
            parameters: None,
            connections,
            labels: vec![],
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
