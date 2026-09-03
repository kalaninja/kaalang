//! Places the one authored End below every path that reaches it.

use kaalang_model::Plan;

use super::{Builder, Incoming, Placed};

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
}
