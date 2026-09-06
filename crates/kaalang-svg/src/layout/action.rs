//! Places an action on its single continuing skewer.

use kaalang_model::ExecutionPlan;

use super::{Builder, Incoming, NODE_LABEL_WIDTH, NODE_WIDTH, Origin, Placed, block_dimensions};

pub(super) fn dimensions(label: &str) -> (i32, i32, Vec<String>) {
    block_dimensions(label, NODE_WIDTH, NODE_LABEL_WIDTH, 64)
}

impl Builder<'_> {
    pub(super) fn place_action(
        &mut self,
        index: usize,
        next: &ExecutionPlan,
        skewer: usize,
        top: i32,
        incoming: Incoming,
    ) -> Placed {
        let node = self.add_block_node(index, skewer, top);
        self.connect_to_node(incoming, node);
        self.place(
            next,
            skewer,
            self.bottom_anchor(node).y + self.vertical_gap,
            Incoming {
                origin: Origin::bottom(node),
                branch: None,
                skewer,
            },
        )
    }
}
