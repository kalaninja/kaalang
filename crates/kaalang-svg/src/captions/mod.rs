//! Derives captions and shared-label decisions from the authored flow.
//! Exits own hand-overs; nodes own captures.

use std::collections::{BTreeMap, BTreeSet};

use kaalang_compiler::{
    Analysis, BlockKind, Input, ProducerId, SemanticModel, is_unnamed,
    topology::{Connection, Destination, ExitId, Node, NodeId, NodeKind, Source, Topology},
};
use syn::{Expr, FnArg, Pat, PatIdent, ext::IdentExt};

pub(crate) use self::stage::derive_stage;
use crate::text::RichText;

mod stage;

/// Every string the diagram shows, keyed by the structural item that owns it.
#[derive(Clone, Default)]
pub(crate) struct Captions {
    label: BTreeMap<NodeId, RichText>,
    capture: BTreeMap<NodeId, Vec<String>>,
    capture_label: BTreeMap<NodeId, Vec<String>>,
    handover: BTreeMap<ExitId, Vec<String>>,
    branch_description: BTreeMap<ExitId, RichText>,
    cycle_inputs: BTreeMap<usize, String>,
    /// Per cycle, each declared output's binding label and bare name.
    cycle_outputs: BTreeMap<usize, Vec<(String, String)>>,
    /// Borrowed for a node that has no caption of its own.
    empty: RichText,
    /// Per junction, the merged wire names, empty for a structural junction.
    junction: Vec<Vec<String>>,
    shared: BTreeSet<Connection>,
    back_marker: BTreeSet<NodeId>,
}

impl Captions {
    /// The node's own caption: its block description, the flow header at start,
    /// or the return type at end.
    pub(crate) fn label(&self, node: NodeId) -> &RichText {
        self.label.get(&node).unwrap_or(&self.empty)
    }

    /// The wires this node captures, in authored order, with capture modifiers.
    pub(crate) fn capture(&self, node: NodeId) -> &[String] {
        self.capture.get(&node).map_or(&[], Vec::as_slice)
    }

    /// The displayed capture list. `()` marks an empty input on a connected
    /// computational node; it is neither a wire name nor a synthetic capture.
    pub(crate) fn capture_label(&self, node: NodeId) -> &[String] {
        self.capture_label.get(&node).map_or(&[], Vec::as_slice)
    }

    /// Derived cycle inputs may share a hand-over regardless of order.
    pub(crate) fn matches_capture(&self, node: &Node, handover: &[String]) -> bool {
        let capture = self.capture(node.id);
        capture == handover
            || (node.kind == NodeKind::Cycle
                && capture.len() == handover.len()
                && capture.iter().collect::<BTreeSet<_>>() == handover.iter().collect())
    }

    /// The wires newly provided at this exit, labeled whether or not any
    /// connection leaves.
    pub(crate) fn handover(&self, exit: ExitId) -> &[String] {
        self.handover.get(&exit).map_or(&[], Vec::as_slice)
    }

    /// The authored description of a question branch, which replaces that
    /// branch's output hand-over label.
    pub(crate) fn branch_description(&self, exit: ExitId) -> Option<&RichText> {
        self.branch_description.get(&exit)
    }

    pub(crate) fn cycle_inputs(&self, block: usize) -> &str {
        self.cycle_inputs.get(&block).map_or("()", String::as_str)
    }

    /// A cycle's declared outputs, `()` for none. Several are alternatives:
    /// one leaves per completion.
    pub(crate) fn cycle_outputs(&self, block: usize) -> String {
        match self.cycle_outputs.get(&block).map(Vec::as_slice) {
            None | Some([]) => "()".to_owned(),
            Some(outputs) => outputs
                .iter()
                .map(|(label, _)| label.as_str())
                .collect::<Vec<_>>()
                .join(" or "),
        }
    }

    /// The name of one declared output of a cycle, without its `mut`.
    pub(crate) fn cycle_output(&self, block: usize, output: usize) -> &str {
        &self.cycle_outputs[&block][output].1
    }

    /// The wires that meet at one junction, in model order.
    pub(crate) fn junction_wires(&self, junction: usize) -> &[String] {
        self.junction.get(junction).map_or(&[], Vec::as_slice)
    }

    /// Whether the two ends of one connection are drawn as a single label.
    pub(crate) fn shares_label(&self, connection: &Connection) -> bool {
        self.shared.contains(connection)
    }

    pub(crate) fn back_marker(&self, node: NodeId) -> bool {
        self.back_marker.contains(&node)
    }
}

/// Reads every caption of one flow's topology. `start` labels the start node and
/// `return_type` captions end, both taken from the authored source text.
pub(crate) fn derive(model: &SemanticModel, start: &str, return_type: &str) -> Captions {
    derive_with_parameters(
        model,
        start,
        return_type,
        &named_parameters(&model.analysis),
    )
}

#[allow(clippy::too_many_lines)] // One cohesive pass derives every displayed caption.
fn derive_with_parameters(
    model: &SemanticModel,
    start: &str,
    return_type: &str,
    parameters: &[String],
) -> Captions {
    let topology = &model.topology;
    let end_input = end_input(model);
    let mut captions = Captions::default();

    for node in &topology.nodes {
        let label = match node.id {
            NodeId::Start => RichText::literal(start),
            NodeId::Block(_) if node.kind == NodeKind::End => RichText::literal(return_type),
            // Undescribed calls use the callee path.
            NodeId::Block(block) => match &model.analysis.flow.blocks[block].description {
                Some(text) => RichText::markdown(text),
                None if node.kind == NodeKind::Call => {
                    RichText::literal(model.analysis.flow.blocks[block].callee())
                }
                None => RichText::default(),
            },
            NodeId::Case { choice, branch } => {
                RichText::markdown(&model.analysis.flow.blocks[choice].case_descriptions[branch])
            }
        };
        captions.label.insert(node.id, label);
        // A case represents one choice output and captures nothing of its own.
        // End shows the transferred value, not every wire the structural
        // return captures for ordering.
        let capture = match node.id {
            NodeId::Block(_) if node.kind == NodeKind::End => end_input.clone(),
            NodeId::Block(block) if node.kind == NodeKind::Cycle => {
                cycle_input_names(model, parameters, block)
            }
            NodeId::Block(block) => model.analysis.flow.blocks[block]
                .inputs
                .iter()
                .map(captured)
                .collect(),
            NodeId::Start | NodeId::Case { .. } => Vec::new(),
        };
        let displayed = if capture.is_empty()
            && !matches!(node.kind, NodeKind::Start | NodeKind::End | NodeKind::Case)
            && topology
                .incoming(Destination::Node(node.id))
                .next()
                .is_some()
        {
            vec!["()".to_owned()]
        } else {
            capture.clone()
        };
        captions.capture.insert(node.id, capture);
        captions.capture_label.insert(node.id, displayed);
    }

    for boundary in &topology.cycle_boundaries {
        let block = &model.analysis.flow.blocks[boundary.header];
        captions.label.insert(
            NodeId::Block(boundary.header),
            block
                .description
                .as_deref()
                .map_or_else(RichText::default, RichText::markdown),
        );
        let inputs = cycle_input_names(model, parameters, boundary.header);
        captions.cycle_inputs.insert(
            boundary.header,
            if inputs.is_empty() {
                "()".to_owned()
            } else {
                inputs.join(", ")
            },
        );
    }
    // Both views name a cycle's outputs: the expanded results and the
    // collapsed node's exits.
    for (header, block) in model.analysis.flow.blocks.iter().enumerate() {
        if block.cycle_end.is_some() {
            captions.cycle_outputs.insert(
                header,
                (0..block.outputs.len())
                    .map(|output| block.output_binding(output))
                    .filter(|binding| !is_unnamed(&binding.ident))
                    .map(|binding| (binding_label(binding), binding.ident.unraw().to_string()))
                    .collect(),
            );
        }
    }

    for exit in &topology.exits {
        captions.handover.insert(
            exit.id,
            exit.provides
                .iter()
                .filter(|&&producer| match producer {
                    ProducerId::BlockOutput { block, output } => !is_unnamed(
                        &model.analysis.flow.blocks[block]
                            .output_binding(output)
                            .ident,
                    ),
                    ProducerId::FlowInput(_) => true,
                })
                .map(|&producer| provided(&model.analysis, parameters, producer))
                .collect(),
        );
        if let (NodeId::Block(block), Some(branch)) = (exit.id.node, exit.id.branch)
            && let Some(description) = model.analysis.flow.blocks[block]
                .question_branches
                .get(branch)
                .and_then(|answer| answer.description.clone())
        {
            captions
                .branch_description
                .insert(exit.id, RichText::markdown(&description));
        }
    }

    captions.junction = topology
        .junctions
        .iter()
        .map(|junction| {
            junction
                .merges
                .iter()
                .map(|&merge| {
                    provided(
                        &model.analysis,
                        parameters,
                        model.analysis.merges[merge].producers[0],
                    )
                })
                .collect()
        })
        .collect();

    captions.shared = topology
        .connections
        .iter()
        .filter(|connection| shares_label(topology, &captions, connection))
        .copied()
        .collect();

    captions
}

/// The value transferred into end.
fn end_input(model: &SemanticModel) -> Vec<String> {
    model
        .analysis
        .flow
        .blocks
        .iter()
        .find(|block| block.kind == BlockKind::Return)
        .map_or_else(Vec::new, |block| transferred(&block.body))
}

/// Return operands label their wires individually, so an adjacent hand-over can
/// share the same ordered list. A tuple-valued wire still contributes one name.
fn transferred(value: &Expr) -> Vec<String> {
    match value {
        Expr::Group(group) => transferred(&group.expr),
        Expr::Paren(parenthesized) => transferred(&parenthesized.expr),
        Expr::Path(path) => vec![
            path.path
                .get_ident()
                .expect("a return value is one captured input")
                .unraw()
                .to_string(),
        ],
        Expr::Tuple(tuple) if tuple.elems.is_empty() => vec!["()".to_owned()],
        Expr::Tuple(tuple) => tuple.elems.iter().flat_map(transferred).collect(),
        _ => unreachable!("a return value is validated before captions are derived"),
    }
}

/// Shares equal, nonempty lists only on a connection unique at both ends.
/// Modifiers and empty-input markers prevent a match with bare wire names.
fn shares_label(topology: &Topology, captions: &Captions, connection: &Connection) -> bool {
    let (Source::Exit(exit), Destination::Node(node)) = (connection.source, connection.destination)
    else {
        return false;
    };
    !captions.handover(exit).is_empty()
        // The body's hand-over and the cycle's output are separate transfers,
        // even when a body exit supplies the cycle's result directly.
        && !topology
            .cycle_boundaries
            .iter()
            .any(|boundary| boundary.results.contains(&connection.source))
        && captions.matches_capture(topology.node(node), captions.handover(exit))
        && topology.leaving(exit).count() == 1
        && topology.single_arrival(node)
}

/// The named flow parameters, in flow-input order, as the start node's
/// hand-over addresses them. A receiver is one of them, under the one name
/// Rust gives it.
fn named_parameters(analysis: &Analysis) -> Vec<String> {
    analysis
        .parameters
        .iter()
        .filter_map(|parameter| match parameter {
            FnArg::Receiver(_) => Some("self".to_owned()),
            FnArg::Typed(parameter) => match parameter.pat.as_ref() {
                Pat::Ident(binding) => Some(binding_label(binding)),
                _ => None,
            },
        })
        .collect()
}

/// One binding as a caption: its authored spelling, with a permitted `mut`.
fn binding_label(binding: &PatIdent) -> String {
    let mutable = if binding.mutability.is_some() {
        "mut "
    } else {
        ""
    };
    format!("{mutable}{}", binding.ident.unraw())
}

/// The label one producer occurrence carries at the exit providing it.
fn provided(analysis: &Analysis, parameters: &[String], producer: ProducerId) -> String {
    match producer {
        ProducerId::FlowInput(input) => parameters
            .get(input)
            .expect("a flow input caption names a declared parameter")
            .clone(),
        ProducerId::BlockOutput { block, output } => {
            binding_label(analysis.flow.blocks[block].output_binding(output))
        }
    }
}

/// A cycle's gate and the outer wires its body captures, each with its
/// producer's `mut` rather than a capture form.
fn cycle_input_names(model: &SemanticModel, parameters: &[String], block: usize) -> Vec<String> {
    let flow = &model.analysis.flow;
    flow.blocks[block]
        .inputs
        .iter()
        .map(|input| {
            let producer = flow
                .producer(&input.ident)
                .expect("a cycle input names a produced wire");
            provided(&model.analysis, parameters, producer)
        })
        .collect()
}

/// Capture modifiers distinguish the four authored input forms.
fn captured(input: &Input) -> String {
    let borrow = if input.borrowed { "&" } else { "" };
    let mutable = if input.mutable { "mut " } else { "" };
    format!("{borrow}{mutable}{}", input.alias.unraw())
}

#[cfg(test)]
mod tests;
