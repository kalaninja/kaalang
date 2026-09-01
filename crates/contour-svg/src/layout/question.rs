//! Places a question and its two ordered branches.

use contour_model::{Branch, Convergence};

use super::{Builder, Incoming, Origin, Placed, VERTICAL_GAP, plan_span};

impl Builder<'_> {
    pub(super) fn place_question(
        &mut self,
        index: usize,
        branches: &[Branch; 2],
        convergence: Option<&Convergence>,
        skewer: usize,
        top: i32,
        incoming: Incoming,
    ) -> Placed {
        let node = self.add_authored_node(index, skewer, top);
        self.connect_to_node(incoming, node);
        let branch_top = self.node_bottom(node) + VERTICAL_GAP;
        let outputs = &self.graph.flow.blocks[index].outputs;
        let spans = branches
            .iter()
            .map(|branch| plan_span(&branch.plan))
            .collect::<Vec<_>>();
        let mut branch_skewer = skewer;
        let mut placed = Vec::with_capacity(branches.len());
        for (branch_index, branch) in branches.iter().enumerate() {
            let origin = if branch_index == 0 {
                Origin::bottom(node)
            } else {
                Origin::right(node)
            };
            placed.push(self.place(
                &branch.plan,
                branch_skewer,
                branch_top,
                Incoming {
                    origin,
                    label: Some(outputs[branch_index].to_string()),
                    skewer: branch_skewer,
                },
            ));
            branch_skewer += spans[branch_index];
        }

        self.finish_branches(placed, convergence)
    }
}
