//! Names the parts of a topology the way the author wrote them, so a
//! diagnostic points at a block description or a loop rather than at an
//! internal identifier.

use proc_macro2::Span;

use crate::model::{BlockKind, Flow, WireMerge};
use crate::topology::{ExitId, NodeId, Source, Topology, Vertex};

/// The span to report one connection at: the block it leaves.
pub(super) fn span(flow: &Flow, topology: &Topology, connection: usize) -> Span {
    let wire = topology.connections[connection];
    match Vertex::from(wire.source) {
        Vertex::Node(NodeId::Block(block) | NodeId::Case { choice: block, .. }) => {
            flow.blocks[block].span
        }
        Vertex::Node(NodeId::Start) | Vertex::Junction(_) => flow.end_span(),
    }
}

/// One connection, named by the ends it joins.
pub(super) fn connection(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    connection: usize,
) -> String {
    let wire = topology.connections[connection];
    let branch = match wire.source {
        Source::Exit(ExitId {
            node: NodeId::Block(block),
            branch: Some(branch),
        }) => flow.blocks[block]
            .question_branches
            .get(branch)
            .and_then(|answer| answer.description.clone())
            .map_or_else(
                || format!(" answer {}", branch + 1),
                |text| format!(" `{text}`"),
            ),
        _ => String::new(),
    };
    format!(
        "the route from {from}{branch} to {to}",
        from = vertex(flow, merges, topology, Vertex::from(wire.source)),
        to = vertex(flow, merges, topology, wire.destination),
    )
}

/// One vertex, named by the block or the structure it draws.
pub(super) fn vertex(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    vertex: Vertex,
) -> String {
    match vertex {
        Vertex::Node(NodeId::Start) => "the start of the flow".to_owned(),
        Vertex::Node(NodeId::Block(block)) => block_name(flow, block),
        Vertex::Node(NodeId::Case { choice, branch }) => flow.blocks[choice]
            .case_descriptions
            .get(branch)
            .map_or_else(
                || format!("case {} of {}", branch + 1, block_name(flow, choice)),
                |text| format!("case `{text}`"),
            ),
        Vertex::Junction(junction) => junction_name(flow, merges, topology, junction),
    }
}

/// One loop, named by its authored label when it has one.
pub(super) fn loop_name(flow: &Flow, header: usize) -> String {
    match &flow.blocks[header].loop_label {
        Some(label) => format!("the loop `{}`", label.ident),
        None => "this loop".to_owned(),
    }
}

fn block_name(flow: &Flow, block: usize) -> String {
    let declaration = &flow.blocks[block];
    match (&declaration.description, declaration.kind) {
        (Some(text), _) => format!("`{text}`"),
        (None, BlockKind::Loop) => loop_name(flow, block),
        (None, BlockKind::Break) => "a break".to_owned(),
        (None, BlockKind::End) => "the end of the flow".to_owned(),
        (None, _) => format!("the block at position {}", block + 1),
    }
}

fn junction_name(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    junction: usize,
) -> String {
    if let Some(loop_) = topology
        .loops
        .iter()
        .find(|loop_| loop_.tail == junction || loop_.entry == junction)
    {
        let part = if loop_.tail == junction {
            "the iteration tail"
        } else {
            "the entry"
        };
        return format!("{part} of {}", loop_name(flow, loop_.header));
    }
    let wires = topology.junctions[junction]
        .merges
        .iter()
        .map(|&merge| flow.wire_name(&merges[merge].wire))
        .collect::<Vec<_>>();
    if wires.is_empty() {
        "a loop exit".to_owned()
    } else {
        format!("the `{}` merge", wires.join("` and `"))
    }
}
