//! Cycle-boundary rules for renderer-proposed arrangements.

use std::collections::BTreeSet;

use kaalang_compiler::geometry::{Point, contains, enters, inside, overlaps};
use kaalang_compiler::topology::{Connection, Topology, Vertex};
use kaalang_compiler::{Arrangement, ArrangementGeometry, Flow};

/// Boundary contents in `Topology::loop_boundaries` order, cached because
/// ownership is independent of candidate coordinates.
pub(super) struct Bodies(Vec<Body>);

struct Body {
    owned: BTreeSet<Vertex>,
    foreign: Vec<Vertex>,
    internal: Vec<usize>,
    foreign_routes: Vec<usize>,
    own_back_edge: Option<usize>,
    foreign_loops: Vec<usize>,
    nests: Vec<usize>,
    beside: Vec<usize>,
}

impl Bodies {
    pub(super) fn of(flow: &Flow, topology: &Topology) -> Self {
        Self(
            topology
                .loop_boundaries
                .iter()
                .map(|boundary| {
                    let owned = owned(flow, topology, boundary.header);
                    let inside = |vertex: Vertex| owned.contains(&vertex);
                    let interface = |edge: &Connection| {
                        edge.destination == boundary.entry || boundary.result == Some(edge.source)
                    };
                    let (mut internal, mut foreign_routes) = (Vec::new(), Vec::new());
                    for (index, edge) in topology.connections.iter().enumerate() {
                        let both = inside(Vertex::from(edge.source)) && inside(edge.destination);
                        if both {
                            internal.push(index);
                        } else if !interface(edge) {
                            foreign_routes.push(index);
                        }
                    }
                    let foreign_loops = topology
                        .loops
                        .iter()
                        .enumerate()
                        .filter_map(|(index, loop_)| {
                            (!(boundary.header..boundary.end).contains(&loop_.header))
                                .then_some(index)
                        })
                        .collect();
                    let own_back_edge = topology
                        .loops
                        .iter()
                        .position(|loop_| loop_.header == boundary.header);
                    let (mut nests, mut beside) = (Vec::new(), Vec::new());
                    for (index, other) in topology.loop_boundaries.iter().enumerate() {
                        if other.header == boundary.header {
                            continue;
                        }
                        if (boundary.header + 1..boundary.end).contains(&other.header) {
                            nests.push(index);
                        } else if !(other.header + 1..other.end).contains(&boundary.header) {
                            beside.push(index);
                        }
                    }
                    Body {
                        foreign: topology
                            .vertices
                            .iter()
                            .copied()
                            .filter(|vertex| !owned.contains(vertex))
                            .collect(),
                        owned,
                        internal,
                        foreign_routes,
                        own_back_edge,
                        foreign_loops,
                        nests,
                        beside,
                    }
                })
                .collect(),
        )
    }

    pub(super) fn may_rise_beside(&self, arrangement: &Arrangement, edge: &Connection) -> bool {
        let landing = arrangement.column[&edge.destination];
        self.0.iter().any(|body| {
            if !body.owned.contains(&Vertex::from(edge.source))
                || body.owned.contains(&edge.destination)
            {
                return false;
            }
            let mut columns = body.owned.iter().map(|vertex| arrangement.column[vertex]);
            columns.clone().all(|column| column > landing) || columns.all(|column| column < landing)
        })
    }

    pub(super) fn completes_a_boundary(&self, edge: &Connection) -> bool {
        self.0.iter().any(|body| {
            body.owned.contains(&Vertex::from(edge.source))
                && !body.owned.contains(&edge.destination)
        })
    }

    pub(super) fn verify(
        &self,
        topology: &Topology,
        geometry: &ArrangementGeometry,
    ) -> Result<(), String> {
        if topology.loop_boundaries.is_empty() {
            return Ok(());
        }
        let route_extents = geometry.connections().map(extents).collect::<Vec<_>>();
        let back_extents = geometry.back_edges().map(extents).collect::<Vec<_>>();
        let drawn = rectangles(geometry, topology, self, &route_extents, &back_extents);
        let named = |index: usize| topology.loop_boundaries[index].header + 1;
        for (index, body) in self.0.iter().enumerate() {
            let bounds = drawn[index];
            for &other in &body.nests {
                if !contains(bounds, drawn[other]) {
                    return Err(format!(
                        "the boundary of the cycle at block {} does not hold the one at block {}",
                        named(index),
                        named(other)
                    ));
                }
            }
            for &other in &body.beside {
                if overlaps(bounds, drawn[other]) {
                    return Err(format!(
                        "the boundaries of the cycles at blocks {} and {} overlap",
                        named(index),
                        named(other)
                    ));
                }
            }
            for &vertex in &body.foreign {
                let point = geometry
                    .vertex(vertex)
                    .expect("every projected vertex has a geometry point");
                if inside(point, bounds) {
                    return Err(format!(
                        "the cycle at block {} encloses {vertex:?}",
                        named(index)
                    ));
                }
            }
            for &route in &body.foreign_routes {
                if geometry
                    .connection(route)
                    .expect("every connection has a geometry polyline")
                    .windows(2)
                    .any(|segment| enters(segment[0], segment[1], bounds))
                {
                    let edge = topology.connections[route];
                    return Err(format!(
                        "the cycle at block {} is crossed by {:?} -> {:?}, a route it does not own",
                        named(index),
                        edge.source,
                        edge.destination
                    ));
                }
            }
            for &loop_ in &body.foreign_loops {
                if geometry
                    .back_edge(loop_)
                    .expect("every iteration has a geometry polyline")
                    .windows(2)
                    .any(|segment| enters(segment[0], segment[1], bounds))
                {
                    return Err(format!(
                        "the iteration back edge of the cycle at block {} enters the boundary of the one at block {}",
                        topology.loops[loop_].header + 1,
                        named(index)
                    ));
                }
            }
        }
        Ok(())
    }
}

fn owned(flow: &Flow, topology: &Topology, header: usize) -> BTreeSet<Vertex> {
    let mut body = topology.body_vertices(flow, header);
    body.extend(
        topology
            .loop_boundaries
            .iter()
            .filter(|boundary| boundary.header == header)
            .filter_map(|boundary| boundary.result.map(Vertex::from)),
    );
    body
}

fn extents(points: &[Point]) -> (i32, i32, i32, i32) {
    points
        .iter()
        .map(|point| (point.x, point.y, point.x, point.y))
        .reduce(union)
        .unwrap_or_default()
}

fn union(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
}

fn rectangles(
    geometry: &ArrangementGeometry,
    topology: &Topology,
    bodies: &Bodies,
    route_extents: &[(i32, i32, i32, i32)],
    back_extents: &[(i32, i32, i32, i32)],
) -> Vec<(i32, i32, i32, i32)> {
    let mut settled = vec![None; topology.loop_boundaries.len()];
    let mut inward = (0..topology.loop_boundaries.len()).collect::<Vec<_>>();
    inward.sort_by_key(|&index| std::cmp::Reverse(topology.loop_boundaries[index].header));
    for index in inward {
        let body = &bodies.0[index];
        let cells = body.owned.iter().map(|&vertex| {
            let point = geometry
                .vertex(vertex)
                .expect("every body vertex has a geometry point");
            (point.x, point.y, point.x, point.y)
        });
        let held = cells
            .chain(body.internal.iter().map(|&route| route_extents[route]))
            .chain(body.own_back_edge.iter().map(|&loop_| back_extents[loop_]))
            .chain(body.nests.iter().filter_map(|&nested| settled[nested]));
        let (left, top, right, bottom) = held.reduce(union).unwrap_or_default();
        settled[index] = Some((left - 1, top - 1, right + 1, bottom + 1));
    }
    settled
        .into_iter()
        .map(|rectangle| rectangle.expect("every boundary settles once"))
        .collect()
}
