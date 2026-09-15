//! Names the parts of a topology the way the author wrote them, so a
//! diagnostic points at a block description or a cycle rather than at an
//! internal identifier.

use proc_macro2::Span;

use crate::model::{BlockKind, Flow, WireMerge};
use crate::topology::{ExitId, NodeId, Source, Topology, Vertex};

/// The span to report one connection at: the block it leaves.
pub(super) fn span(flow: &Flow, topology: &Topology, connection: usize) -> Span {
    vertex_span(
        flow,
        topology,
        Vertex::from(topology.connections[connection].source),
    )
}

/// The span to report one vertex at: the block it draws, or the cycle whose
/// entry or tail it is.
pub(super) fn vertex_span(flow: &Flow, topology: &Topology, vertex: Vertex) -> Span {
    match vertex {
        Vertex::Node(NodeId::Block(block) | NodeId::Case { choice: block, .. }) => {
            flow.blocks[block].span
        }
        Vertex::Junction(junction) => topology
            .loops
            .iter()
            .find(|loop_| loop_.tail == junction || loop_.entry == junction)
            .map_or_else(|| flow.end_span(), |loop_| flow.blocks[loop_.header].span),
        Vertex::Node(NodeId::Start) => flow.end_span(),
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

/// One cycle, named by its authored description.
pub(super) fn loop_name(flow: &Flow, header: usize) -> String {
    flow.blocks[header].description.as_deref().map_or_else(
        || "this cycle".to_owned(),
        |text| format!("the cycle `{text}`"),
    )
}

/// One call, named by the function it runs, so two undescribed calls in one
/// flow are told apart and the name matches the diagram's own label.
fn call_name(flow: &Flow, block: usize) -> String {
    format!("the call `{}`", flow.blocks[block].callee())
}

fn block_name(flow: &Flow, block: usize) -> String {
    let declaration = &flow.blocks[block];
    match (&declaration.description, declaration.kind) {
        (Some(text), _) => format!("`{text}`"),
        (None, BlockKind::Loop) => loop_name(flow, block),
        (None, BlockKind::Call) => call_name(flow, block),
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
    let junction = &topology.junctions[junction];
    if junction.is_loop_result {
        return "a cycle result".to_owned();
    }
    if junction.is_break {
        return "a cycle break".to_owned();
    }
    let wires = junction
        .merges
        .iter()
        .map(|&merge| flow.wire_name(&merges[merge].wire))
        .collect::<Vec<_>>();
    if wires.is_empty() {
        "a structural junction".to_owned()
    } else {
        format!("the `{}` merge", wires.join("` and `"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_junctions_use_their_authored_roles() {
        let source = r#"
            fn example(value: u8) -> u8 {
                #[cycle("Choose the result.")]
                let result = |value| {
                    |value| break value;
                };
                |result| return result;
            }
        "#;
        let model = crate::build(&crate::tests::fixture(source, "example")).unwrap();
        let name = |predicate: fn(&crate::topology::Junction) -> bool| {
            let index = model.topology.junctions.iter().position(predicate).unwrap();
            junction_name(&model.flow, &model.merges, &model.topology, index)
        };
        assert_eq!(name(|junction| junction.is_break), "a cycle result");
        assert_eq!(name(|junction| junction.is_loop_result), "a cycle result");

        let mut topology = model.topology.clone();
        topology
            .junctions
            .push(crate::topology::Junction::default());
        assert_eq!(
            junction_name(
                &model.flow,
                &model.merges,
                &topology,
                topology.junctions.len() - 1,
            ),
            "a structural junction"
        );
    }

    #[test]
    fn undescribed_calls_are_told_apart_by_the_functions_they_run() {
        let source = r"
            fn example(value: u8) -> u8 {
                #[call]
                let halved = |value| math::halve(value);

                #[call]
                let end = |halved| math::square(halved);

                |end| return end;
            }
        ";
        let model = crate::build(&crate::tests::fixture(source, "example")).unwrap();
        assert_eq!(block_name(&model.flow, 0), "the call `math::halve`");
        assert_eq!(block_name(&model.flow, 1), "the call `math::square`");
    }
}
