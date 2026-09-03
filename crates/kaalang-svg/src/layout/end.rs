//! Places the one authored End below every path that reaches it.

use std::cmp::Reverse;

use kaalang_model::Plan;

use super::{
    Builder, Incoming, NODE_WIDTH, NodeId, Placed, Point, compact_points, label::capture_rise,
    skewer_x,
};

const TERMINAL_STUB: i32 = 24;
/// Height of the collector every terminal branch enters, measured up from the
/// top edge of End.
const COLLECTOR_GAP: i32 = 32;
const END_LANE_GAP: i32 = 52;

pub(super) fn dimensions() -> (i32, i32, Vec<String>) {
    (180, 58, Vec::new())
}

impl Builder<'_> {
    pub(super) fn place_end(
        &mut self,
        index: usize,
        body: &Plan,
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

    /// Reports whether a terminal can drop straight down its own column, which
    /// it cannot when a node or an earlier connection stands in the way.
    fn terminal_is_clear(&self, terminal: &Incoming, end: NodeId, join_y: i32) -> bool {
        let start = self.anchor(terminal.origin);
        let end_top = self.top_anchor(end);
        let blocked_by_node = self.scene.nodes.iter().any(|node| {
            // A descent on the node's own border is hidden by it: both carry
            // the same stroke and nodes are drawn last, so the span counts as
            // blocked up to and including its edges.
            let top = node.y - node.height / 2;
            node.id != terminal.origin.node
                && node.id != end
                && (start.x - node.x).abs() <= node.width / 2
                && top > start.y
                && top < end_top.y
        });
        // Every lane is chosen before the first terminal connection is drawn,
        // so the scene holds no terminal edges yet and none obstructs another.
        let blocked_by_edge = self.scene.edges.iter().any(|edge| {
            edge.points
                .windows(2)
                .any(|segment| vertical_route_hits(segment, start.x, start.y, join_y))
        });

        !blocked_by_node && !blocked_by_edge
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
        let end_lane = self.end_lane_x();
        let terminals = std::mem::take(&mut self.terminals);

        // A terminal whose own column is blocked takes an outer lane of its
        // own, or two of them trace the same one and hide each other on the
        // way down. The deepest start takes the innermost lane: a detour
        // leaves on a stepped row below its own start, so ordering the lanes
        // by depth keeps every row above the lanes it crosses on the way out.
        let mut blocked = terminals
            .iter()
            .enumerate()
            .filter(|(_, terminal)| !self.terminal_is_clear(terminal, end, collector_y))
            .map(|(index, terminal)| (self.anchor(terminal.origin).y, index))
            .collect::<Vec<_>>();
        blocked.sort_unstable_by_key(|(depth, _)| Reverse(*depth));
        let detours = blocked.len() as i32;
        // The lane each terminal drops down, counted out from the first one
        // clear of every skewer, or its own column when nothing blocks it.
        let mut lanes = vec![None; terminals.len()];
        for (lane, (_, index)) in blocked.into_iter().enumerate() {
            lanes[index] = Some(lane as i32);
        }

        for (terminal, lane) in terminals.into_iter().zip(lanes) {
            let start = self.anchor(terminal.origin);
            let points = match lane {
                // The collector is degenerate for a branch already above the
                // node, exactly as the distributor is for a case below a Select.
                None if start.x == end_top.x => vec![start, end_top],
                None => compact_points([
                    start,
                    Point {
                        x: start.x,
                        y: collector_y,
                    },
                    Point {
                        x: end_top.x,
                        y: collector_y,
                    },
                    end_top,
                ]),
                Some(lane) => {
                    let lane_x = end_lane + lane * END_LANE_GAP;
                    // Detours leave on stepped rows, so an inner one crosses
                    // under the next instead of running along it out to the
                    // lanes.
                    // ponytail: a row still crosses an ordinary connection in
                    // the columns it traverses, which no choice of row can
                    // avoid while a blocked column stays occupied all the way
                    // down; route detours around obstacles if that reads badly.
                    let stub_y = start.y + TERMINAL_STUB * (detours - lane);
                    compact_points([
                        start,
                        Point {
                            x: start.x,
                            y: stub_y,
                        },
                        Point {
                            x: lane_x,
                            y: stub_y,
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
                }
            };
            self.connect(terminal, end, points);
        }
    }

    fn end_lane_x(&self) -> i32 {
        skewer_x(self.skewer_count - 1) + NODE_WIDTH / 2 + END_LANE_GAP
    }
}

fn vertical_route_hits(segment: &[Point], x: i32, top: i32, bottom: i32) -> bool {
    let [from, to] = segment else {
        return false;
    };
    if from.x == to.x {
        from.x == x && from.y.max(to.y) > top && from.y.min(to.y) <= bottom
    } else {
        debug_assert_eq!(from.y, to.y);
        from.y > top && from.y <= bottom && from.x.min(to.x) <= x && x <= from.x.max(to.x)
    }
}
