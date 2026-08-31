//! Places a choice, its derived case row, and its ordered branches.

use contour_model::{Branch, Merge};

use super::{
    Builder, CASE_LABEL_WIDTH, CASE_TIP_HEIGHT, CASE_WIDTH, Incoming, LABEL_FONT, LINE_HEIGHT,
    NodeId, NodeKind, Origin, Placed, Point, VERTICAL_GAP, compact_points, plan_span, wrap_text,
};

pub(super) fn case_dimensions(label: &str) -> (i32, i32, Vec<String>) {
    let lines = wrap_text(label, CASE_LABEL_WIDTH, LABEL_FONT);
    let body_height = 52.max(28 + lines.len() as i32 * LINE_HEIGHT);
    (CASE_WIDTH, body_height + CASE_TIP_HEIGHT, lines)
}

impl Builder<'_> {
    pub(super) fn place_choice(
        &mut self,
        index: usize,
        branches: &[Branch],
        merge: Option<&Merge>,
        skewer: usize,
        top: i32,
        incoming: Incoming,
    ) -> Placed {
        let select = self.add_authored_node(index, skewer, top);
        self.connect_to_node(incoming, select);
        let block = &self.graph.flow.blocks[index];
        let spans = branches
            .iter()
            .map(|branch| plan_span(&branch.plan))
            .collect::<Vec<_>>();
        let mut branch_skewer = skewer;
        let mut cases = Vec::with_capacity(branches.len());
        for (branch_index, description) in block.case_descriptions.iter().enumerate() {
            let case = self.add_node(
                NodeId::Case {
                    choice: index,
                    branch: branch_index,
                },
                NodeKind::Case,
                description.clone(),
                branch_skewer,
                self.node_bottom(select) + VERTICAL_GAP,
            );
            cases.push((case, branch_skewer));
            branch_skewer += spans[branch_index];
        }
        self.align_case_row(&cases);

        let case_top = self.node_top(cases[0].0);
        for (case, _) in &cases {
            self.connect_points(
                select,
                *case,
                None,
                self.distributor_points(select, *case, case_top),
                None,
            );
        }

        let branch_top = cases
            .iter()
            .map(|(case, _)| self.node_bottom(*case))
            .max()
            .unwrap_or(self.node_bottom(select))
            + VERTICAL_GAP;
        let mut placed = Vec::with_capacity(branches.len());
        for (branch_index, branch) in branches.iter().enumerate() {
            let (case, branch_skewer) = cases[branch_index];
            placed.push(self.place(
                &branch.plan,
                branch_skewer,
                branch_top,
                Incoming {
                    origin: Origin::bottom(case),
                    label: Some(block.outputs[branch_index].to_string()),
                    skewer: branch_skewer,
                },
            ));
        }

        self.finish_branches(placed, merge, skewer)
    }

    fn align_case_row(&mut self, cases: &[(NodeId, usize)]) {
        let height = cases
            .iter()
            .map(|(case, _)| self.node(*case).height)
            .max()
            .unwrap_or(0);
        for (case, _) in cases {
            let top = self.node_top(*case);
            let index = self.indexes[case];
            self.scene.nodes[index].height = height;
            self.scene.nodes[index].y = top + height / 2;
        }
    }

    fn distributor_points(&self, from: NodeId, to: NodeId, case_top: i32) -> Vec<Point> {
        let start = self.bottom_anchor(from);
        let end = self.top_anchor(to);
        if start.x == end.x {
            return vec![start, end];
        }
        let distributor_y = i32::midpoint(start.y, case_top);
        compact_points([
            start,
            Point {
                x: start.x,
                y: distributor_y,
            },
            Point {
                x: end.x,
                y: distributor_y,
            },
            end,
        ])
    }
}
