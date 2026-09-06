//! Places a question and its two ordered branches.

use kaalang_model::{Branch, Join};

use super::{Builder, Incoming, Origin, Placed, branch_layout};

impl Builder<'_> {
    pub(super) fn place_question(
        &mut self,
        index: usize,
        branches: &[Branch; 2],
        convergence: Option<&Join>,
        skewer: usize,
        top: i32,
        incoming: Incoming,
    ) -> Placed {
        let node = self.add_block_node(index, skewer, top);
        self.connect_to_node(incoming, node);
        let branch_top = self.bottom_anchor(node).y + self.vertical_gap;
        let (offsets, _) = branch_layout(index, branches, convergence);
        let mut placed = Vec::with_capacity(branches.len());
        for (branch_index, branch) in branches.iter().enumerate() {
            let branch_skewer = skewer + offsets[branch_index];
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
                    branch: Some(branch_index),
                    skewer: branch_skewer,
                },
            ));
        }

        self.finish_branches(index, placed, convergence)
    }
}
