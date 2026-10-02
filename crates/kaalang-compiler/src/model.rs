//! Flow models, execution summaries, convergence, and lowering plans.

use proc_macro2::{Delimiter, Ident, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::ext::IdentExt;
use syn::{Expr, FnArg, Pat, PatIdent, ReturnType};

use crate::construct::Arrangement;
use crate::executions::Executions;
use crate::topology::Topology;

/// Semantic analysis and lowering plan, before diagram construction.
#[derive(Clone)]
pub struct Analysis {
    /// The authored flow function name.
    pub name: Ident,
    /// The authored flow parameters.
    pub parameters: Vec<FnArg>,
    /// The authored flow return type.
    pub return_type: ReturnType,
    /// The resolved blocks and wires, ending with the implicit completion boundary.
    pub flow: Flow,
    /// The verified lowering plan.
    pub execution_plan: ExecutionPlan,
    /// Structural summaries in [`Execution`]'s derived order. Accessing the
    /// complete slice enumerates all executions on demand.
    pub executions: Executions,
    /// Every continuation group, ordered by branching block, then branch list.
    pub convergence_groups: Vec<ConvergenceGroup>,
    /// Implicit junctions of equally named alternative outputs, before captures.
    pub merges: Vec<WireMerge>,
    /// Other locally analyzed parts of a staged flow, in declaration order.
    pub stages: Vec<StageAnalysis>,
    /// Preparation outputs that remain in scope across stage visits.
    pub common_wires: Vec<Ident>,
}

/// One stage and its independent, verified local execution plan.
#[derive(Clone)]
pub struct StageAnalysis {
    pub description: String,
    pub entry: Ident,
    pub entry_alias: Ident,
    pub analysis: Box<Analysis>,
}

/// An analyzed flow with a verified diagram arrangement. See [`Analysis`].
pub struct SemanticModel {
    /// Everything the analysis established, before any diagram.
    pub analysis: Analysis,
    /// Structural topology of the flow.
    pub topology: Topology,
    /// Verified arrangement for the renderer to realize.
    pub arrangement: Arrangement,
    /// Locally checked stage diagrams, in declaration order.
    pub stages: Vec<SemanticModel>,
}

/// The semantic role of one block. Every kind but `End` and `Export` is authored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Action,
    Call,
    Question,
    Cycle,
    /// The boundary consumer of one declared cycle output, closing the body in
    /// declaration order. It lowers to a Rust `break`.
    Export,
    Continue,
    Return,
    Choice,
    End,
}

/// One kaalang block: an authored statement, a cycle's boundary consumer, or
/// the implicit end block.
#[derive(Clone)]
pub struct Block {
    pub kind: BlockKind,
    /// The exact authored description, absent for transfers and end.
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
    /// The authored captures, in order. A cycle's are its optional gate, then
    /// its derived inputs.
    pub inputs: Vec<Input>,
    /// The authored body normalized to a plain block expression.
    pub body: Expr,
    pub span: Span,
    /// The enclosing cycle block, if this block belongs to an iteration.
    pub parent: Option<usize>,
    /// The exclusive end of a cycle body's depth-first block sequence.
    pub cycle_end: Option<usize>,
    /// The cycle whose declared output a boundary consumer exports.
    pub export_target: Option<usize>,
    /// Destination of a synthetic stage transition, absent on authored returns.
    pub transition_target: Option<usize>,
}

impl Block {
    /// The number of alternative control exits: a question's two answers, a
    /// choice's cases, or a cycle's declared outputs when it has several. A
    /// single cycle output is an ordinary completed result.
    #[must_use]
    pub fn branch_count(&self) -> usize {
        match self.kind {
            BlockKind::Question => 2,
            BlockKind::Choice => self.outputs.len(),
            BlockKind::Cycle if self.outputs.len() > 1 => self.outputs.len(),
            _ => 0,
        }
    }

    /// Whether this is a cycle whose declared outputs are alternatives: it
    /// completes with exactly one of several, like the cases of a choice.
    #[must_use]
    pub fn has_alternative_outputs(&self) -> bool {
        self.kind == BlockKind::Cycle && self.outputs.len() > 1
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

    /// The callee path with normalized spacing and no comments, e.g.
    /// `math:: /* note */ twice` becomes `math::twice`.
    ///
    /// # Panics
    ///
    /// Panics unless this block is a call, whose body `parse::call` validated.
    #[must_use]
    pub fn callee(&self) -> String {
        let Expr::Call(application) = &self.body else {
            unreachable!("parse::call validates a call body as one application")
        };
        // A `macro_rules!` substitution arrives inside an invisible group.
        let Expr::Path(path) = crate::parse::ungrouped(&application.func) else {
            unreachable!("parse::call validates a callee as a path")
        };
        let mut text = String::new();
        write_tokens(&mut text, path.to_token_stream());
        text
    }
}

/// Appends every token in source order, separated only where running two
/// together would change what they say.
fn write_tokens(text: &mut String, tokens: TokenStream) {
    for token in tokens {
        if needs_space(text, &token) {
            text.push(' ');
        }
        match token {
            TokenTree::Group(group) => {
                let (open, close) = delimiters(group.delimiter());
                text.push_str(open);
                write_tokens(text, group.stream());
                text.push_str(close);
            }
            TokenTree::Punct(punct) => text.push(punct.as_char()),
            token => text.push_str(&token.to_string()),
        }
    }
}

/// Separates comma-delimited items and adjacent words, including `Vec<u8> as ::Trait`.
fn needs_space(text: &str, token: &TokenTree) -> bool {
    if text.ends_with(',') {
        return true;
    }
    text.ends_with(word_end)
        && (matches!(token, TokenTree::Ident(_) | TokenTree::Literal(_)) || text.ends_with(" as"))
}

/// Whether a character can end a word or the type a word wrapped.
fn word_end(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '>' | ']' | ')')
}

/// Delimiters; invisible macro substitution groups add no characters.
const fn delimiters(delimiter: Delimiter) -> (&'static str, &'static str) {
    match delimiter {
        Delimiter::Parenthesis => ("(", ")"),
        Delimiter::Brace => ("{", "}"),
        Delimiter::Bracket => ("[", "]"),
        Delimiter::None => ("", ""),
    }
}

/// One positional branch of a question.
#[derive(Clone)]
pub struct QuestionBranch {
    /// Whether a true question body selects this branch.
    pub is_yes: bool,
    /// Its optional authored description.
    pub description: Option<String>,
}

/// One consuming or borrowing block input.
#[derive(Clone)]
pub struct Input {
    /// Whether the input borrows the wire rather than binding its value.
    pub borrowed: bool,
    /// Mutability of the reference for a borrow, or of the local value binding.
    pub mutable: bool,
    /// The logical wire key: raw spellings normalize and cycle locals are scoped.
    pub ident: Ident,
    /// The authored spelling, which keeps `r#` so a keyword-named wire binds.
    pub alias: Ident,
    /// An outer wire a cycle's body captures, recorded on the cycle itself so
    /// the dependency enters through the cycle. It binds
    /// nothing and does not decide whether the cycle runs.
    pub derived: bool,
}

/// A flow's named inputs and blocks, wire-validated by the time consumers see it.
#[derive(Clone)]
pub struct Flow {
    pub flow_inputs: Vec<Ident>,
    pub blocks: Vec<Block>,
    pub kind: FlowKind,
}

/// The local sequence represented by a flow's block arena.
#[derive(Clone)]
pub enum FlowKind {
    Plain,
    Preparation,
    Stage { entry: Ident, self_output: bool },
}

impl Flow {
    /// The first producer of one logical wire. Alternative producers share its
    /// name and mutability.
    #[must_use]
    pub fn producer(&self, wire: &Ident) -> Option<ProducerId> {
        self.flow_inputs
            .iter()
            .position(|input| input == wire)
            .map(ProducerId::FlowInput)
            .or_else(|| {
                self.blocks
                    .iter()
                    .enumerate()
                    .find_map(|(block, declaration)| {
                        declaration
                            .outputs
                            .iter()
                            .position(|output| output == wire)
                            .map(|output| ProducerId::BlockOutput { block, output })
                    })
            })
    }

    /// The cycle whose body provides a producer's wire, or `None` at the root.
    pub(crate) fn producer_cycle(&self, producer: ProducerId) -> Option<usize> {
        match producer {
            ProducerId::FlowInput(_) => None,
            ProducerId::BlockOutput { block, .. } => self.blocks[block].parent,
        }
    }

    /// The cycles enclosing one block, innermost first.
    pub(crate) fn enclosing(&self, block: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(self.blocks[block].parent, |&header| {
            self.blocks[header].parent
        })
    }

    /// The hidden boundary consumers of a cycle's declared outputs: one per
    /// output, in declaration order, closing its body.
    #[must_use]
    pub(crate) fn exports(&self, header: usize) -> std::ops::Range<usize> {
        let end = self.blocks[header].cycle_end.expect("a cycle owns a body");
        end - self.blocks[header].outputs.len()..end
    }

    /// The position, among its cycle's declared outputs, of the one a boundary
    /// consumer exports.
    #[must_use]
    pub(crate) fn exported_output(&self, consumer: usize) -> usize {
        let header = self.blocks[consumer]
            .export_target
            .expect("a boundary consumer has a cycle");
        consumer - self.exports(header).start
    }

    /// The innermost cycle whose repeat reaches `block`: the block itself for a
    /// cycle, otherwise the cycle directly containing it.
    #[must_use]
    pub(crate) fn level(&self, block: usize) -> Option<usize> {
        match self.blocks[block].kind {
            BlockKind::Cycle => Some(block),
            _ => self.blocks[block].parent,
        }
    }

    /// The repeat relation depends on where two blocks sit, not on which
    /// execution repeats. [`Passes::of`] computes it once per cycle and block.
    pub(crate) fn repeat_reaches(
        &self,
        cycle_index: usize,
        block: usize,
        together: impl FnOnce(usize, usize) -> bool,
    ) -> bool {
        if std::iter::successors(self.level(block), |&header| self.blocks[header].parent)
            .any(|header| header == cycle_index)
        {
            return true;
        }
        let sequence = std::iter::successors(Some(self.blocks[block].parent), |&sequence| {
            sequence.map(|header| self.blocks[header].parent)
        })
        .find(|&sequence| {
            std::iter::successors(Some(cycle_index), |&header| self.blocks[header].parent)
                .any(|header| self.blocks[header].parent == sequence)
        })
        .expect("the root contains every block");
        let part = |mut block: usize| {
            while self.blocks[block].parent != sequence {
                block = self.blocks[block]
                    .parent
                    .expect("the sequence encloses the block");
            }
            block
        };
        let (block_part, repeat_part) = (part(block), part(cycle_index));
        block_part == repeat_part || (block_part < repeat_part && together(block_part, repeat_part))
    }

    /// A cycle completes only when the execution exports one of its outputs.
    #[must_use]
    pub(crate) fn completes_cycle(&self, execution: &Execution, header: usize) -> bool {
        self.exports(header)
            .any(|consumer| execution.participates(consumer))
    }

    /// Whether one producer occurrence exists in this finite execution.
    #[must_use]
    pub(crate) fn produces(&self, execution: &Execution, producer: ProducerId) -> bool {
        match producer {
            ProducerId::FlowInput(_) => true,
            ProducerId::BlockOutput { block, output } => {
                execution.participates(block)
                    && match self.blocks[block].kind {
                        BlockKind::Question | BlockKind::Choice => {
                            execution.selected(block) == Some(output)
                        }
                        BlockKind::Cycle => {
                            execution.participates(self.exports(block).start + output)
                        }
                        BlockKind::Action | BlockKind::Call => true,
                        BlockKind::Export
                        | BlockKind::Continue
                        | BlockKind::Return
                        | BlockKind::End => false,
                    }
            }
        }
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
        match self.producer(wire) {
            Some(ProducerId::BlockOutput { block, output }) => self.blocks[block]
                .output_binding(output)
                .ident
                .unraw()
                .to_string(),
            _ => wire.unraw().to_string(),
        }
    }
}

/// One occurrence that provides a wire: a flow input or one block output.
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

/// A finite execution summary with at most one iteration per cycle.
/// Start is implicit; `Return` reaches end. Blocks run in source order.
/// All vectors are sorted and deduplicated; field order defines the derived order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Execution {
    pub branches: Vec<BranchSelection>,
    pub blocks: Vec<usize>,
    pub dependencies: Vec<CaptureDependency>,
    /// Whether this finite summary finishes the flow or repeats a cycle.
    pub outcome: ExecutionOutcome,
}

/// Whether a repeated cycle reaches each block, computed from the finite
/// executions once per analysis. Blocks of one sequence run in source order.
pub(crate) struct Passes {
    /// Indexed by repeating cycle, then queried block. Non-cycle rows are empty.
    reaching: Vec<Vec<bool>>,
}

impl Passes {
    /// Whether an execution reaches `block`: it finishes the flow, repeats a
    /// cycle enclosing `block`, or passes `block` before its repetition.
    #[must_use]
    pub(crate) fn reaches(&self, execution: &Execution, block: usize) -> bool {
        let ExecutionOutcome::Repeat { cycle_index } = execution.outcome else {
            return true;
        };
        self.reaching[cycle_index][block]
    }

    #[must_use]
    pub(crate) fn of(flow: &Flow, executions: &[Execution]) -> Self {
        let mut runs =
            vec![vec![0u64; executions.len().div_ceil(u64::BITS as usize)]; flow.blocks.len()];
        for (index, execution) in executions.iter().enumerate() {
            for &block in &execution.blocks {
                runs[block][index / u64::BITS as usize] |= 1 << (index % u64::BITS as usize);
            }
        }
        let reaching = flow
            .blocks
            .iter()
            .enumerate()
            .map(|(cycle_index, block)| {
                if block.kind == BlockKind::Cycle {
                    (0..flow.blocks.len())
                        .map(|block| {
                            flow.repeat_reaches(cycle_index, block, |first, second| {
                                runs[first]
                                    .iter()
                                    .zip(&runs[second])
                                    .any(|(a, b)| a & b != 0)
                            })
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            })
            .collect();
        Self { reaching }
    }
}

/// The boundary reached by one finite structural execution summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionOutcome {
    Return { block_index: usize },
    Repeat { cycle_index: usize },
}

impl Execution {
    /// Reports whether an authored block runs in this execution.
    #[must_use]
    pub fn participates(&self, block: usize) -> bool {
        self.blocks.binary_search(&block).is_ok()
    }

    /// The selected output, or `None` if the block does not branch or participate.
    #[must_use]
    pub fn selected(&self, block: usize) -> Option<usize> {
        self.branches
            .iter()
            .find(|selection| selection.block == block)
            .map(|selection| selection.branch)
    }
}

/// A dependency-derived shared continuation of one question or choice.
/// This projection does not prescribe the scopes or joins used by lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConvergenceGroup {
    pub branching_block: usize,
    /// Question-output or choice-case positions, in authored order.
    pub branches: Vec<usize>,
    /// The shared continuation: the authored blocks whose branch set is
    /// exactly `branches`, in authored order. The end block never belongs.
    pub continuation: Vec<usize>,
}

/// One implicit convergence point for a logical wire. Repeated output names
/// always join here before any consumer captures the wire. This junction is
/// neither an authored block nor a new producer occurrence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireMerge {
    pub wire: Ident,
    pub producers: Vec<ProducerId>,
    /// Branch-local blocks, including producers, that must finish before the merge.
    pub before: Vec<usize>,
    /// Its consumers, in source order. Every block in `before` is declared
    /// above every one of them.
    pub after: Vec<usize>,
}

/// Verified serial lowering plan. Its joins are distinct from semantic
/// [`ConvergenceGroup`] records.
#[derive(Clone)]
pub enum ExecutionPlan {
    Cycle {
        index: usize,
        body: Box<ExecutionPlan>,
        /// What runs once the cycle completes: one continuation per declared
        /// output when it declares several, otherwise the one continuation of
        /// its result, or none when it never completes.
        branches: Vec<Branch>,
        /// The shared continuations of several outputs, as for a choice.
        joins: Vec<Join>,
    },
    /// A boundary consumer exporting one declared output of an active cycle.
    Export { index: usize, target: usize },
    /// An authored completion of the root flow.
    Return { index: usize },
    /// An authored repetition of the directly containing cycle.
    Continue { index: usize },
    Action {
        index: usize,
        next: Box<ExecutionPlan>,
    },
    Call {
        index: usize,
        next: Box<ExecutionPlan>,
    },
    Question {
        index: usize,
        branches: Vec<Branch>,
        /// Successive joins from the narrowest execution context outward.
        joins: Vec<Join>,
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
pub type Branch = Box<ExecutionPlan>;

/// Wire bindings and continuation shared by branches yielding into a lowering join.
#[derive(Clone)]
pub struct Join {
    /// Output positions yielding here, in authored order. The same position may
    /// also yield to an outer join; codegen must use each yield's target.
    pub branches: Vec<usize>,
    /// Logical wire names, ordered by their first authored producer.
    pub wires: Vec<Ident>,
    pub next: Box<ExecutionPlan>,
}

#[cfg(test)]
mod tests {
    use syn::{ItemFn, parse_quote};

    /// The callee of the first block of a one-call flow.
    fn callee(body: &ItemFn) -> String {
        crate::parse::flow(body).expect("the flow parses").blocks[0].callee()
    }

    #[test]
    fn a_callee_reads_as_the_author_spelled_it() {
        assert_eq!(
            callee(&parse_quote! {
                fn probe(left: i32, right: i32) -> i32 {
                    #[call("Subtract.")]
                    let end = |left, right| math::difference(left, right);

                    |end| return end;
                }
            }),
            "math::difference"
        );
    }

    #[test]
    fn a_callee_keeps_its_generic_arguments() {
        assert_eq!(
            callee(&parse_quote! {
                fn probe(text: &str) -> u32 {
                    #[call]
                    let end = |text| str::parse::<u32>(text);

                    |end| return end;
                }
            }),
            "str::parse::<u32>"
        );
        assert_eq!(
            callee(&parse_quote! {
                fn probe() -> Map {
                    #[call]
                    let end = || HashMap::<String, u32>::new();

                    |end| return end;
                }
            }),
            "HashMap::<String, u32>::new"
        );
        assert_eq!(
            callee(&parse_quote! {
                fn probe(f: F) -> u32 {
                    #[call]
                    let end = |f| <F as ::core::ops::Fn<(u32,)>>::call(f);

                    |end| return end;
                }
            }),
            "<F as ::core::ops::Fn<(u32,)>>::call"
        );
    }

    #[test]
    fn a_callee_keeps_the_words_of_a_qualified_self_type_apart() {
        assert_eq!(
            callee(&parse_quote! {
                fn probe(text: &str) -> u32 {
                    #[call]
                    let end = |text| <u32 as FromStr>::from_str(text);

                    |end| return end;
                }
            }),
            "<u32 as FromStr>::from_str"
        );
        assert_eq!(
            callee(&parse_quote! {
                fn probe(bytes: Vec<u8>) -> &'static [u8] {
                    #[call]
                    let end = |bytes| <Vec<u8> as AsRef<[u8]>>::as_ref(bytes);

                    |end| return end;
                }
            }),
            "<Vec<u8> as AsRef<[u8]>>::as_ref"
        );
        assert_eq!(
            callee(&parse_quote! {
                fn probe(text: &str) -> u32 {
                    #[call]
                    let end = |text| <u32 as ::core::str::FromStr>::from_str(text);

                    |end| return end;
                }
            }),
            "<u32 as ::core::str::FromStr>::from_str"
        );
    }

    #[test]
    fn a_comment_inside_a_callee_stays_out_of_its_name() {
        assert_eq!(
            callee(
                &syn::parse_str(
                    "fn probe(a: u32) -> u32 {
                        #[call]
                        let end = |a| math:: /* sneaky */ twice(a);

                        |end| return end;
                    }"
                )
                .expect("the flow parses")
            ),
            "math::twice"
        );
    }
}
