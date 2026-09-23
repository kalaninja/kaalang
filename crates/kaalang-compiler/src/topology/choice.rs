//! Projects a choice's select, cases, and distributor connections.

use crate::model::{Block, Execution};

use super::{
    Connection, Destination, Exit, ExitId, Node, NodeId, NodeKind, Source, block_node, branch_exits,
};

pub(super) fn project(index: usize, block: &Block, nodes: &mut Vec<Node>, exits: &mut Vec<Exit>) {
    // The cases and the outputs are walked separately below, so the pairing the
    // parser guarantees is stated here: a case with no output would be drawn
    // without an exit, and an output with no case would be addressed at a node
    // that was never projected.
    debug_assert_eq!(
        block.case_descriptions.len(),
        block.outputs.len(),
        "a choice declares one output per case"
    );
    nodes.push(block_node(index, NodeKind::Select));
    // A case represents one output and captures nothing of its own.
    nodes.extend((0..block.case_descriptions.len()).map(|branch| Node {
        id: case(index, branch),
        kind: NodeKind::Case,
    }));
    // The distributor provides nothing: each branch output is handed over at
    // its own case exit.
    exits.push(Exit {
        id: distributor(index),
        provides: Vec::new(),
    });
    exits.extend(branch_exits(index, block, exit));
}

/// The node one case is drawn as. A choice gives each case a node of its own, so
/// the branch is named in the node and its exit takes no branch.
const fn case(index: usize, branch: usize) -> NodeId {
    NodeId::Case {
        choice: index,
        branch,
    }
}

/// One case's own exit, which hands over the output that case provides.
pub(super) const fn exit(index: usize, branch: usize) -> ExitId {
    ExitId::of(case(index, branch))
}

/// The select's own exit, which every connection to a case leaves by.
const fn distributor(index: usize) -> ExitId {
    ExitId::of(NodeId::Block(index))
}

pub(super) fn connection(index: usize, execution: &Execution) -> Connection {
    let branch = execution
        .selected(index)
        .expect("an executed choice selects one case");
    Connection {
        source: Source::Exit(distributor(index)),
        destination: Destination::Node(case(index, branch)),
    }
}
