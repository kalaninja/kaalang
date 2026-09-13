//! Derives the displayed text of one topology from the authored flow.
//!
//! RFC 0002 §6 gives a hand-over to the exit that provides it and a capture to
//! the node that receives it, so each is drawn once however many connections
//! leave or arrive. The model owns the structure; which string stands for a
//! producer occurrence, and whether the two ends of a connection may share one
//! label, are presentation choices and live here.

use std::collections::{BTreeMap, BTreeSet};

use kaalang_model::topology::{
    Connection, Destination, ExitId, NodeId, NodeKind, Source, Topology,
};
use kaalang_model::{Input, ProducerId, SemanticModel};
use syn::{FnArg, Pat, PatIdent, ext::IdentExt};

/// Every string the diagram shows, keyed by the structural item that owns it.
#[derive(Clone, Default)]
pub(crate) struct Captions {
    label: BTreeMap<NodeId, String>,
    capture: BTreeMap<NodeId, Vec<String>>,
    capture_label: BTreeMap<NodeId, Vec<String>>,
    handover: BTreeMap<ExitId, Vec<String>>,
    branch_description: BTreeMap<ExitId, String>,
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
pub(crate) fn derive(model: &SemanticModel, start: &str, return_type: &str) -> Captions {
    let topology = &model.topology;
    let parameters = named_parameters(model);
    let mut captions = Captions::default();

    for node in &topology.nodes {
        let label = match node.id {
            NodeId::Start => start.to_owned(),
            NodeId::Block(_) if node.kind == NodeKind::End => return_type.to_owned(),
            NodeId::Block(block) => model.flow.blocks[block]
                .description
                .clone()
                .unwrap_or_default(),
            NodeId::Case { choice, branch } => {
                model.flow.blocks[choice].case_descriptions[branch].clone()
            }
        };
        captions.label.insert(node.id, label);
        // A case represents one choice output and captures nothing of its own.
        let capture = match node.id {
            NodeId::Block(block) => model.flow.blocks[block]
                .inputs
                .iter()
                .map(captured)
                .collect(),
            NodeId::Start | NodeId::Case { .. } => Vec::new(),
        };
        let displayed = if capture.is_empty()
            && matches!(
                node.kind,
                NodeKind::Action | NodeKind::Question | NodeKind::Select
            )
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

/// Equal, nonempty displayed lists share a label only at the sole connection
/// between their ends. Capture modifiers and the empty-input marker never match
/// a bare wire name, and an exit handing over nothing shares no label with a
/// node that shows no capture list either.
///
/// Both checks are reached by authored flows: the outdegree one at a select
/// distributor, and the indegree one where two loop exits continue into one
/// block, as `multiple_exits` and `sequential_exits` do.
fn shares_label(topology: &Topology, captions: &Captions, connection: &Connection) -> bool {
    let (Source::Exit(exit), Destination::Node(node)) = (connection.source, connection.destination)
    else {
        return false;
    };
    !captions.handover(exit).is_empty()
        && captions.handover(exit) == captions.capture_label(node)
        && topology.leaving(exit).count() == 1
        && topology.single_arrival(node)
}

/// The named flow parameters, in signature order, as the start node's
/// hand-over addresses them.
fn named_parameters(model: &SemanticModel) -> Vec<&PatIdent> {
    model
        .parameters
        .iter()
        .filter_map(|parameter| {
            let FnArg::Typed(parameter) = parameter else {
                return None;
            };
            let Pat::Ident(binding) = parameter.pat.as_ref() else {
                return None;
            };
            Some(binding)
        })
        .collect()
}

/// The label one producer occurrence carries at the exit providing it.
fn provided(model: &SemanticModel, parameters: &[&PatIdent], producer: ProducerId) -> String {
    let binding = match producer {
        ProducerId::FlowInput(input) => *parameters
            .get(input)
            .expect("a flow input caption names a declared parameter"),
        ProducerId::BlockOutput { block, output } => {
            model.flow.blocks[block].output_binding(output)
        }
    };
    let mutable = if binding.mutability.is_some() {
        "mut "
    } else {
        ""
    };
    format!("{mutable}{}", binding.ident.unraw())
}

/// Capture modifiers distinguish the four authored input forms.
fn captured(input: &Input) -> String {
    let borrow = if input.borrowed { "&" } else { "" };
    let mutable = if input.mutable { "mut " } else { "" };
    format!("{borrow}{mutable}{}", input.alias.unraw())
}

#[cfg(test)]
mod tests;
