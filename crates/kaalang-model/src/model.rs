//! The authored and resolved flow models, the recorded executions and
//! convergence groups and wire merges, and the compiler's execution plan.

use proc_macro2::{Ident, Span};
use syn::ext::IdentExt;
use syn::{Expr, FnArg, Lifetime, Pat, PatIdent, ReturnType};

use crate::construct::Arrangement;
use crate::topology::Topology;

/// A validated kaalang flow: its blocks, finite structural execution summaries, its
/// convergence groups and wire merges, and the verified plan that lowers it.
pub struct SemanticModel {
    /// The authored flow function name.
    pub name: Ident,
    /// The authored flow parameters.
    pub parameters: Vec<FnArg>,
    /// The authored flow return type.
    pub return_type: ReturnType,
    /// The resolved blocks and wires, ending with the implicit end block.
    pub flow: Flow,
    /// The verified lowering plan.
    pub execution_plan: ExecutionPlan,
    /// Every structural execution summary, ordered by branch selections, then blocks,
    /// then capture dependencies and implicit block order.
    pub executions: Vec<Execution>,
    /// Every continuation group, ordered by branching block, then branch list.
    pub convergence_groups: Vec<ConvergenceGroup>,
    /// Implicit junctions of equally named alternative outputs, before captures.
    pub merges: Vec<WireMerge>,
    /// The diagram's structural topology: its nodes, exits, junctions, and the
    /// connections RFC 0002 §7 draws between them.
    pub topology: Topology,
    /// The checked arrangement of that topology. Every accepted flow has one:
    /// `build` decides realizability, so a renderer realizes this rather than
    /// looking for an arrangement of its own.
    pub arrangement: Arrangement,
}

impl SemanticModel {
    /// Removes redundant bends and spacing from a diagram with long detours.
    /// Every replacement passes the complete arrangement verifier. Failure to
    /// simplify keeps the existing witness; it never rejects the flow.
    /// This optional presentation work is not part of macro compilation.
    pub fn compact_arrangement(&mut self) {
        if self
            .arrangement
            .routes
            .iter()
            .any(|route| route.runs.len() > 2)
        {
            crate::construct::compact::arrangement(
                &self.flow,
                &self.topology,
                &mut self.arrangement,
            );
        }
    }

    /// The vertices one loop's body draws, so a presentation measures the same
    /// body the construction did when it placed the loop's return
    /// (RFC 0002 §8).
    #[must_use]
    pub fn body_vertices(
        &self,
        header: usize,
    ) -> std::collections::BTreeSet<crate::topology::Vertex> {
        crate::construct::body_vertices(&self.flow, &self.topology, header)
    }
}

/// The logical wire whose value is the flow output. The implicit end block
/// captures it; no authored block may.
pub(crate) const END_WIRE: &str = "end";

/// The semantic role of one block. Every kind but `End` is authored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Action,
    Question,
    Loop,
    Break,
    Choice,
    End,
}

/// One kaalang block: an authored statement, or the implicit end block.
pub struct Block {
    pub kind: BlockKind,
    /// The exact authored description, absent for loops, breaks, and end.
    pub description: Option<String>,
    /// A question's two answers, paired with its outputs.
    pub question_branches: Vec<QuestionBranch>,
    /// The ordered authored case descriptions of a choice.
    pub case_descriptions: Vec<String>,
    pub outputs: Vec<Ident>,
    /// The validated identifier or flat tuple pattern declaring the outputs.
    /// Each binding preserves its authored mutability. An outputless block uses `()`.
    pub output_pattern: Pat,
    pub output_span: Span,
    pub inputs: Vec<Input>,
    /// The authored body normalized to a plain block expression.
    pub body: Expr,
    pub span: Span,
    /// The enclosing loop block, if this block belongs to an iteration.
    pub parent: Option<usize>,
    /// The exclusive end of a loop body's depth-first block sequence.
    pub loop_end: Option<usize>,
    /// The authored loop label, resolved lexically by break statements.
    pub loop_label: Option<Lifetime>,
    /// The enclosing loop a break exits.
    pub break_target: Option<usize>,
}

impl Block {
    /// The number of alternative control exits, for a question or choice.
    #[must_use]
    pub fn branch_count(&self) -> usize {
        match self.kind {
            BlockKind::Question => 2,
            BlockKind::Choice => self.outputs.len(),
            _ => 0,
        }
    }

    /// Returns the authored binding at one validated output position.
    ///
    /// # Panics
    ///
    /// Panics if the position does not exist or the output pattern is invalid.
    #[must_use]
    pub fn output_binding(&self, index: usize) -> &PatIdent {
        let pattern = match &self.output_pattern {
            Pat::Tuple(tuple) => &tuple.elems[index],
            pattern if index == 0 => pattern,
            _ => panic!("output position must exist"),
        };
        let Pat::Ident(binding) = pattern else {
            unreachable!("validated outputs are identifier bindings")
        };
        binding
    }
}

/// One positional branch of a question.
pub struct QuestionBranch {
    /// Whether a true question body selects this branch.
    pub is_yes: bool,
    /// Its optional authored description.
    pub description: Option<String>,
}

/// One consuming or borrowing block input.
pub struct Input {
    /// Whether the input borrows the wire rather than binding its value.
    pub borrowed: bool,
    /// Mutability of the reference for a borrow, or of the local value binding.
    pub mutable: bool,
    /// The logical wire key: raw spellings normalize and loop locals are scoped.
    pub ident: Ident,
    /// The authored spelling, which keeps `r#` so a keyword-named wire binds.
    pub alias: Ident,
}

/// A flow's named inputs and blocks, wire-validated by the time consumers see it.
pub struct Flow {
    pub flow_inputs: Vec<Ident>,
    pub blocks: Vec<Block>,
}

impl Flow {
    /// The loops enclosing one block, innermost first.
    pub(crate) fn enclosing(&self, block: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(self.blocks[block].parent, |&header| {
            self.blocks[header].parent
        })
    }

    /// The span of the implicit end block, where a diagnostic about the flow as
    /// a whole lands.
    pub(crate) fn end_span(&self) -> Span {
        self.blocks
            .last()
            .expect("a flow owns the implicit end block")
            .span
    }

    /// The displayed name of a wire, without internal scope keys or raw prefixes.
    #[must_use]
    pub(crate) fn wire_name(&self, wire: &Ident) -> String {
        self.blocks
            .iter()
            .find_map(|block| {
                block
                    .outputs
                    .iter()
                    .position(|output| output == wire)
                    .map(|index| block.output_binding(index).ident.unraw().to_string())
            })
            .unwrap_or_else(|| wire.unraw().to_string())
    }
}

/// One occurrence that provides a wire: a named flow input or one output of
/// one block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProducerId {
    FlowInput(usize),
    BlockOutput { block: usize, output: usize },
}

/// One block input, identified by the block and the input position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaptureId {
    pub block: usize,
    pub input: usize,
}

/// A capture resolved to the producer occurrence that provided it in one
/// execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaptureDependency {
    pub producer: ProducerId,
    pub capture: CaptureId,
}

/// The output a question or choice selected in one execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BranchSelection {
    pub block: usize,
    pub branch: usize,
}

/// One structural execution summary: the authored blocks that participate,
/// the branches it selects, its capture dependencies, and its finite outcome.
/// Start participates implicitly; end participates for an `End` outcome. Each
/// loop is represented by zero or one iteration. Source order is the order the
/// blocks run in, so a summary records which of them take part rather than a
/// schedule; every vector is sorted and deduplicated. Field order is the derived
/// sort order: branch selections first.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Execution {
    pub branches: Vec<BranchSelection>,
    pub blocks: Vec<usize>,
    pub dependencies: Vec<CaptureDependency>,
    /// Loops whose represented iteration reaches its end. Each execution
    /// summarizes at most one iteration per loop.
    pub repeats: Vec<usize>,
    /// Whether this finite summary finishes the flow or repeats a loop.
    pub outcome: ExecutionOutcome,
}

/// The boundary reached by one finite structural execution summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionOutcome {
    End,
    Repeat { loop_index: usize },
}

impl Execution {
    /// Reports whether an authored block runs in this execution.
    #[must_use]
    pub fn participates(&self, block: usize) -> bool {
        self.blocks.binary_search(&block).is_ok()
    }

    /// The output a question or choice selected here, or `None` for a block
    /// that does not branch.
    #[must_use]
    pub fn selected(&self, block: usize) -> Option<usize> {
        self.branches
            .iter()
            .find(|selection| selection.block == block)
            .map(|selection| selection.branch)
    }
}

/// A dependency-derived shared continuation of one question or choice.
/// Its entries are authored blocks after implicit `WireMerge` junctions.
/// This projection does not prescribe the scopes or joins used by lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConvergenceGroup {
    pub branching_block: usize,
    /// Question-output or choice-case positions, in authored order.
    pub branches: Vec<usize>,
    /// The shared continuation: the authored blocks whose branch set is
    /// exactly `branches`, in authored order. The end block never belongs.
    pub continuation: Vec<usize>,
    /// The first consumers of the shared continuation, not merge points.
    /// These are the blocks that no other continuation block precedes in
    /// any execution, including executions in which the branching block does
    /// not run. In authored order and never empty: nothing in the continuation
    /// precedes its first block.
    pub entries: Vec<usize>,
}

/// One implicit convergence point for a logical wire. Repeated output names
/// always join here before any consumer captures the wire. This junction is
/// neither an authored block nor a new producer occurrence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireMerge {
    pub wire: Ident,
    pub producers: Vec<ProducerId>,
    /// Every block whose execution the producer-selecting question or choice
    /// decides, the producers included. Each must finish before the merge in
    /// any execution where it participates, so no branch-local value outlives
    /// its own branch.
    pub before: Vec<usize>,
    /// Its consumers, in source order. Every block in `before` is declared
    /// above every one of them.
    pub after: Vec<usize>,
}

/// A verified lowering plan: one permitted serial order of the flow. Every
/// kaalang invariant already holds, so consumers do not need to walk the
/// authored flow a second time. Its joins are lowering structure and say
/// nothing about semantic convergence groups.
pub enum ExecutionPlan {
    Loop {
        index: usize,
        body: Box<ExecutionPlan>,
        next: Option<Box<ExecutionPlan>>,
    },
    /// An authored exit from an active enclosing loop.
    Break { index: usize, target: usize },
    /// Normal completion of an iteration, returning to the loop's entry.
    Repeat { index: usize },
    Action {
        index: usize,
        next: Box<ExecutionPlan>,
    },
    Question {
        index: usize,
        branches: [Branch; 2],
        join: Option<Join>,
    },
    Choice {
        index: usize,
        branches: Vec<Branch>,
        /// One join per set of cases that share downstream computation,
        /// ordered by first case.
        joins: Vec<Join>,
    },
    /// The implicit end block and the plan body, whether or not a path reaches it.
    End {
        index: usize,
        body: Box<ExecutionPlan>,
        /// Logical wire names whose alternative producers no common binding
        /// unifies; lowering adds a type gate for each.
        gates: Vec<Ident>,
    },
    /// One branch hands the `end` wire to end.
    EndArrival { wire: Ident },
    /// The branch yields alternative producer values to a join.
    Yield { wires: Vec<Ident>, join: JoinTarget },
}

/// The join a yield enters: an enclosing question or choice and the position
/// of the join among that block's joins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JoinTarget {
    pub block: usize,
    pub join: usize,
}

/// One verified branch continuation.
pub struct Branch {
    pub plan: Box<ExecutionPlan>,
}

/// Logical wire bindings and the continuation shared by the branches that
/// yield into one join. A join is lowering structure; the semantic
/// `ConvergenceGroup` records are derived from the executions instead.
pub struct Join {
    /// The output positions with at least one yield into this join, in
    /// authored order. Another yield of the same position may pass this join
    /// toward an enclosing one, so codegen routes by each yield's target; the
    /// renderer and the plan replay read this list.
    pub branches: Vec<usize>,
    /// Logical wire names, ordered by their first authored producer.
    pub wires: Vec<Ident>,
    pub next: Box<ExecutionPlan>,
}
