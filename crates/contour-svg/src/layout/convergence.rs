//! Routes implicit branch yields into one shared continuation.

use contour_model::{Convergence, Plan};

use super::{Builder, Incoming, NodeId, Placed, Point, VERTICAL_GAP, compact_points};

impl Builder<'_> {
    pub(super) fn place_convergence(
        &mut self,
        mut arrivals: Vec<Incoming>,
        convergence: &Convergence,
        bottom: i32,
    ) -> Placed {
        // A convergence nested inside another one yields its value onward
        // instead of reaching an authored block, so its branches keep looking
        // for the shared consumer the outer convergence places.
        let Some(root) = plan_root(&convergence.next) else {
            return Placed { bottom, arrivals };
        };

        debug_assert!(
            !arrivals.is_empty(),
            "an implicit convergence has continuing branches"
        );
        let first = arrivals.remove(0);
        let continuation = self.place(
            &convergence.next,
            first.skewer,
            bottom + VERTICAL_GAP,
            first,
        );
        let join_y = i32::midpoint(bottom, self.top_anchor(root).y);
        for arrival in arrivals {
            self.connect_to_convergence(arrival, root, join_y);
        }
        continuation
    }

    /// Routes a sibling below every branch node before it enters the shared
    /// consumer, so a shorter path cannot cut through a deeper one.
    fn connect_to_convergence(&mut self, arrival: Incoming, root: NodeId, join_y: i32) {
        let start = self.anchor(arrival.origin);
        let end = self.top_anchor(root);
        let points = compact_points([
            start,
            Point {
                x: start.x,
                y: join_y,
            },
            Point {
                x: end.x,
                y: join_y,
            },
            end,
        ]);
        let label_at = arrival.label.as_ref().map(|_| Point {
            x: start.x + 42,
            y: start.y + 18,
        });
        self.connect_points(arrival.origin.node, root, arrival.label, points, label_at);
    }
}

/// The authored block a convergence continuation starts at, or `None` when it
/// only yields its value to an enclosing convergence.
fn plan_root(plan: &Plan) -> Option<NodeId> {
    match plan {
        Plan::End { body, .. } => plan_root(body),
        Plan::Action { index, .. } | Plan::Question { index, .. } | Plan::Choice { index, .. } => {
            Some(NodeId::Block(*index))
        }
        Plan::Yield { .. } => None,
        Plan::EndArrival { .. } => {
            unreachable!("a convergence continuation is a shared consumer or a yield")
        }
    }
}
