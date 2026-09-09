//! Parses a kaalang flow and validates each block's local syntax.

use proc_macro2::{Ident, Span};
use syn::{
    Attribute, Error, Expr, ExprAsync, ExprClosure, ExprReturn, ExprTry, FnArg, Item, ItemFn,
    LitStr, MacroDelimiter, Meta, Pat, Result, ReturnType, Stmt, ext::IdentExt,
    parse_quote_spanned, spanned::Spanned, visit::Visit,
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
    let (attributes, output_pattern, closure) = block_statement(statement)?;
    let (kind, kind_attribute, companions) = block_kind(attributes, statement.span())?;
    let (inputs, body) = block_closure(closure)?;
    let outputs = block_outputs(&output_pattern)?;
    let output_span = output_pattern.span();
    reject_control_transfers(&body)?;
    let syntax = BlockSyntax {
        kind,
        closure,
        kind_attribute,
        companions,
        inputs,
        outputs,
        output_pattern,
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
    pub(crate) output_pattern: Pat,
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
            output_pattern: self.output_pattern,
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

/// Extracts a block's attributes, output pattern, and closure initializer.
/// An expression statement declares an action without outputs.
fn block_statement(statement: &Stmt) -> Result<(&[Attribute], Pat, &ExprClosure)> {
    let (attributes, pattern, expression) = match statement {
        Stmt::Local(local) => {
            let Some(initializer) = &local.init else {
                return Err(Error::new_spanned(
                    local,
                    "a kaalang block requires an initializer",
                ));
            };
            if let Some((_, diverge)) = &initializer.diverge {
                return Err(Error::new_spanned(
                    diverge,
                    "kaalang blocks do not support `let else`",
                ));
            }
            (
                local.attrs.as_slice(),
                local.pat.clone(),
                initializer.expr.as_ref(),
            )
        }
        Stmt::Expr(Expr::Closure(closure), _) => {
            return Ok((
                &closure.attrs,
                parse_quote_spanned!(closure.inputs_end.span()=> ()),
                closure,
            ));
        }
        _ => {
            return Err(Error::new_spanned(
                statement,
                "a kaalang flow body may contain only attributed block statements",
            ));
        }
    };
    let Expr::Closure(closure) = expression else {
        return Err(Error::new_spanned(
            expression,
            "a kaalang block initializer must have the form `|inputs| { body }`",
        ));
    };
    if !closure.attrs.is_empty() {
        return Err(Error::new_spanned(
            expression,
            "kaalang block attributes belong before the statement",
        ));
    }
    Ok((attributes, pattern, closure))
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

    if !matches!(closure.output, ReturnType::Default) {
        return Err(Error::new_spanned(
            &closure.output,
            "kaalang block closures do not support return type annotations",
        ));
    }

    let body = match closure.body.as_ref() {
        Expr::Block(block) if !block.attrs.is_empty() || block.label.is_some() => {
            return Err(Error::new_spanned(
                block,
                "kaalang block bodies do not support attributes or labels",
            ));
        }
        body @ Expr::Block(_) => body.clone(),
        // rustfmt removes braces around a closure's single expression. Keep
        // one body shape for validation and lowering regardless of spelling.
        body => parse_quote_spanned!(body.span()=> { #body }),
    };

    let inputs = closure
        .inputs
        .iter()
        .map(block_input)
        .collect::<Result<_>>()?;
    Ok((inputs, body))
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

/// Parses an identifier or flat tuple of output bindings, retaining the pattern
/// so an action distinguishes binding a whole value from tuple destructuring.
fn block_outputs(pattern: &Pat) -> Result<Vec<Ident>> {
    match pattern {
        Pat::Tuple(tuple) if tuple.attrs.is_empty() => {
            tuple.elems.iter().map(output_ident).collect()
        }
        output => Ok(vec![output_ident(output)?]),
    }
}

fn output_ident(pattern: &Pat) -> Result<Ident> {
    simple_binding(pattern, "kaalang block outputs", true).map(|ident| ident.unraw())
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

    use super::{block_input, block_outputs, flow};
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
    fn outputs_accept_only_plain_or_mutable_identifiers_in_a_flat_pattern() {
        for pattern in [
            parse_quote!(_),
            parse_quote!((value, _)),
            parse_quote!((value, (nested,))),
            parse_quote!(ref value),
            parse_quote!(ref mut value),
            parse_quote!(&value),
            parse_quote!(value @ _),
            parse_quote!([value]),
            parse_quote!(Some(value)),
            parse_quote!(Value { field }),
        ] {
            assert!(block_outputs(&pattern).is_err());
        }
        for pattern in [
            parse_quote!(value),
            parse_quote!(mut r#value),
            parse_quote!((value,)),
            parse_quote!((mut left, right)),
            parse_quote!(()),
        ] {
            assert!(block_outputs(&pattern).is_ok());
        }
    }

    #[test]
    fn every_flow_ends_with_an_implicit_block_capturing_the_result_wire() {
        let function: ItemFn = parse_quote! {
            fn double(input: u32) -> u32 {
                #[action("Double the input.")]
                let result = |input| { input * 2 };
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
                let result = |input| { input * 2 };

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
                    let () = |input| { drop(input) };
                    #[action("Finish.")]
                    let result = || {};
                }
            },
            parse_quote! {
                fn effects(input: u32) {
                    #[action("Take the input without producing a wire.")]
                    |input| { drop(input) };
                    #[action("Finish.")]
                    let result = || {};
                }
            },
        ] {
            let flow: ItemFn = function;
            let flow = super::flow(&flow).expect("an action may declare no outputs");
            assert!(flow.blocks[0].outputs.is_empty());
        }
    }

    #[test]
    fn an_expression_body_is_normalized_to_a_block() {
        let function: ItemFn = parse_quote! {
            fn invalid(input: u32) -> u32 {
                #[action("Double the input.")]
                let result = |input| input * 2;
            }
        };

        let model = flow(&function).expect("expression bodies are accepted");
        assert!(matches!(model.blocks[0].body, syn::Expr::Block(_)));
    }

    #[test]
    fn expression_bodies_keep_the_control_transfer_restrictions() {
        for (body, diagnostic) in [
            (
                parse_quote!(return input),
                "a kaalang block body must not use a `return` expression",
            ),
            (
                parse_quote!(input?),
                "a kaalang block body must not use the `?` operator",
            ),
        ] {
            let body: syn::Expr = body;
            let function = parse_quote! {
                fn invalid(input: Option<u32>) -> u32 {
                    #[action("Attempt a control transfer.")]
                    let result = |input| #body;
                }
            };
            assert_eq!(error(&function), diagnostic);
        }
    }

    #[test]
    fn an_empty_output_tuple_is_rejected_for_a_question_and_a_choice() {
        let question: ItemFn = parse_quote! {
            fn invalid(input: u32) -> u32 {
                #[question("Ask without outputs.")]
                let () = |&input| { true };
            }
        };
        let choice: ItemFn = parse_quote! {
            fn invalid(input: u32) -> u32 {
                #[choice("Pick without outputs.")]
                #[case("First.")]
                #[case("Second.")]
                let () = |&input| {
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
                let (fallback, proceed) = |input| { input };
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
                let (proceed, fallback) = |input| { input };
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
