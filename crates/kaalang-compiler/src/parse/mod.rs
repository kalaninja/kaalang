//! Parses a kaalang flow and validates each block's local syntax.

use proc_macro2::{Ident, Span};
use syn::{
    Attribute, Error, Expr, ExprAsync, ExprClosure, ExprReturn, ExprTry, FnArg, Item, ItemFn,
    LitStr, MacroDelimiter, Meta, Pat, Receiver, ReceiverKind, Result, ReturnType, Stmt, Type,
    ext::IdentExt,
    parse_quote_spanned,
    spanned::Spanned,
    visit::{self, Visit},
};

use crate::model::{Block, BlockKind, Flow, Input};

mod action;
mod break_block;
mod call;
mod choice;
mod end;
mod loop_block;
mod question;
mod return_block;

/// Parses a flow function into its named flow inputs and blocks.
pub(crate) fn flow(function: &ItemFn) -> Result<Flow> {
    if let Some(asyncness) = &function.sig.asyncness {
        return Err(Error::new(
            asyncness.span(),
            "kaalang 0.1 does not support async flows",
        ));
    }
    let flow = Flow {
        flow_inputs: flow_inputs(function)?,
        blocks: blocks(function)?,
    };
    receiver_captures(&flow, receiver(function))?;

    Ok(flow)
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
            // A receiver is a flow input like any other, under the one name
            // Rust gives it. Its wire is the receiver itself, so no capture
            // rebinds it and no spelling other than the authored one is legal.
            FnArg::Receiver(receiver) => Some(Ok(Ident::new("self", receiver.self_token.span()))),
        })
        .collect()
}

/// The receiver this flow declares, if it declares one.
fn receiver(function: &ItemFn) -> Option<&Receiver> {
    function
        .sig
        .inputs
        .iter()
        .find_map(|argument| match argument {
            FnArg::Receiver(receiver) => Some(receiver),
            FnArg::Typed(_) => None,
        })
}

/// How a capture of the receiver is spelled, and the wire it names: the
/// receiver's own spelling. A receiver taken by reference borrows, however it
/// is spelled; every other receiver, including `mut self` and `self: Box<Self>`,
/// is a value like `mut name: T` is.
fn receiver_capture(receiver: &Receiver) -> (&'static str, bool, bool) {
    let borrows = match &receiver.kind {
        ReceiverKind::Reference(_, _, mutability) => Some(mutability.is_some()),
        ReceiverKind::Typed(_, ty) => match ty.as_ref() {
            Type::Reference(reference) => Some(reference.mutability.is_some()),
            _ => None,
        },
        _ => None,
    };
    match borrows {
        Some(true) => ("&mut self", true, true),
        Some(false) => ("&self", true, false),
        None => ("self", false, false),
    }
}

/// Checks every use of the receiver. A capture names a receiver and is spelled
/// the way the signature declares it; a body that reads `self` captures it.
///
/// The receiver binds itself, so a capture cannot borrow or move it into
/// anything else and nothing can put the name out of a body's reach. This walk
/// is what keeps an uncaptured read off the diagram from compiling.
fn receiver_captures(flow: &Flow, receiver: Option<&Receiver>) -> Result<()> {
    for block in &flow.blocks {
        for capture in block.inputs.iter().filter(|input| input.ident == "self") {
            let Some(receiver) = receiver else {
                return Err(Error::new(
                    capture.alias.span(),
                    "`self` names the receiver of a kaalang method, and this flow declares none",
                ));
            };
            let (spelling, borrowed, mutable) = receiver_capture(receiver);
            if (capture.borrowed, capture.mutable) != (borrowed, mutable) {
                return Err(Error::new(
                    capture.alias.span(),
                    format!(
                        "a kaalang capture of the receiver is spelled `{spelling}`, as the signature declares it"
                    ),
                ));
            }
        }
        // A cycle's body holds the statements that parse into their own blocks,
        // and each of those is checked in turn.
        if block.kind == BlockKind::Loop || block.inputs.iter().any(|input| input.ident == "self") {
            continue;
        }
        if let Some(span) = receiver_use(&block.body) {
            return Err(Error::new(
                span,
                match receiver {
                    Some(_) => "a kaalang block body that reads `self` must capture the receiver",
                    None => {
                        "`self` names the receiver of a kaalang method, and this flow declares none"
                    }
                },
            ));
        }
    }

    Ok(())
}

/// Where a body first names the receiver, if it does. A nested item owns no
/// receiver and macro tokens are opaque, so the walk reaches neither.
fn receiver_use(body: &Expr) -> Option<Span> {
    #[derive(Default)]
    struct FirstUse(Option<Span>);

    impl<'ast> Visit<'ast> for FirstUse {
        fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
            if self.0.is_none() && path.qself.is_none() && path.path.is_ident("self") {
                self.0 = Some(path.path.span());
            }
        }

        fn visit_item(&mut self, _: &'ast Item) {}
    }

    let mut first = FirstUse::default();
    first.visit_expr(body);
    first.0
}

/// Parses every function-body statement as one kaalang block, then appends the
/// implicit completion boundary.
fn blocks(function: &ItemFn) -> Result<Vec<Block>> {
    let mut blocks = Vec::new();
    statements(&function.block.stmts, None, &mut blocks)?;
    if let Some(second) = blocks
        .iter()
        .filter(|block| block.kind == BlockKind::Return)
        .nth(1)
    {
        return Err(Error::new(
            second.span,
            "a kaalang flow may declare at most one structural `return`",
        ));
    }
    blocks.push(end::block(function));

    Ok(blocks)
}

/// Flattens lexical cycle regions without changing their authored order.
fn statements(statements: &[Stmt], parent: Option<usize>, blocks: &mut Vec<Block>) -> Result<()> {
    for statement in statements {
        let mut inputs = Vec::new();
        let mut normalized = None;
        let expression = if let Stmt::Expr(expression, _) = statement {
            if let Expr::Closure(closure) = expression
                && closure.attrs.is_empty()
            {
                let (captures, body) = block_closure(closure)?;
                let body = structural_expression(&body);
                if matches!(
                    body,
                    Expr::Loop(_) | Expr::Break(_) | Expr::Return(_) | Expr::Continue(_)
                ) {
                    inputs = captures;
                    normalized = Some(body.clone());
                }
            }
            Some(normalized.as_ref().unwrap_or(expression))
        } else {
            None
        };
        let mut block = match expression {
            Some(Expr::Loop(expression)) => return Err(loop_block::legacy(expression)),
            Some(Expr::Break(expression)) => {
                break_block::parse(expression, inputs, parent, blocks)?
            }
            Some(Expr::Return(expression)) => return_block::parse(expression, inputs, parent)?,
            Some(Expr::Continue(expression)) => {
                return Err(Error::new_spanned(
                    expression,
                    "kaalang cycles repeat implicitly and do not support authored `continue`",
                ));
            }
            Some(Expr::While(expression)) => {
                return Err(Error::new_spanned(
                    expression,
                    "kaalang does not support structural `while`; use a `#[cycle(\"description\")]` block with a question and `break`",
                ));
            }
            _ => parse_block(statement)?,
        };
        // Every kaalang block statement ends the same way, so no shape has to
        // be read twice to know where it stops. A `let` gets its semicolon
        // from Rust; every other spelling is checked here.
        if matches!(statement, Stmt::Expr(_, None)) {
            return Err(Error::new_spanned(
                statement,
                format!(
                    "a kaalang {} requires a trailing semicolon",
                    noun(block.kind)
                ),
            ));
        }
        block.parent = parent;
        let index = blocks.len();
        blocks.push(block);
        if blocks[index].kind == BlockKind::Loop {
            let Expr::Block(body) = &blocks[index].body else {
                unreachable!("a cycle body is normalized to a block")
            };
            let statements = body.block.stmts.clone();
            self::statements(&statements, Some(index), blocks)?;
            blocks[index].loop_end = Some(blocks.len());
        }
    }
    Ok(())
}

/// What a diagnostic calls one block kind.
fn noun(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Action => "action",
        BlockKind::Call => "call",
        BlockKind::Question => "question",
        BlockKind::Choice => "choice",
        BlockKind::Loop => "cycle",
        BlockKind::Break => "break",
        BlockKind::Return => "return",
        BlockKind::End => unreachable!("the end block is implicit"),
    }
}

/// Peels the invisible group a `macro_rules!` substitution arrives in, so a
/// flow another macro wrote is read the way its author spelled it.
pub(crate) fn ungrouped(mut expression: &Expr) -> &Expr {
    while let Expr::Group(group) = expression {
        expression = &group.expr;
    }
    expression
}

/// Braces around a single structural expression do not change its meaning.
fn structural_expression(mut expression: &Expr) -> &Expr {
    while let Expr::Block(block) = expression {
        if !block.attrs.is_empty() || block.label.is_some() {
            break;
        }
        let [Stmt::Expr(inner, _)] = block.block.stmts.as_slice() else {
            break;
        };
        expression = inner;
    }
    expression
}

fn structural_block(kind: BlockKind, span: Span, inputs: Vec<Input>) -> Block {
    Block {
        kind,
        description: None,
        question_branches: Vec::new(),
        case_descriptions: Vec::new(),
        outputs: Vec::new(),
        output_pattern: syn::parse_quote!(()),
        output_span: span,
        inputs,
        body: syn::parse_quote!({}),
        span,
        parent: None,
        loop_end: None,
        break_target: None,
    }
}

/// Parses one statement and hands it to its kind's parser.
fn parse_block(statement: &Stmt) -> Result<Block> {
    let (attributes, output_pattern, closure) = block_statement(statement)?;
    let (kind, kind_attribute, companions) = block_kind(attributes, statement.span())?;
    let (inputs, body) = match closure {
        Some(closure) => block_closure(closure)?,
        None => (Vec::new(), bare_block_body(statement, kind)?),
    };
    let outputs = block_outputs(&output_pattern)?;
    let output_span = output_pattern.span();
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

    if kind != BlockKind::Loop {
        reject_control_transfers(&syntax.body)?;
    }
    match kind {
        BlockKind::Action => action::parse(syntax),
        BlockKind::Call => call::parse(syntax),
        BlockKind::Question => question::parse(syntax),
        BlockKind::Choice => choice::parse(syntax),
        BlockKind::Loop => loop_block::parse(syntax),
        BlockKind::End | BlockKind::Break | BlockKind::Return => {
            unreachable!("structural blocks parse separately")
        }
    }
}

/// One block's syntax, before its kind decides which rules apply.
pub(crate) struct BlockSyntax<'a> {
    pub(crate) kind: BlockKind,
    pub(crate) closure: Option<&'a ExprClosure>,
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
                self.closure
                    .map_or_else(|| self.body.span(), |closure| closure.inputs_end.span()),
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
            parent: None,
            loop_end: None,
            break_target: None,
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

/// Extracts a block's attributes, interfaces, and body. An expression statement
/// declares no outputs; a bare block or bare application additionally declares
/// no inputs.
fn block_statement(statement: &Stmt) -> Result<(&[Attribute], Pat, Option<&ExprClosure>)> {
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
            // A call that captures nothing writes its application alone, with
            // or without outputs; there is no capture list to delimit.
            if matches!(ungrouped(initializer.expr.as_ref()), Expr::Call(_)) {
                return Ok((local.attrs.as_slice(), local.pat.clone(), None));
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
                Some(closure),
            ));
        }
        Stmt::Expr(Expr::Block(block), _) => {
            return Ok((&block.attrs, parse_quote_spanned!(block.span()=> ()), None));
        }
        // A call's body is one application, so it needs no braces to delimit
        // it. The attributes decide: an unattributed application is ordinary
        // Rust, which a flow body does not accept, and it must keep reporting
        // that through the arm below.
        Stmt::Expr(Expr::Call(call), _) if !call.attrs.is_empty() => {
            return Ok((&call.attrs, parse_quote_spanned!(call.span()=> ()), None));
        }
        // The same statement written by another macro, where the substitution
        // carries the attributes and the application sits inside it.
        Stmt::Expr(Expr::Group(group), _)
            if !group.attrs.is_empty() && matches!(ungrouped(&group.expr), Expr::Call(_)) =>
        {
            return Ok((&group.attrs, parse_quote_spanned!(group.span()=> ()), None));
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
    Ok((attributes, pattern, Some(closure)))
}

fn bare_block_body(statement: &Stmt, kind: BlockKind) -> Result<Expr> {
    let application = match statement {
        Stmt::Expr(Expr::Block(block), _) => {
            if block.label.is_some()
                || block
                    .attrs
                    .iter()
                    .any(|attribute| matches!(attribute.style, syn::AttrStyle::Inner(_)))
            {
                return Err(Error::new_spanned(
                    block,
                    "kaalang block bodies do not support attributes or labels",
                ));
            }
            let mut body = block.clone();
            body.attrs.clear();
            if kind == BlockKind::Call {
                return Err(call::braced(&Expr::Block(body)));
            }
            return Ok(Expr::Block(body));
        }
        // A statement carries its attributes on its expression, and the kind
        // attribute was classified before this ran.
        Stmt::Expr(expression, _) if matches!(ungrouped(expression), Expr::Call(_)) => {
            let Expr::Call(call) = ungrouped(expression) else {
                unreachable!("the guard matched an application")
            };
            let mut application = call.clone();
            application.attrs.clear();
            application
        }
        // A `let` carries its own, so whatever sits on the application there
        // is the body's and stays.
        Stmt::Local(local) => {
            let Some(initializer) = &local.init else {
                unreachable!("a bare application body has an initializer")
            };
            let Expr::Call(application) = ungrouped(initializer.expr.as_ref()) else {
                unreachable!("a missing closure denotes a bare application body")
            };
            application.clone()
        }
        _ => unreachable!("a missing closure denotes a bare block or bare application body"),
    };
    if kind != BlockKind::Call {
        return Err(Error::new_spanned(
            &application,
            "only a kaalang call may be written as a bare application; another kind needs a capture list or braces",
        ));
    }
    Ok(parse_quote_spanned!(application.span()=> { #application }))
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
        Some("call") => Role::Kind(BlockKind::Call),
        Some("question") => Role::Kind(BlockKind::Question),
        Some("choice") => Role::Kind(BlockKind::Choice),
        Some("cycle") => Role::Kind(BlockKind::Loop),
        Some("loop" | "r#loop") => return Err(loop_block::legacy_attribute(attribute)),
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
        binding: None,
    }
}

/// Returns the one value a structural transfer carries, after checking that it
/// consists only of bindings introduced by that transfer's capture list.
pub(super) fn transfer_value(
    value: Option<&Expr>,
    span: Span,
    inputs: &[Input],
    kind: &str,
) -> Result<Expr> {
    let value = value
        .cloned()
        .unwrap_or_else(|| parse_quote_spanned!(span=> ()));
    validate_transfer_value(&value, inputs, kind)?;
    Ok(value)
}

fn validate_transfer_value(value: &Expr, inputs: &[Input], kind: &str) -> Result<()> {
    match value {
        Expr::Paren(parenthesized) if parenthesized.attrs.is_empty() => {
            validate_transfer_value(&parenthesized.expr, inputs, kind)
        }
        Expr::Group(group) if group.attrs.is_empty() => {
            validate_transfer_value(&group.expr, inputs, kind)
        }
        Expr::Tuple(tuple) if tuple.attrs.is_empty() && tuple.elems.is_empty() => Ok(()),
        Expr::Tuple(tuple) if tuple.attrs.is_empty() => tuple
            .elems
            .iter()
            .try_for_each(|element| captured_transfer_input(element, inputs, kind)),
        value => captured_transfer_input(value, inputs, kind),
    }
}

fn captured_transfer_input(value: &Expr, inputs: &[Input], kind: &str) -> Result<()> {
    let ident = match value {
        Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none() => path.path.get_ident(),
        _ => None,
    };
    let Some(ident) = ident else {
        return Err(Error::new_spanned(
            value,
            format!(
                "a kaalang {kind} value must be a captured input, a tuple of captured inputs, or `()`"
            ),
        ));
    };
    if inputs
        .iter()
        .any(|input| input.alias.unraw() == ident.unraw())
    {
        Ok(())
    } else {
        Err(Error::new_spanned(
            ident,
            format!("a kaalang {kind} value must name a captured input"),
        ))
    }
}

/// Rejects transfers out of the body's own control-flow
/// scope. A nested closure, async block, or item owns its control flow, and macro
/// token streams are opaque, so the walk stops at each of those.
fn reject_control_transfers(body: &Expr) -> Result<()> {
    #[derive(Default)]
    struct FirstTransfer {
        error: Option<Error>,
        // A label and whether this Rust construct is a loop rather than a block.
        scopes: Vec<(Option<Ident>, bool)>,
    }

    impl<'ast> Visit<'ast> for FirstTransfer {
        fn visit_expr(&mut self, expression: &'ast Expr) {
            let scope = match expression {
                Expr::Loop(loop_) => Some((loop_.label.as_ref(), true)),
                Expr::While(loop_) => Some((loop_.label.as_ref(), true)),
                Expr::ForLoop(loop_) => Some((loop_.label.as_ref(), true)),
                Expr::Block(block) if block.label.is_some() => Some((block.label.as_ref(), false)),
                _ => None,
            };
            if let Some((label, is_loop)) = scope {
                self.scopes
                    .push((label.map(|label| label.name.ident.clone()), is_loop));
            }
            let transfer = match expression {
                Expr::Break(transfer) => Some((transfer.label.as_ref(), false)),
                Expr::Continue(transfer) => Some((transfer.label.as_ref(), true)),
                _ => None,
            };
            if let Some((label, continuing)) = transfer {
                let local = self.scopes.iter().rev().any(|(name, is_loop)| match label {
                    Some(label) => name.as_ref() == Some(&label.ident) && (!continuing || *is_loop),
                    None => *is_loop,
                });
                if !local {
                    self.error.get_or_insert_with(|| Error::new_spanned(expression,
                        "a kaalang block body must not use `break` or `continue` outside its own Rust loops"));
                }
            }
            visit::visit_expr(self, expression);
            if scope.is_some() {
                self.scopes.pop();
            }
        }

        fn visit_expr_return(&mut self, expression: &'ast ExprReturn) {
            self.error.get_or_insert_with(|| {
                Error::new_spanned(
                    expression,
                    "a kaalang block body must not use a `return` expression",
                )
            });
        }

        fn visit_expr_try(&mut self, expression: &'ast ExprTry) {
            self.error.get_or_insert_with(|| {
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

    let mut first = FirstTransfer::default();
    first.visit_expr(body);
    first.error.map_or(Ok(()), Err)
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
    let ident = simple_binding(pattern, "kaalang block outputs", true)?.unraw();
    if ident == "self" {
        return Err(Error::new(
            ident.span(),
            "`self` names the receiver of a kaalang method and cannot name a block output",
        ));
    }

    Ok(ident)
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
    use crate::model::BlockKind;

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
    fn every_flow_ends_with_an_implicit_zero_input_block() {
        let function: ItemFn = parse_quote! {
            fn double(input: u32) -> u32 {
                #[action("Double the input.")]
                let result = |input| { input * 2 };

                |result| return result;
            }
        };

        let flow = flow(&function).expect("the flow parses");
        let [action, return_block, end] = flow.blocks.as_slice() else {
            panic!("two authored blocks and the implicit end")
        };
        assert_eq!(action.kind, BlockKind::Action);
        assert_eq!(return_block.kind, BlockKind::Return);
        assert_eq!(end.kind, BlockKind::End);
        assert!(end.outputs.is_empty());
        assert!(end.description.is_none());
        assert!(end.inputs.is_empty());
    }

    #[test]
    fn a_flow_may_declare_at_most_one_structural_return() {
        let function: ItemFn = parse_quote! {
            fn decide(condition: bool, value: u32) -> u32 {
                #[question("Choose a return.")]
                let (first, second) = |condition| condition;

                |first, value| return value;
                |second, value| return value;
            }
        };

        assert_eq!(
            error(&function),
            "a kaalang flow may declare at most one structural `return`"
        );
    }

    #[test]
    fn an_authored_end_statement_is_rejected() {
        let function: ItemFn = parse_quote! {
            fn double(input: u32) -> u32 {
                #[action("Double the input.")]
                let end = |input| { input * 2 };

                #[end]
                |end| {};
            }
        };

        assert_eq!(
            error(&function),
            "a kaalang flow has no end statement; use a structural `return` to finish it"
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
                    let end = || {};
                }
            },
            parse_quote! {
                fn effects(input: u32) {
                    #[action("Take the input without producing a wire.")]
                    |input| { drop(input) };
                    #[action("Finish.")]
                    let end = || {};
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
                let end = |input| input * 2;
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
                    let end = |input| #body;
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
