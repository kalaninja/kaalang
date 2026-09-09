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
    if let Some(asyncness) = &function.sig.asyncness {
        return Err(Error::new(
            asyncness.span(),
            "kaalang 0.1 does not support async flows",
        ));
    }
    Ok(Flow {
        flow_inputs: flow_inputs(function)?,
        blocks: blocks(function)?,
    })
}

/// Extracts identifier flow parameters, preserving mutability in the signature.
fn flow_inputs(function: &ItemFn) -> Result<Vec<Ident>> {
    function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Typed(argument) => match argument.pat.as_ref() {
                Pat::Wild(wildcard) if wildcard.attrs.is_empty() => None,
                pattern => Some(
                    simple_binding(pattern, "kaalang flow parameters", true)
                        .map(|input| input.unraw()),
                ),
            },
            FnArg::Receiver(receiver) => Some(Err(Error::new(
                receiver.span(),
                "#[kaalang] is supported only on free functions",
            ))),
        })
        .collect()
}

/// Parses every function-body statement as one kaalang block, then appends the
/// implicit end block that captures the flow's `result` wire.
fn blocks(function: &ItemFn) -> Result<Vec<Block>> {
    let mut blocks = function
        .block
        .stmts
        .iter()
        .map(parse_block)
        .collect::<Result<Vec<_>>>()?;
    blocks.push(end::block(function));

    Ok(blocks)
}

/// Parses one closure-shaped statement and hands it to its kind's parser.
fn parse_block(statement: &Stmt) -> Result<Block> {
    let closure = block_statement(statement)?;
    let (kind, kind_attribute, companions) =
        block_kind(&closure.attrs, closure.inputs_begin.span())?;
    let (inputs, body) = block_closure(closure)?;
    let (outputs, output_span) = block_outputs(closure)?;
    reject_control_transfers(&body)?;
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
        BlockKind::End => unreachable!("the end block is implicit, never parsed"),
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
    pub(crate) fn accept_companions(&self, accepted: &[&str]) -> Result<Vec<&'a Attribute>> {
        self.companions
            .iter()
            .map(|companion| {
                if accepted.iter().any(|name| companion.path().is_ident(name)) {
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
            question_branches: Vec::new(),
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
                return Err(if attribute.path().is_ident("case") {
                    choice::case_before_choice(attribute)
                } else {
                    question::answer_before_question(attribute)
                });
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
        Some("end") => return Err(end::authored(attribute.span())),
        Some("case" | "yes" | "no") => Role::Companion,
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

    // With the output arrow optional, an unbraced body would otherwise parse.
    let body = closure.body.as_ref();
    if !matches!(body, Expr::Block(block) if block.attrs.is_empty() && block.label.is_none()) {
        return Err(Error::new_spanned(
            body,
            "a kaalang block body must be a braced block",
        ));
    }

    let inputs = closure
        .inputs
        .iter()
        .map(block_input)
        .collect::<Result<_>>()?;
    Ok((inputs, body.clone()))
}

/// Parses one consuming or borrowing block input.
fn block_input(pattern: &Pat) -> Result<Input> {
    match pattern {
        Pat::Ident(binding) => Ok(input(
            simple_binding(pattern, "kaalang block inputs", true)?,
            false,
            binding.mutability.is_some(),
        )),
        Pat::Reference(reference) if reference.attrs.is_empty() => Ok(input(
            simple_binding(&reference.pat, "kaalang block inputs", false)?,
            true,
            reference.mutability.is_some(),
        )),
        _ => Err(Error::new_spanned(
            pattern,
            "kaalang block inputs must contain only `name`, `mut name`, `&name`, or `&mut name`",
        )),
    }
}

/// Pairs one authored input spelling with the logical wire it names.
fn input(alias: Ident, borrowed: bool, mutable: bool) -> Input {
    Input {
        borrowed,
        mutable,
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
/// An omitted arrow and `-> ()` both declare no wires. The span they report is
/// the closing bar and the empty tuple respectively, so a kind that requires
/// outputs still points at the declaration.
fn block_outputs(closure: &ExprClosure) -> Result<(Vec<Ident>, Span)> {
    let ReturnType::Type(_, output) = &closure.output else {
        return Ok((Vec::new(), closure.inputs_end.span()));
    };

    let outputs = match output.as_ref() {
        Type::Tuple(tuple) if tuple.elems.is_empty() => Vec::new(),
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

/// Extracts an identifier binding, optionally permitting an authored `mut`.
fn simple_binding(pattern: &Pat, subject: &str, allow_mut: bool) -> Result<Ident> {
    match pattern {
        Pat::Ident(binding)
            if binding.attrs.is_empty()
                && binding.by_ref.is_none()
                && (allow_mut || binding.mutability.is_none())
                && binding.subpat.is_none() =>
        {
            Ok(binding.ident.clone())
        }
        _ => Err(Error::new_spanned(
            pattern,
            format!("{subject} must use simple identifiers"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use syn::{ItemFn, parse_quote};

    use super::{block_input, flow};
    use crate::model::{BlockKind, RESULT_WIRE};

    #[test]
    fn captures_preserve_borrowing_mutability_and_authored_spelling() {
        for (pattern, borrowed, mutable) in [
            (parse_quote!(r#value), false, false),
            (parse_quote!(mut r#value), false, true),
            (parse_quote!(&r#value), true, false),
            (parse_quote!(&mut r#value), true, true),
        ] {
            let input = block_input(&pattern).expect("one of the four capture forms");
            assert_eq!((input.borrowed, input.mutable), (borrowed, mutable));
            assert_eq!(input.ident, "value");
            assert_eq!(input.alias, "r#value");
        }
    }

    #[test]
    fn mutability_does_not_admit_other_input_patterns() {
        for pattern in [
            parse_quote!(ref mut value),
            parse_quote!(&mut mut value),
            parse_quote!(&mut (value,)),
            parse_quote!(&mut _),
            parse_quote!(&mut &value),
            parse_quote!(mut value @ _),
        ] {
            assert!(block_input(&pattern).is_err());
        }
    }

    #[test]
    fn every_flow_ends_with_an_implicit_block_capturing_the_result_wire() {
        let function: ItemFn = parse_quote! {
            fn double(input: u32) -> u32 {
                #[action("Double the input.")]
                |input| -> result { input * 2 };
            }
        };

        let flow = flow(&function).expect("the flow parses");
        let [action, end] = flow.blocks.as_slice() else {
            panic!("one authored block and the implicit end")
        };
        assert_eq!(action.kind, BlockKind::Action);
        assert_eq!(end.kind, BlockKind::End);
        assert!(end.outputs.is_empty());
        assert!(end.description.is_none());
        let [captured] = end.inputs.as_slice() else {
            panic!("end captures one wire")
        };
        assert!(!captured.borrowed);
        assert!(!captured.mutable);
        assert_eq!(captured.ident, RESULT_WIRE);
    }

    #[test]
    fn an_authored_end_statement_is_rejected() {
        let function: ItemFn = parse_quote! {
            fn double(input: u32) -> u32 {
                #[action("Double the input.")]
                |input| -> result { input * 2 };

                #[end]
                |result| {};
            }
        };

        assert_eq!(
            error(&function),
            "a kaalang flow has no end statement; the block that produces the `result` wire finishes it"
        );
    }

    #[test]
    fn an_action_may_declare_no_outputs_in_either_spelling() {
        for function in [
            parse_quote! {
                fn effects(input: u32) {
                    #[action("Take the input without producing a wire.")]
                    |input| -> () { drop(input) };
                    #[action("Finish.")]
                    || -> result {};
                }
            },
            parse_quote! {
                fn effects(input: u32) {
                    #[action("Take the input without producing a wire.")]
                    |input| { drop(input) };
                    #[action("Finish.")]
                    || -> result {};
                }
            },
        ] {
            let flow: ItemFn = function;
            let flow = super::flow(&flow).expect("an action may declare no outputs");
            assert!(flow.blocks[0].outputs.is_empty());
        }
    }

    #[test]
    fn an_unbraced_body_is_rejected() {
        let function: ItemFn = parse_quote! {
            fn invalid(input: u32) -> u32 {
                #[action("Double the input.")]
                |input| input * 2;
            }
        };

        assert_eq!(
            error(&function),
            "a kaalang block body must be a braced block"
        );
    }

    #[test]
    fn an_empty_output_tuple_is_rejected_for_a_question_and_a_choice() {
        let question: ItemFn = parse_quote! {
            fn invalid(input: u32) -> u32 {
                #[question("Ask without outputs.")]
                |&input| -> () { true };
            }
        };
        let choice: ItemFn = parse_quote! {
            fn invalid(input: u32) -> u32 {
                #[choice("Pick without outputs.")]
                #[case("First.")]
                #[case("Second.")]
                |&input| -> () {
                    match input {
                        0 => (),
                        _ => (),
                    }
                };
            }
        };

        assert_eq!(
            error(&question),
            "a kaalang question must declare exactly two outputs"
        );
        assert_eq!(
            error(&choice),
            "a kaalang choice must declare exactly one output for each case"
        );
    }

    #[test]
    fn question_answers_follow_their_attribute_order() {
        let function: ItemFn = parse_quote! {
            fn decide(input: bool) -> u8 {
                #[question("Decide.")]
                #[no("Use the fallback.")]
                #[yes]
                |input| -> (fallback, proceed) { input };
            }
        };
        let parsed = flow(&function).expect("the answer attributes are valid");
        let branches = &parsed.blocks[0].question_branches;

        assert_eq!(branches.len(), 2);
        assert!(!branches[0].is_yes);
        assert_eq!(
            branches[0].description.as_deref(),
            Some("Use the fallback.")
        );
        assert!(branches[1].is_yes);
        assert!(branches[1].description.is_none());

        let implicit: ItemFn = parse_quote! {
            fn decide(input: bool) -> u8 {
                #[question("Decide.")]
                |input| -> (proceed, fallback) { input };
            }
        };
        let parsed = flow(&implicit).expect("questions keep their implicit yes-no order");
        assert!(parsed.blocks[0].question_branches[0].is_yes);
        assert!(!parsed.blocks[0].question_branches[1].is_yes);
    }

    fn error(function: &ItemFn) -> String {
        match flow(function) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("the flow is rejected"),
        }
    }
}
