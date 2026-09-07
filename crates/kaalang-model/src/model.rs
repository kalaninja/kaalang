//! The authored and resolved flow models, the recorded executions and
//! convergence groups and wire merges, and the compiler's execution plan.

use std::cmp::Ordering;

use proc_macro2::{Ident, Span};
use syn::{Expr, FnArg, ReturnType};

/// A validated kaalang flow: its blocks, every possible execution, its
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
    /// Every possible execution, ordered by branch selections, then blocks,
    /// then capture dependencies and implicit block order.
    pub executions: Vec<Execution>,
    /// Every continuation group, ordered by branching block, then branch list.
    pub convergence_groups: Vec<ConvergenceGroup>,
    /// Implicit junctions of equally named alternative outputs, before captures.
    pub merges: Vec<WireMerge>,
}

/// The logical wire whose value is the flow output. The implicit end block
/// captures it; no computational block may.
pub(crate) const RESULT_WIRE: &str = "result";

/// The semantic role of one block. Every kind but `End` is authored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Action,
    Question,
    Choice,
    End,
}

/// One kaalang block: an authored statement, or the implicit end block.
pub struct Block {
    pub kind: BlockKind,
    /// The exact authored description, absent for the implicit end block.
    pub description: Option<String>,
    /// The ordered authored case descriptions of a choice.
    pub case_descriptions: Vec<String>,
    pub outputs: Vec<Ident>,
    pub output_span: Span,
    pub inputs: Vec<Input>,
    pub body: Expr,
    pub span: Span,
}

/// One consuming or borrowing block input.
pub struct Input {
    pub borrowed: bool,
    /// The logical wire name: raw and ordinary spellings normalize to one ident.
    pub ident: Ident,
    /// The authored spelling, which keeps `r#` so a keyword-named wire binds.
    pub alias: Ident,
}

/// A flow's named inputs and blocks, wire-validated by the time consumers see it.
pub struct Flow {
    pub flow_inputs: Vec<Ident>,
    pub blocks: Vec<Block>,
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

/// One possible execution: the computational blocks that participate, the
/// branches it selects, its capture dependencies, and implicit block order. Start and
/// end participate implicitly. The serial order in which independent blocks
/// happened to run is not part of an execution; every vector is sorted and
/// deduplicated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Execution {
    pub blocks: Vec<usize>,
    pub branches: Vec<BranchSelection>,
    pub dependencies: Vec<CaptureDependency>,
    /// Implicit block order, as `(before, after)` pairs: an action producing an
    /// ordinary wire precedes a selection deciding its consumers. These pairs add
    /// no capture and are sorted and deduplicated like capture dependencies.
    pub ordering: Vec<(usize, usize)>,
}

impl Execution {
    /// Reports whether a computational block runs in this execution.
    #[must_use]
    pub fn participates(&self, block: usize) -> bool {
        self.blocks.binary_search(&block).is_ok()
    }
}

impl Ord for Execution {
    fn cmp(&self, other: &Self) -> Ordering {
        self.branches
            .cmp(&other.branches)
            .then_with(|| self.blocks.cmp(&other.blocks))
            .then_with(|| self.dependencies.cmp(&other.dependencies))
            .then_with(|| self.ordering.cmp(&other.ordering))
    }
}

impl PartialOrd for Execution {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A dependency-derived shared continuation of one question or choice.
/// Its entries are computational blocks after implicit `WireMerge` junctions.
/// This projection does not prescribe the scopes or joins used by lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConvergenceGroup {
    pub branching_block: usize,
    /// Question-output or choice-case positions, in authored order.
    pub branches: Vec<usize>,
    /// The shared continuation: the computational blocks whose branch set is
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
    /// Blocks ordered after this merge, in authored order: its consumers and
    /// eligible questions or choices that decide them. An implicit ordering does
    /// not add a capture to the branching block.
    pub after: Vec<usize>,
}

/// A verified lowering plan: one permitted serial order of the flow. Every
/// kaalang invariant already holds, so consumers do not need to walk the
/// authored flow a second time. Its joins are lowering structure and say
/// nothing about semantic convergence groups.
pub enum ExecutionPlan {
    /// Runs each authored block once when its captures are available. This
    /// schedule represents dependency graphs that cannot share every body in
    /// nested Rust branches; lowering keeps a slot for each logical wire.
    Guarded {
        /// The wires already bound when the schedule starts: the flow inputs
        /// for a whole-flow schedule, or the bindings a structured prefix
        /// leaves available, with one producer, in every execution entering
        /// this suffix.
        inputs: Vec<Ident>,
        /// The remaining computational blocks, ordered by dependencies and
        /// implicit wire order, with authored order breaking ties.
        blocks: Vec<usize>,
    },
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
    /// The implicit end block and the execution that feeds it.
    End {
        index: usize,
        body: Box<ExecutionPlan>,
        /// Logical wire names whose alternative producers no common binding
        /// unifies; lowering adds a type gate for each.
        gates: Vec<Ident>,
    },
    /// One branch hands the `result` wire to end.
    EndArrival { result: Ident },
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
    /// Set when a sibling yields to a join, which forces this branch to return
    /// its end value rather than yield it to the enclosing expression.
    pub early_return: bool,
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
    /// Set when a sibling join of the same block yields to an enclosing join
    /// while this continuation reaches end.
    pub early_return: bool,
}
