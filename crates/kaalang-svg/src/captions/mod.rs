//! Derives captions and shared-label decisions from the authored flow.
//! Exits own hand-overs; nodes own captures (RFC 0002 §6).

use std::collections::{BTreeMap, BTreeSet};

use kaalang_compiler::topology::{
    Connection, Destination, ExitId, NodeId, NodeKind, Source, Topology,
};
use kaalang_compiler::{BlockKind, Input, ProducerId, SemanticModel};
use syn::{Expr, FnArg, Pat, PatIdent, ext::IdentExt};

/// Every string the diagram shows, keyed by the structural item that owns it.
#[derive(Clone, Default)]
pub(crate) struct Captions {
    label: BTreeMap<NodeId, String>,
    capture: BTreeMap<NodeId, Vec<String>>,
    capture_label: BTreeMap<NodeId, Vec<String>>,
    handover: BTreeMap<ExitId, Vec<String>>,
    branch_description: BTreeMap<ExitId, String>,
    loop_inputs: BTreeMap<usize, String>,
    loop_outputs: BTreeMap<usize, String>,
    /// Per junction, the merged wire names, empty for a structural junction.
    junction: Vec<Vec<String>>,
    shared: BTreeSet<Connection>,
}

impl Captions {
    /// The node's own caption: its block description, the flow header at start,
    /// or the return type at end.
    pub(crate) fn label(&self, node: NodeId) -> &str {
        self.label.get(&node).map_or("", String::as_str)
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

    /// The wires newly provided at this exit, labeled whether or not any
    /// connection leaves.
    pub(crate) fn handover(&self, exit: ExitId) -> &[String] {
        self.handover.get(&exit).map_or(&[], Vec::as_slice)
    }

    /// The authored description of a question branch, which replaces that
    /// branch's output hand-over label.
    pub(crate) fn branch_description(&self, exit: ExitId) -> Option<&str> {
        self.branch_description.get(&exit).map(String::as_str)
    }

    pub(crate) fn loop_inputs(&self, block: usize) -> &str {
        self.loop_inputs.get(&block).map_or("()", String::as_str)
    }

    pub(crate) fn loop_outputs(&self, block: usize) -> &str {
        self.loop_outputs.get(&block).map_or("()", String::as_str)
    }

    /// The wires that meet at one junction, in model order.
    pub(crate) fn junction_wires(&self, junction: usize) -> &[String] {
        self.junction.get(junction).map_or(&[], Vec::as_slice)
    }

    /// Whether the two ends of one connection are drawn as a single label.
    pub(crate) fn shares_label(&self, connection: &Connection) -> bool {
        self.shared.contains(connection)
    }
}

/// Reads every caption of one flow's topology. `start` labels the start node and
/// `return_type` captions end, both taken from the authored source text.
#[allow(clippy::too_many_lines)] // One cohesive pass derives every displayed caption.
pub(crate) fn derive(model: &SemanticModel, start: &str, return_type: &str) -> Captions {
    let topology = &model.topology;
    let parameters = named_parameters(model);
    let end_input = end_input(model);
    let mut captions = Captions::default();

    for node in &topology.nodes {
        let label = match node.id {
            NodeId::Start => start.to_owned(),
            NodeId::Block(_) if node.kind == NodeKind::End => return_type.to_owned(),
            // Undescribed calls use the callee path (RFC 0002 §4.3).
            NodeId::Block(block) => match &model.flow.blocks[block].description {
                Some(text) => text.clone(),
                None if node.kind == NodeKind::Call => model.flow.blocks[block].callee(),
                None => String::new(),
            },
            NodeId::Case { choice, branch } => {
                model.flow.blocks[choice].case_descriptions[branch].clone()
            }
        };
        captions.label.insert(node.id, label);
        // A case represents one choice output and captures nothing of its own.
        // End shows the transferred value, not every wire the structural
        // return captures for ordering.
        let capture = match node.id {
            NodeId::Block(_) if node.kind == NodeKind::End => end_input.clone(),
            NodeId::Block(block) => model.flow.blocks[block]
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

    for boundary in &topology.loop_boundaries {
        let block = &model.flow.blocks[boundary.header];
        captions.label.insert(
            NodeId::Block(boundary.header),
            block.description.clone().unwrap_or_default(),
        );
        captions.loop_inputs.insert(
            boundary.header,
            if block.inputs.is_empty() {
                "()".to_owned()
            } else {
                block
                    .inputs
                    .iter()
                    .map(captured)
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        );
        captions.loop_outputs.insert(
            boundary.header,
            if block.outputs.is_empty() {
                "()".to_owned()
            } else {
                (0..block.outputs.len())
                    .map(|output| binding_label(block.output_binding(output)))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        );
    }

    for exit in &topology.exits {
        captions.handover.insert(
            exit.id,
            exit.provides
                .iter()
                .map(|&producer| provided(model, &parameters, producer))
                .collect(),
        );
        if let (NodeId::Block(block), Some(branch)) = (exit.id.node, exit.id.branch)
            && let Some(description) = model.flow.blocks[block]
                .question_branches
                .get(branch)
                .and_then(|answer| answer.description.clone())
        {
            captions.branch_description.insert(exit.id, description);
        }
    }

    captions.junction = topology
        .junctions
        .iter()
        .map(|junction| {
            junction
                .merges
                .iter()
                .map(|&merge| provided(model, &parameters, model.merges[merge].producers[0]))
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
        && !topology.loop_boundaries.iter().any(|boundary| {
            boundary.result == Some(connection.source)
        })
        && captions.handover(exit) == captions.capture_label(node)
        && topology.leaving(exit).count() == 1
        && topology.single_arrival(node)
}

/// The named flow parameters, in flow-input order, as the start node's
/// hand-over addresses them. A receiver is one of them, under the one name
/// Rust gives it.
fn named_parameters(model: &SemanticModel) -> Vec<String> {
    model
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
fn provided(model: &SemanticModel, parameters: &[String], producer: ProducerId) -> String {
    match producer {
        ProducerId::FlowInput(input) => parameters
            .get(input)
            .expect("a flow input caption names a declared parameter")
            .clone(),
        ProducerId::CycleInput { block, input } => {
            let capture = &model.flow.blocks[block].inputs[input];
            format!(
                "{}{}",
                if capture.mutable { "mut " } else { "" },
                capture.alias.unraw()
            )
        }
        ProducerId::BlockOutput { block, output } => {
            binding_label(model.flow.blocks[block].output_binding(output))
        }
    }
}

/// Capture modifiers distinguish the four authored input forms.
fn captured(input: &Input) -> String {
    let borrow = if input.borrowed { "&" } else { "" };
    let mutable = if input.mutable { "mut " } else { "" };
    format!("{borrow}{mutable}{}", input.alias.unraw())
}

#[cfg(test)]
mod tests;
