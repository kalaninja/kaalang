//! Exits own hand-overs; nodes own captures.

use std::collections::{BTreeMap, BTreeSet};

use kaalang_compiler::{
    Analysis, Input, ProducerId, SemanticModel,
    topology::{Connection, Destination, ExitId, Node, NodeId, NodeKind, Source, Topology},
};
use proc_macro2::Span;
use syn::{FnArg, Pat, PatIdent, ext::IdentExt};

pub(crate) use self::stage::derive_stage;
use crate::text::RichText;

mod stage;

/// One list of wire names, as a label draws it.
#[derive(Clone, Copy)]
pub(crate) enum WireList {
    /// The displayed captures of a node.
    Capture(NodeId),
    /// The hand-over of an exit.
    Handover(ExitId),
    /// The wires that meet at a junction.
    Junction(usize),
}

/// One name of a wire list and where its producer binds it.
type Wire = (String, Option<Span>);

/// The names of one wire list, and where the producer of each binds it. `()`
/// names no wire and has no binding.
#[derive(Clone, Default)]
struct NamedWires {
    names: Vec<String>,
    bindings: Vec<Option<Span>>,
}

impl FromIterator<Wire> for NamedWires {
    fn from_iter<I: IntoIterator<Item = Wire>>(wires: I) -> Self {
        let (names, bindings) = wires.into_iter().unzip();
        Self { names, bindings }
    }
}

impl NamedWires {
    /// Keeps the names, with their bindings, whose position `keep` accepts.
    fn retain_at(&mut self, keep: impl Fn(usize) -> bool) {
        let wires = std::mem::take(self);
        *self = wires
            .names
            .into_iter()
            .zip(wires.bindings)
            .enumerate()
            .filter(|(position, _)| keep(*position))
            .map(|(_, wire)| wire)
            .collect();
    }
}

/// Every string the diagram shows, keyed by the structural item that owns it.
#[derive(Clone, Default)]
pub(crate) struct Captions {
    label: BTreeMap<NodeId, RichText>,
    capture: BTreeMap<NodeId, Vec<String>>,
    capture_label: BTreeMap<NodeId, NamedWires>,
    handover: BTreeMap<ExitId, NamedWires>,
    branch_description: BTreeMap<ExitId, RichText>,
    cycle_inputs: BTreeMap<usize, String>,
    /// Per cycle, each declared output's binding label and bare name.
    cycle_outputs: BTreeMap<usize, Vec<(String, String)>>,
    /// Borrowed for a node that has no caption of its own.
    empty: RichText,
    /// Per junction, the merged wire names, empty for a structural junction.
    junction: Vec<NamedWires>,
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
        self.names(WireList::Capture(node))
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
        self.names(WireList::Handover(exit))
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
        self.names(WireList::Junction(junction))
    }

    fn wires(&self, list: WireList) -> Option<&NamedWires> {
        match list {
            WireList::Capture(node) => self.capture_label.get(&node),
            WireList::Handover(exit) => self.handover.get(&exit),
            WireList::Junction(junction) => self.junction.get(junction),
        }
    }

    /// The names one label lists.
    pub(crate) fn names(&self, list: WireList) -> &[String] {
        self.wires(list).map_or(&[], |wires| &wires.names)
    }

    /// Where the producer of each of those names binds it.
    pub(crate) fn bindings(&self, list: WireList) -> &[Option<Span>] {
        self.wires(list).map_or(&[], |wires| &wires.bindings)
    }

    /// Whether the two ends of one connection are drawn as a single label:
    /// equal, nonempty lists on a connection unique at both ends. Modifiers
    /// and empty-input markers prevent a match with bare wire names.
    pub(crate) fn shares_label(&self, topology: &Topology, connection: &Connection) -> bool {
        let (Source::Exit(exit), Destination::Node(node)) =
            (connection.source, connection.destination)
        else {
            return false;
        };
        !self.handover(exit).is_empty()
            // The body's hand-over and the cycle's output are separate transfers,
            // even when a body exit supplies the cycle's result directly.
            && !topology
                .cycle_boundaries
                .iter()
                .any(|boundary| boundary.results.contains(&connection.source))
            && self.matches_capture(topology.node(node), self.handover(exit))
            && topology.leaving(exit).count() == 1
            && topology.single_arrival(node)
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
        &model.analysis,
        None,
        start,
        return_type,
        &named_parameters(&model.analysis),
    )
}

/// `root` is the whole flow's analysis and `part` the local flow `model`
/// draws, as `Analysis::binding` takes them; `parameters` captions that local
/// flow's inputs.
#[allow(clippy::too_many_lines)] // One cohesive pass derives every displayed caption.
fn derive_with_parameters(
    model: &SemanticModel,
    root: &Analysis,
    part: Option<usize>,
    start: &str,
    return_type: &str,
    parameters: &[String],
) -> Captions {
    let topology = &model.topology;
    let flow = &model.analysis.flow;
    let binding = |producer| root.binding(part, producer);
    let produced = |producer| -> Wire {
        (
            provided(&model.analysis, parameters, producer),
            Some(binding(producer)),
        )
    };
    let end_input = end_input(model, binding);
    let mut captions = Captions::default();
    let captured_wire =
        |input: &Input| -> Wire { (captured(input), flow.producer(&input.ident).map(binding)) };

    for node in &topology.nodes {
        let label = match node.id {
            NodeId::Start => RichText::marked_code(start),
            NodeId::Block(_) if node.kind == NodeKind::End => RichText::code(return_type),
            // Undescribed calls use the callee path.
            NodeId::Block(block) => match &model.analysis.flow.blocks[block].description {
                Some(text) => RichText::markdown(text),
                None if node.kind == NodeKind::Call => {
                    RichText::code(model.analysis.flow.blocks[block].callee())
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
                cycle_inputs(model, block, produced)
            }
            // The for-entry captures what the for cycle's header does.
            NodeId::Block(block) if node.kind == NodeKind::ForEntry => {
                let header = model.analysis.flow.blocks[block]
                    .parent
                    .expect("a for-entry opens a for cycle");
                model.analysis.flow.blocks[header]
                    .inputs
                    .iter()
                    .filter(|input| !input.derived)
                    .map(captured_wire)
                    .collect()
            }
            NodeId::Block(block) => model.analysis.flow.blocks[block]
                .inputs
                .iter()
                .map(captured_wire)
                .collect(),
            NodeId::Start | NodeId::Case { .. } => Vec::new(),
        };
        let capture: NamedWires = capture.into_iter().collect();
        let displayed = if capture.names.is_empty()
            && !matches!(
                node.kind,
                NodeKind::Start | NodeKind::End | NodeKind::Case | NodeKind::ForEnd
            )
            && topology
                .incoming(Destination::Node(node.id))
                .next()
                .is_some()
        {
            NamedWires {
                names: vec!["()".to_owned()],
                bindings: vec![None],
            }
        } else {
            capture.clone()
        };
        captions.capture.insert(node.id, capture.names);
        captions.capture_label.insert(node.id, displayed);
    }

    for boundary in &topology.cycle_boundaries {
        let block = &model.analysis.flow.blocks[boundary.header];
        let label = block
            .description
            .as_deref()
            .map_or_else(RichText::default, RichText::markdown);
        // The for-end repeats the description of the cycle it closes.
        if let Some(caps) = boundary.caps {
            captions.label.insert(caps.bottom, label.clone());
        }
        captions.label.insert(NodeId::Block(boundary.header), label);
        let inputs = cycle_inputs(model, boundary.header, produced);
        if !inputs.is_empty() {
            let names: Vec<String> = inputs.into_iter().map(|(name, _)| name).collect();
            captions
                .cycle_inputs
                .insert(boundary.header, names.join(", "));
        }
    }
    // Both views name a cycle's outputs: the expanded results and the
    // collapsed node's exits.
    for (header, block) in model.analysis.flow.blocks.iter().enumerate() {
        if block.cycle_end.is_some() {
            captions.cycle_outputs.insert(
                header,
                (0..block.outputs.len())
                    .filter(|output| !block.unnamed_outputs.contains(output))
                    .map(|output| block.output_binding(output))
                    .map(|binding| (binding_label(binding), binding.ident.unraw().to_string()))
                    .collect(),
            );
        }
    }

    for exit in &topology.exits {
        let wires = exit
            .provides
            .iter()
            .filter(|&&producer| match producer {
                ProducerId::BlockOutput { block, output } => !model.analysis.flow.blocks[block]
                    .unnamed_outputs
                    .contains(&output),
                ProducerId::FlowInput(_) => true,
            })
            .map(|&producer| produced(producer))
            .collect();
        captions.handover.insert(exit.id, wires);
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
                .map(|&merge| produced(model.analysis.merges[merge].producers[0]))
                .collect()
        })
        .collect();

    captions
}

/// The value transferred into end, each wire with where its producer binds it.
fn end_input(model: &SemanticModel, binding: impl Fn(ProducerId) -> Span) -> Vec<Wire> {
    model
        .analysis
        .returned()
        .into_iter()
        .map(|operand| match operand {
            Some((name, producer)) => (name.unraw().to_string(), producer.map(&binding)),
            None => ("()".to_owned(), None),
        })
        .collect()
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
    let mutable = binding.mutability.map_or("", |_| "mut ");
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
fn cycle_inputs(
    model: &SemanticModel,
    block: usize,
    wire: impl Fn(ProducerId) -> Wire,
) -> Vec<Wire> {
    let flow = &model.analysis.flow;
    flow.blocks[block]
        .inputs
        .iter()
        .map(|input| {
            wire(
                flow.producer(&input.ident)
                    .expect("a cycle input names a produced wire"),
            )
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
