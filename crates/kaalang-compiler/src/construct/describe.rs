//! Names the parts of a topology the way the author wrote them, so a
//! diagnostic points at a block description or a cycle rather than at an
//! internal identifier.

use proc_macro2::Span;

use crate::{
    model::{BlockKind, Flow, FlowKind, WireMerge},
    topology::{ExitId, NodeId, Source, Topology, Vertex},
};

/// The span to report one vertex at: the block it draws, or the cycle whose
/// entry or tail it is.
pub(super) fn vertex_span(flow: &Flow, topology: &Topology, vertex: Vertex) -> Span {
    match vertex {
        Vertex::Node(NodeId::Block(block) | NodeId::Case { choice: block, .. }) => {
            flow.blocks[block].span
        }
        Vertex::Junction(junction) => topology
            .cycles
            .iter()
            .find(|cycle| cycle.tail == junction || cycle.entry == junction)
            .map_or_else(|| flow.end_span(), |cycle| flow.blocks[cycle.header].span),
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
        Vertex::Node(NodeId::Start) => match &flow.kind {
            FlowKind::Stage { entry, .. } => format!("the stage entry `{}`", flow.wire_name(entry)),
            FlowKind::Plain | FlowKind::Preparation => "the start of the flow".to_owned(),
        },
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
pub(super) fn cycle_name(flow: &Flow, header: usize) -> String {
    flow.blocks[header].description.as_deref().map_or_else(
        || "this cycle".to_owned(),
        |text| format!("the cycle `{text}`"),
    )
}

fn block_name(flow: &Flow, block: usize) -> String {
    let declaration = &flow.blocks[block];
    // A for cycle's hidden choice and continue draw as its for-entry and for-end.
    if let Some(header) = declaration
        .parent
        .filter(|_| flow.takes_next_item(block) || flow.ends_iteration(block))
    {
        let part = if flow.takes_next_item(block) {
            "for-entry"
        } else {
            "for-end"
        };
        return flow.blocks[header].description.as_deref().map_or_else(
            || format!("the {part} of this cycle"),
            |text| format!("the {part} `{text}`"),
        );
    }
    if declaration.transition_target.is_some() {
        return format!(
            "the stage transition `{}`",
            flow.wire_name(&declaration.inputs[0].ident)
        );
    }
    match (&declaration.description, declaration.kind) {
        (Some(text), _) => format!("`{text}`"),
        (None, BlockKind::Cycle) => cycle_name(flow, block),
        (None, BlockKind::Call) => format!("the call `{}`", flow.blocks[block].callee()),
        (None, BlockKind::Export) => {
            let header = declaration
                .export_target
                .expect("a boundary consumer has a cycle");
            let output = &flow.blocks[header]
                .output_binding(flow.exported_output(block))
                .ident;
            format!("the export of `{output}`")
        }
        (None, BlockKind::Continue) => "a continue".to_owned(),
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
    if let Some(cycle) = topology
        .cycles
        .iter()
        .find(|cycle| cycle.tail == junction || cycle.entry == junction)
    {
        let part = if cycle.tail == junction {
            "the iteration tail"
        } else {
            "the entry"
        };
        return format!("{part} of {}", cycle_name(flow, cycle.header));
    }
    // Several iteration endings of a for cycle meet here before its for-end.
    if let Some(header) = topology
        .outgoing(Vertex::Junction(junction))
        .find_map(|edge| match edge.destination {
            Vertex::Node(NodeId::Block(block)) if flow.ends_iteration(block) => {
                flow.blocks[block].parent
            }
            _ => None,
        })
    {
        return format!("the iteration tail of {}", cycle_name(flow, header));
    }
    let junction = &topology.junctions[junction];
    if junction.is_cycle_result {
        return "a cycle result".to_owned();
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
                let result = |value| loop {
                    #[question("Keep the value?")]
                    let (keep, zero) = |value| value > 1;
                    #[action("Keep it.")]
                    let result = |keep, value| value;
                    #[action("Use zero.")]
                    let result = |zero| 0;
                };
                |result| return result;
            }
        "#;
        let model = crate::tests::model(source);
        let name = |predicate: fn(&crate::topology::Junction) -> bool| {
            let index = model.topology.junctions.iter().position(predicate).unwrap();
            junction_name(
                &model.analysis.flow,
                &model.analysis.merges,
                &model.topology,
                index,
            )
        };
        assert_eq!(name(|junction| junction.is_cycle_result), "a cycle result");

        let mut topology = model.topology.clone();
        topology
            .junctions
            .push(crate::topology::Junction::default());
        assert_eq!(
            junction_name(
                &model.analysis.flow,
                &model.analysis.merges,
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
        let model = crate::tests::model(source);
        assert_eq!(
            block_name(&model.analysis.flow, 0),
            "the call `math::halve`"
        );
        assert_eq!(
            block_name(&model.analysis.flow, 1),
            "the call `math::square`"
        );
    }
}
