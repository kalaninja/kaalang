//! Canonical numbering for arrangement columns and routing lanes.

use crate::topology::Vertex;

use super::{Arrangement, RunLine};

pub(super) fn arrangement(built: &mut Arrangement) {
    // Normalization runs for every compaction candidate, so plain integer
    // coordinates are numbered through a sorted slice rather than a map.
    let mut coordinates = built
        .column
        .values()
        .copied()
        .chain(
            built
                .exit_offset
                .iter()
                .map(|(exit, offset)| built.column[&Vertex::Node(exit.node)] + offset),
        )
        .chain(built.all_routes().flat_map(|route| {
            [route.departure, route.arrival]
                .into_iter()
                .chain(route.runs.iter().flat_map(|run| [run.enter, run.exit]))
        }))
        .chain(built.contours.iter().map(|contour| contour.column))
        .collect::<Vec<_>>();
    coordinates.sort_unstable();
    coordinates.dedup();
    let numbered = |coordinate: i32| {
        i32::try_from(
            coordinates
                .binary_search(&coordinate)
                .expect("every coordinate was collected"),
        )
        .expect("a normalized column fits its own index")
    };
    for (exit, offset) in &mut built.exit_offset {
        let column = built.column[&Vertex::Node(exit.node)];
        *offset = numbered(column + *offset) - numbered(column);
    }
    for column in built.column.values_mut() {
        *column = numbered(*column);
    }
    for contour in &mut built.contours {
        contour.column = numbered(contour.column);
    }
    let mut used = built
        .all_routes()
        .flat_map(|route| &route.runs)
        .filter_map(|run| match run.line {
            RunLine::Lane { gap, lane } => Some((gap, lane)),
            RunLine::Rank(_) => None,
        })
        .collect::<Vec<_>>();
    used.sort_unstable();
    used.dedup();
    let mut lanes = Vec::with_capacity(used.len());
    built.gap_lanes.fill(0);
    for (gap, lane) in used {
        lanes.push(((gap, lane), built.gap_lanes[gap]));
        built.gap_lanes[gap] += 1;
    }
    for route in built.all_routes_mut() {
        route.departure = numbered(route.departure);
        route.arrival = numbered(route.arrival);
        for run in &mut route.runs {
            if let RunLine::Lane { gap, lane } = &mut run.line {
                let at = lanes
                    .binary_search_by_key(&(*gap, *lane), |&(key, _)| key)
                    .expect("every drawn lane was collected");
                *lane = lanes[at].1;
            }
            run.enter = numbered(run.enter);
            run.exit = numbered(run.exit);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::construct::{Route, Run};

    #[test]
    fn normalization_keeps_a_bend_above_a_junction_with_a_straight_arrival() {
        let parts =
            super::super::tests::parts_of(&super::super::tests::looping(&["repeat", "break"]))
                .unwrap();
        let junction = Vertex::Junction(parts.topology.loops[0].entry);
        let mut built = Arrangement {
            rank: BTreeMap::from([(junction, 1)]),
            ranks: 2,
            routes: vec![Route {
                departure: 0,
                arrival: 1,
                runs: vec![Run {
                    line: RunLine::Lane { gap: 0, lane: 0 },
                    enter: 0,
                    exit: 1,
                }],
            }],
            gap_lanes: vec![2, 0],
            ..Arrangement::default()
        };
        let separated = |built: &Arrangement| {
            let grid = super::super::verify::Grid::of(&parts.topology, built);
            grid.line(built.routes[0].runs[0].line) < grid.rank(built.rank[&junction])
        };
        assert!(separated(&built));
        arrangement(&mut built);
        assert_eq!(
            built.gap_lanes[0], 1,
            "unused lanes no longer hold a junction"
        );
        assert!(
            separated(&built),
            "normalization moved a bend onto the junction line"
        );
    }
}
