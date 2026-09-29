use std::collections::BTreeSet;

use kaalang_compiler::topology::{ExitId, NodeId};
use kaalang_compiler::{Analysis, ProducerId, SemanticModel};

use crate::text::RichText;

use super::{Captions, derive_with_parameters, named_parameters, provided, shares_label};

/// Reuses ordinary captions, then resolves stage labels from their destinations.
pub(crate) fn derive_stage(
    model: &SemanticModel,
    root: &Analysis,
    part: Option<usize>,
    start: &str,
    return_type: &str,
) -> Captions {
    let mut parameters = named_parameters(root);
    if let Some(index) = part {
        parameters = model
            .analysis
            .flow
            .flow_inputs
            .iter()
            .map(|wire| {
                if wire == &root.stages[index].entry {
                    wire.to_string()
                } else {
                    provided(
                        root,
                        &parameters,
                        root.flow
                            .producer(wire)
                            .expect("a stage's outer wire has a preparation producer"),
                    )
                }
            })
            .collect();
    }
    let mut captions = derive_with_parameters(model, start, return_type, &parameters);
    if let Some(index) = part {
        captions.label.insert(
            NodeId::Start,
            RichText::markdown(&root.stages[index].description),
        );
        captions
            .handover
            .insert(ExitId::of(NodeId::Start), Vec::new());
        if root.stages.iter().enumerate().any(|(from, stage)| {
            stage
                .analysis
                .flow
                .blocks
                .iter()
                .any(|block| block.transition_target == Some(index) && index <= from)
        }) {
            captions.back_marker.insert(NodeId::Start);
        }
    }
    for (block, statement) in model.analysis.flow.blocks.iter().enumerate() {
        let Some(target) = statement.transition_target else {
            continue;
        };
        let node = NodeId::Block(block);
        captions
            .label
            .insert(node, RichText::markdown(&root.stages[target].description));
        captions.capture.insert(node, Vec::new());
        captions.capture_label.insert(node, Vec::new());
        if part.is_some_and(|from| target <= from) {
            captions.back_marker.insert(node);
        }
    }
    let flow = &model.analysis.flow;
    let signals = flow
        .blocks
        .iter()
        .filter(|block| block.transition_target.is_some())
        .flat_map(|block| &block.inputs)
        .filter(|input| !input.derived)
        .map(|input| &input.ident)
        .collect::<BTreeSet<_>>();
    let is_signal = |producer: ProducerId| {
        signals.contains(match producer {
            ProducerId::FlowInput(input) => &flow.flow_inputs[input],
            ProducerId::BlockOutput { block, output } => &flow.blocks[block].outputs[output],
        })
    };
    for exit in &model.topology.exits {
        let mut producers = exit.provides.iter();
        captions
            .handover
            .get_mut(&exit.id)
            .expect("each exit has a hand-over caption")
            .retain(|_| !is_signal(*producers.next().expect("each hand-over names a producer")));
    }
    for (junction, names) in model.topology.junctions.iter().zip(&mut captions.junction) {
        let mut merges = junction.merges.iter();
        names.retain(|_| {
            let merge = *merges.next().expect("each junction label names a merge");
            !is_signal(model.analysis.merges[merge].producers[0])
        });
    }
    captions.shared = model
        .topology
        .connections
        .iter()
        .filter(|connection| shares_label(&model.topology, &captions, connection))
        .copied()
        .collect();
    captions
}
