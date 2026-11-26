//! Where a flow's drawn elements stand in its source: facts a renderer reports
//! beside its drawing, with no say in what a reader does with them.

use proc_macro2::{Ident, Span};
use syn::{Expr, spanned::Spanned};

use crate::{
    Analysis, Block, BlockKind, ProducerId, choice_match,
    topology::{Node, NodeId, NodeKind},
};

impl Analysis {
    /// The local flow `part` of this flow: a stage by its index, or with `None`
    /// the flow itself, or its preparation.
    fn part(&self, part: Option<usize>) -> &Self {
        part.map_or(self, |index| &self.stages[index].analysis)
    }

    /// Where `producer`, a producer of the local flow `part`, binds its wire:
    /// in the signature, or in an output pattern. A stage's entry is bound where
    /// the stage's header names it, and a wire a stage shares with preparation
    /// where preparation binds it.
    ///
    /// # Panics
    ///
    /// Panics unless `producer` is one of that local flow.
    #[must_use]
    pub fn binding(&self, part: Option<usize>, producer: ProducerId) -> Span {
        let local = self.part(part);
        match producer {
            ProducerId::BlockOutput { block, output } => {
                local.flow.blocks[block].output_binding(output).ident.span()
            }
            ProducerId::FlowInput(input) => {
                let wire = &local.flow.flow_inputs[input];
                match part {
                    Some(stage) if *wire != self.stages[stage].entry => {
                        let producer = self
                            .flow
                            .producer(wire)
                            .expect("a wire a stage shares has a producer in preparation");
                        self.binding(None, producer)
                    }
                    _ => wire.span(),
                }
            }
        }
    }

    /// The source `node`, a node of the local flow `part`, draws: where its
    /// block starts, at its kind attribute. Start draws the flow's name, a
    /// stage's entry its stage and a transition the stage it selects, each at the
    /// stage's attribute, end the `return`, a for cycle's caps their cycle, and a
    /// case its `match` arm. End draws nothing in a part without a `return`.
    ///
    /// # Panics
    ///
    /// Panics unless `node` is one of that local flow.
    #[must_use]
    pub fn node_span(&self, part: Option<usize>, node: &Node) -> Option<Span> {
        let flow = &self.part(part).flow;
        let index = match node.id {
            NodeId::Start => {
                return Some(
                    part.map_or_else(|| self.name.span(), |stage| self.stages[stage].span),
                );
            }
            NodeId::Case { choice, branch } => {
                let choice =
                    choice_match(&flow.blocks[choice].body).expect("choice bodies are validated");
                return Some(choice.arms[branch].span());
            }
            NodeId::Block(index) => index,
        };
        let block = &flow.blocks[index];
        Some(match node.kind {
            NodeKind::ForEntry | NodeKind::ForEnd => {
                flow.blocks[block.parent.expect("a for cycle's cap stands in its body")].span
            }
            NodeKind::End => self.part(part).returning()?.span,
            NodeKind::StageTransition => {
                let target = block
                    .transition_target
                    .expect("a transition selects a stage");
                self.stages[target].span
            }
            _ => block.span,
        })
    }

    /// What this local flow's `return` hands to end, operand by operand: the
    /// name each operand gives its wire, with the producer whose wire it is, or
    /// `None` for `()`. A tuple-valued wire is one operand.
    #[must_use]
    pub fn returned(&self) -> Vec<Option<(&Ident, Option<ProducerId>)>> {
        let Some(block) = self.returning() else {
            return Vec::new();
        };
        operands(&block.body)
            .into_iter()
            .map(|operand| {
                operand.map(|operand| {
                    let producer = crate::parse::capture(&block.inputs, operand)
                        .and_then(|input| self.flow.producer(&input.ident));
                    (operand, producer)
                })
            })
            .collect()
    }

    /// The authored `return`, not a stage's transition.
    fn returning(&self) -> Option<&Block> {
        self.flow
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Return && block.transition_target.is_none())
    }
}

/// A return value's operands in order, `None` for the empty tuple.
fn operands(value: &Expr) -> Vec<Option<&Ident>> {
    match value {
        Expr::Group(group) => operands(&group.expr),
        Expr::Paren(parenthesized) => operands(&parenthesized.expr),
        Expr::Path(path) => vec![Some(
            path.path
                .get_ident()
                .expect("a return value is one captured input"),
        )],
        Expr::Tuple(tuple) if tuple.elems.is_empty() => vec![None],
        Expr::Tuple(tuple) => tuple.elems.iter().flat_map(operands).collect(),
        _ => unreachable!("a return value is validated as captured inputs"),
    }
}
