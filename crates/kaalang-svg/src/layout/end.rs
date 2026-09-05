//! Places the one authored End below every path that reaches it.

use kaalang_model::ExecutionPlan;

use super::{
    Builder, Incoming, NodeId, Placed, Point, compact_points, label::capture_rise, skewer_x,
};

/// Height of the collector every terminal branch enters, measured up from the
/// top edge of End.
const COLLECTOR_GAP: i32 = 32;

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
        let end = self.add_authored_node(index, skewer, placed.bottom + self.vertical_gap);
        self.connect_terminals(end);

        Placed {
            bottom: self.bottom_anchor(end).y,
            arrivals: Vec::new(),
        }
    }

    /// Draws every terminal into one horizontal collector above End, mirroring
    /// the distributor that fans a Select out to its cases.
    /// Each branch drops vertically onto the collector and the collector makes
    /// the single descent into the node, so the runs terminals share are one
    /// path rather than connections hidden behind each other.
    fn connect_terminals(&mut self, end: NodeId) {
        let end_top = self.top_anchor(end);
        // End's capture label stacks above it inside this same gap. The
        // collector carries the other branches, so it clears that label
        // instead of sitting at a fixed offset: labels are drawn after paths
        // and their halo would erase the row.
        let collector_y = end_top.y - COLLECTOR_GAP.max(capture_rise(&self.capture(end)));
        for terminal in std::mem::take(&mut self.terminals) {
            let start = self.anchor(terminal.origin);
            let lane_x = skewer_x(terminal.skewer);
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
                        y: collector_y,
                    },
                    Point {
                        x: end_top.x,
                        y: collector_y,
                    },
                    end_top,
                ])
            };
            self.connect(terminal, end, points);
        }
    }
}
