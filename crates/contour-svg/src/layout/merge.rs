//! Places a merge and routes every continuing branch into it.

use contour_model::Merge;

use super::{
    Builder, Incoming, Origin, Placed, Point, Side, Tail, VERTICAL_GAP, compact_points, skewer_x,
};

pub(super) fn dimensions() -> (i32, i32, Vec<String>) {
    (38, 38, Vec::new())
}

impl Builder<'_> {
    pub(super) fn place_merge(
        &mut self,
        arrivals: Vec<Tail>,
        merge: &Merge,
        skewer: usize,
        bottom: i32,
    ) -> Placed {
        // Branches that end the flow may lead the ones that reach the merge, and
        // the leading skewers belong to them. The merge and its continuation
        // take the first continuing branch's skewer instead.
        let merge_skewer = arrivals.first().map_or(skewer, |arrival| arrival.skewer);
        let node = self.add_authored_node(merge.index, merge_skewer, bottom + VERTICAL_GAP);
        for arrival in arrivals {
            debug_assert_eq!(arrival.merge, Some(merge.index));
            self.connect_to_merge(arrival, node, merge_skewer);
        }
        let output = self.graph.flow.blocks[merge.index].outputs[0].to_string();
        self.place(
            &merge.next,
            merge_skewer,
            self.node_bottom(node) + VERTICAL_GAP,
            Incoming {
                origin: Origin::bottom(node),
                label: Some(output),
                skewer: merge_skewer,
            },
        )
    }

    fn connect_to_merge(&mut self, arrival: Tail, merge: super::NodeId, merge_skewer: usize) {
        let start = self.anchor(arrival.origin);
        let points = if arrival.skewer == merge_skewer && start.x == skewer_x(merge_skewer) {
            vec![start, self.top_anchor(merge)]
        } else {
            let end = self.right_anchor(merge);
            let lane_x = skewer_x(arrival.skewer);
            compact_points([
                start,
                Point {
                    x: lane_x,
                    y: start.y,
                },
                Point {
                    x: lane_x,
                    y: end.y,
                },
                end,
            ])
        };
        let label_at = Some(match arrival.origin.side {
            Side::Right => Point {
                x: i32::midpoint(start.x, skewer_x(arrival.skewer)),
                y: start.y - 8,
            },
            Side::Bottom => Point {
                x: start.x + 42,
                y: start.y + 18,
            },
        });
        self.connect_points(arrival.origin.node, merge, arrival.label, points, label_at);
    }
}
