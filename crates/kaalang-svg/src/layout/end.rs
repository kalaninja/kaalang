//! Places the implicit end block below every producer of the `result` wire.

use kaalang_model::ExecutionPlan;

use super::{Builder, Incoming, NodeId, Placed, Point, compact_points, skewer_x};

/// Height of the junction every alternative `result` producer enters, measured
/// up from the top edge of the end node.
const JUNCTION_GAP: i32 = 32;

pub(super) fn dimensions() -> (i32, i32, Vec<String>) {
    (180, 58, Vec::new())
}

impl Builder<'_> {
    pub(super) fn place_end(
        &mut self,
        index: usize,
        body: &ExecutionPlan,
        skewer: usize,
        top: i32,
        incoming: Incoming,
    ) -> Placed {
        let placed = self.place(body, skewer, top, incoming);
        let end = self.add_block_node(index, skewer, placed.bottom + self.vertical_gap);
        self.connect_arrivals(end);

        Placed {
            bottom: self.bottom_anchor(end).y,
            arrivals: Vec::new(),
        }
    }

    /// Draws every alternative `result` producer into one horizontal junction
    /// above the end node, mirroring the distributor that fans a select out to
    /// its cases. Each branch drops vertically onto the junction and the
    /// junction makes the single descent into the node, so the runs the
    /// arrivals share are one path rather than connections hidden behind each
    /// other.
    fn connect_arrivals(&mut self, end: NodeId) {
        let end_top = self.top_anchor(end);
        // The end node's capture label is always the one word `result`, which
        // fits under this gap; the junction row therefore clears its halo.
        let junction_y = end_top.y - JUNCTION_GAP;
        for arrival in std::mem::take(&mut self.arrivals) {
            let start = self.anchor(arrival.origin);
            let lane_x = skewer_x(arrival.skewer);
            let points = if lane_x == end_top.x {
                compact_points([
                    start,
                    Point {
                        x: lane_x,
                        y: start.y,
                    },
                    end_top,
                ])
            } else {
                compact_points([
                    start,
                    Point {
                        x: lane_x,
                        y: start.y,
                    },
                    Point {
                        x: lane_x,
                        y: junction_y,
                    },
                    Point {
                        x: end_top.x,
                        y: junction_y,
                    },
                    end_top,
                ])
            };
            self.connect(arrival, end, points);
        }
    }
}
