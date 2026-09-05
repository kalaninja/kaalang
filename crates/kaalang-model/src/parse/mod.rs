//! Parses a kaalang flow and validates each block's local syntax.

use proc_macro2::{Ident, Span};
use syn::{
    Attribute, Error, Expr, ExprAsync, ExprClosure, ExprReturn, ExprTry, FnArg, Item, ItemFn,
    LitStr, MacroDelimiter, Meta, Pat, Result, ReturnType, Stmt, Type, ext::IdentExt,
    spanned::Spanned, visit::Visit,
};

use crate::model::{Block, BlockKind, Flow, Input};

mod action;
mod choice;
mod end;
mod question;

/// Parses a flow function into its named flow inputs and closure-shaped blocks.
pub(crate) fn flow(function: &ItemFn) -> Result<Flow> {
    Ok(Flow {
        flow_inputs: flow_inputs(function)?,
        blocks: blocks(&function.block.stmts)?,
    })
}

/// Extracts simple flow parameters as named flow inputs.
fn flow_inputs(function: &ItemFn) -> Result<Vec<Ident>> {
    function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Typed(argument) => match argument.pat.as_ref() {
                Pat::Wild(wildcard) if wildcard.attrs.is_empty() => None,
                pattern => Some(
                    simple_binding(pattern, "kaalang flow parameters").map(|input| input.unraw()),
                ),
            },
            FnArg::Receiver(receiver) => Some(Err(Error::new(
                receiver.span(),
                "#[kaalang] is supported only on free functions",
            ))),
        })
        .collect()
}

/// Parses every function-body statement as one kaalang block.
fn blocks(statements: &[Stmt]) -> Result<Vec<Block>> {
    let blocks = statements
        .iter()
        .map(parse_block)
        .collect::<Result<Vec<_>>>()?;
    let ends = blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.kind == BlockKind::End)
        .collect::<Vec<_>>();

    match ends.as_slice() {
        [] => Err(Error::new(
            Span::call_site(),
            "a kaalang flow requires exactly one End block",
        )),
        [(index, end)] if *index + 1 != blocks.len() => Err(Error::new(
            end.span,
            "the kaalang End block must be the final statement",
        )),
        [_] => Ok(blocks),
        [_, (_, duplicate), ..] => Err(Error::new(
            duplicate.span,
            "a kaalang flow must not declare more than one End block",
        )),
    }
}

/// Parses one closure-shaped statement and hands it to its kind's parser.
fn parse_block(statement: &Stmt) -> Result<Block> {
    let closure = block_statement(statement)?;
    let (kind, kind_attribute, companions) =
        block_kind(&closure.attrs, closure.inputs_begin.span())?;
    let (inputs, body) = block_closure(closure)?;
    let (outputs, output_span) = block_outputs(closure, kind)?;
    if kind != BlockKind::End {
        reject_control_transfers(&body)?;
    }
    let syntax = BlockSyntax {
        kind,
        closure,
        kind_attribute,
        companions,
        inputs,
        outputs,
        output_span,
        body,
    };

    match kind {
        BlockKind::Action => action::parse(syntax),
        BlockKind::Question => question::parse(syntax),
        BlockKind::Choice => choice::parse(syntax),
        BlockKind::End => end::parse(syntax),
    }
}

/// One block's closure parts, before its kind decides which rules apply.
pub(crate) struct BlockSyntax<'a> {
    pub(crate) kind: BlockKind,
    pub(crate) closure: &'a ExprClosure,
    pub(crate) kind_attribute: &'a Attribute,
    /// Attributes that accompany the one declaring the kind, such as `#[case]`.
    pub(crate) companions: Vec<&'a Attribute>,
    pub(crate) inputs: Vec<Input>,
    pub(crate) outputs: Vec<Ident>,
    pub(crate) output_span: Span,
    pub(crate) body: Expr,
}

impl<'a> BlockSyntax<'a> {
    /// Returns the companion attributes this kind accepts, rejecting any other.
    pub(crate) fn accept_companions(&self, accepted: &str) -> Result<Vec<&'a Attribute>> {
        self.companions
            .iter()
            .map(|companion| {
                if companion.path().is_ident(accepted) {
                    Ok(*companion)
                } else {
                    Err(unexpected_companion(companion))
                }
            })
            .collect()
    }

    /// Rejects every companion attribute, for a kind that accepts none.
    pub(crate) fn reject_companions(&self) -> Result<()> {
        match self.companions.first() {
            Some(companion) => Err(unexpected_companion(companion)),
            None => Ok(()),
        }
    }

    /// Rejects an empty input list, for a kind that requires one.
    pub(crate) fn require_inputs(&self, name: &str) -> Result<()> {
        if self.inputs.is_empty() {
            return Err(Error::new(
                self.closure.inputs_end.span(),
                format!("a kaalang {name} requires at least one input"),
            ));
        }
        Ok(())
    }

    /// Turns syntax its own kind has accepted into a flow block.
    pub(crate) fn into_block(
        self,
        description: Option<String>,
        case_descriptions: Vec<String>,
    ) -> Block {
        Block {
            kind: self.kind,
            description,
            case_descriptions,
            outputs: self.outputs,
            output_span: self.output_span,
            inputs: self.inputs,
            body: self.body,
            span: self.kind_attribute.span(),
        }
    }
}

fn unexpected_companion(companion: &Attribute) -> Error {
    let name = companion
        .path()
        .get_ident()
        .expect("companion attributes are single identifiers");

    Error::new_spanned(
        companion,
        format!("`#[{name}]` is not valid on this kind of kaalang block"),
    )
}

/// Unwraps the closure expression every kaalang block must be written as.
///
/// Rust already requires the semicolon on every block but the last, where it is
/// optional exactly as it is for any tail expression.
fn block_statement(statement: &Stmt) -> Result<&ExprClosure> {
    let Stmt::Expr(expression, _) = statement else {
        return Err(Error::new_spanned(
            statement,
            "a kaalang flow body may contain only attributed block statements",
        ));
    };
    let Expr::Closure(closure) = expression else {
        return Err(Error::new_spanned(
            expression,
            "a kaalang block must have the form `|inputs| -> outputs { body }`",
        ));
    };
    Ok(closure)
}

/// Determines which kind a block declares, that it declares exactly one, and
/// which attributes accompany it.
fn block_kind(
    attributes: &[Attribute],
    fallback_span: Span,
) -> Result<(BlockKind, &Attribute, Vec<&Attribute>)> {
    let mut declared = None;
    let mut companions = Vec::new();

    for attribute in attributes {
        match attribute_role(attribute)? {
            Role::Kind(_) if declared.is_some() => {
                return Err(Error::new_spanned(
                    attribute,
                    "a kaalang block must declare exactly one kind",
                ));
            }
            Role::Kind(kind) => declared = Some((kind, attribute)),
            Role::Companion if declared.is_none() => {
                return Err(choice::case_before_choice(attribute));
            }
            Role::Companion => companions.push(attribute),
            Role::Comment => {}
        }
    }

    let Some((kind, attribute)) = declared else {
        return Err(Error::new(
            fallback_span,
            "a kaalang block must declare its kind, such as `#[action(\"description\")]`",
        ));
    };

    Ok((kind, attribute, companions))
}

/// What one attribute written on a kaalang block means.
enum Role {
    Kind(BlockKind),
    Companion,
    Comment,
}

/// Classifies one attribute written on a kaalang block.
fn attribute_role(attribute: &Attribute) -> Result<Role> {
    let name = attribute.path().get_ident().map(ToString::to_string);

    Ok(match name.as_deref() {
        Some("action") => Role::Kind(BlockKind::Action),
        Some("question") => Role::Kind(BlockKind::Question),
        Some("choice") => Role::Kind(BlockKind::Choice),
        Some("end") => Role::Kind(BlockKind::End),
        Some("case") => Role::Companion,
        Some("doc") => Role::Comment,
        _ => {
            return Err(Error::new_spanned(
                attribute,
                "kaalang blocks support only comments and kaalang attributes",
            ));
        }
    })
}

/// Extracts inputs and the authored body from a block's closure-shaped syntax.
fn block_closure(closure: &ExprClosure) -> Result<(Vec<Input>, Expr)> {
    if closure.lifetimes.is_some()
        || closure.constness.is_some()
        || closure.asyncness.is_some()
        || closure.capture.is_some()
    {
        return Err(Error::new_spanned(
            closure,
            "kaalang block statements do not support `move`, `async`, `const`, or lifetime modifiers",
        ));
    }

    let inputs = closure
        .inputs
        .iter()
        .map(block_input)
        .collect::<Result<_>>()?;
    Ok((inputs, closure.body.as_ref().clone()))
}

/// Parses one consuming or borrowing block input.
fn block_input(pattern: &Pat) -> Result<Input> {
    match pattern {
        Pat::Ident(_) => Ok(input(
            simple_binding(pattern, "kaalang block inputs")?,
            false,
        )),
        Pat::Reference(reference)
            if reference.attrs.is_empty() && reference.mutability.is_none() =>
        {
            Ok(input(
                simple_binding(&reference.pat, "kaalang block inputs")?,
                true,
            ))
        }
        _ => Err(Error::new_spanned(
            pattern,
            "kaalang block inputs must contain only `name` or `&name`",
        )),
    }
}

/// Pairs one authored input spelling with the logical wire it names.
fn input(alias: Ident, borrowed: bool) -> Input {
    Input {
        borrowed,
        ident: alias.unraw(),
        alias,
    }
}

/// Rejects a `return` expression or `?` operator in the body's own control-flow
/// scope. A nested closure, async block, or item owns its control flow, and macro
/// token streams are opaque, so the walk stops at each of those.
fn reject_control_transfers(body: &Expr) -> Result<()> {
    struct FirstTransfer(Option<Error>);

    impl<'ast> Visit<'ast> for FirstTransfer {
        fn visit_expr_return(&mut self, expression: &'ast ExprReturn) {
            self.0.get_or_insert_with(|| {
                Error::new_spanned(
                    expression,
                    "a kaalang block body must not use a `return` expression",
                )
            });
        }

        fn visit_expr_try(&mut self, expression: &'ast ExprTry) {
            self.0.get_or_insert_with(|| {
                Error::new_spanned(
                    expression,
                    "a kaalang block body must not use the `?` operator",
                )
            });
        }

        fn visit_expr_closure(&mut self, _: &'ast ExprClosure) {}

        fn visit_expr_async(&mut self, _: &'ast ExprAsync) {}

        fn visit_item(&mut self, _: &'ast Item) {}
    }

    let mut first = FirstTransfer(None);
    first.visit_expr(body);
    first.0.map_or(Ok(()), Err)
}

/// Parses output wire declarations from the closure return position.
///
/// The empty tuple `()` declares zero outputs.
fn block_outputs(closure: &ExprClosure, kind: BlockKind) -> Result<(Vec<Ident>, Span)> {
    if kind == BlockKind::End {
        return Ok((Vec::new(), closure.output.span()));
    }
    let ReturnType::Type(_, output) = &closure.output else {
        return Err(Error::new(
            closure.inputs_end.span(),
            "a kaalang block must declare its outputs after `->`",
        ));
    };

    let outputs = match output.as_ref() {
        Type::Tuple(tuple) => tuple
            .elems
            .iter()
            .map(output_ident)
            .collect::<Result<Vec<_>>>()?,
        output => vec![output_ident(output)?],
    };
    Ok((outputs, output.span()))
}

/// Reinterprets a simple Rust type path as an output wire name.
fn output_ident(output: &Type) -> Result<Ident> {
    let Type::Path(path) = output else {
        return Err(unexpected_output(output));
    };
    if path.qself.is_some() {
        return Err(unexpected_output(output));
    }

    path.path
        .get_ident()
        .map(IdentExt::unraw)
        .ok_or_else(|| unexpected_output(output))
}

fn unexpected_output(output: &Type) -> Error {
    Error::new_spanned(
        output,
        "kaalang block outputs must contain only identifiers",
    )
}

/// Extracts a parenthesized, nonempty description.
pub(crate) fn description(attribute: &Attribute, subject: &str) -> Result<String> {
    let Meta::List(description) = &attribute.meta else {
        return Err(Error::new_spanned(
            attribute,
            format!("a {subject} attribute requires a parenthesized description"),
        ));
    };
    if !matches!(description.delimiter, MacroDelimiter::Paren(_)) {
        return Err(Error::new_spanned(
            attribute,
            format!("a {subject} description must use parentheses"),
        ));
    }
    let description = attribute.parse_args::<LitStr>().map_err(|_| {
        Error::new_spanned(
            attribute,
            format!("a {subject} description must be a string literal"),
        )
    })?;
    if description.value().trim().is_empty() {
        return Err(Error::new_spanned(
            description,
            format!("a {subject} requires a non-empty description"),
        ));
    }

    Ok(description.value())
}

/// Extracts an unmodified identifier binding, as authored, from a Rust pattern.
fn simple_binding(pattern: &Pat, subject: &str) -> Result<Ident> {
    let Pat::Ident(binding) = pattern else {
        return Err(Error::new_spanned(
            pattern,
            format!("{subject} must use simple identifiers"),
        ));
    };
    if !binding.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
    {
        return Err(Error::new_spanned(
            pattern,
            format!("{subject} must use simple identifiers"),
        ));
    }

    Ok(binding.ident.clone())
}

#[cfg(test)]
mod tests {
    use syn::{ItemFn, parse_quote};

    use super::flow;

    #[test]
    fn empty_output_tuple_declares_zero_outputs() {
        let function: ItemFn = parse_quote! {
            fn effects(input: u32) {
                #[action("Log flow entry.")]
                || -> () { println!("start") };

                #[action("Consume the input.")]
                |input| -> () { drop(input) };

                #[end]
                || {};
            }
        };

        let flow = flow(&function).expect("zero-output actions parse");
        assert!(flow.blocks[0].inputs.is_empty());
        assert!(flow.blocks[0].outputs.is_empty());
        assert_eq!(flow.blocks[1].inputs.len(), 1);
        assert!(flow.blocks[1].outputs.is_empty());
    }

    #[test]
    fn empty_output_tuple_on_a_question_reaches_output_count_validation() {
        let function: ItemFn = parse_quote! {
            fn invalid(input: u32) -> u32 {
                #[question("Ask without outputs.")]
                |&input| -> () { true };

                #[end]
                |input| {};
            }
        };

        let Err(error) = flow(&function) else {
            panic!("a question with `-> ()` is rejected")
        };
        assert_eq!(
            error.to_string(),
            "a kaalang question must declare exactly two outputs"
        );
    }
}
