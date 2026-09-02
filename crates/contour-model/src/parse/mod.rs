//! Parses a Contour flow and validates each block's local syntax.

use proc_macro2::{Ident, Span};
use syn::{
    Attribute, Error, Expr, ExprClosure, FnArg, ItemFn, LitStr, MacroDelimiter, Meta, Pat, Result,
    ReturnType, Stmt, Type, ext::IdentExt, spanned::Spanned,
};

use crate::model::{Block, BlockKind, Flow, Input};

mod action;
mod choice;
mod end;
mod question;

/// Parses a flow function into its source wires and closure-shaped blocks.
pub(crate) fn flow(function: &ItemFn) -> Result<Flow> {
    Ok(Flow {
        sources: source_wires(function)?,
        blocks: blocks(&function.block.stmts)?,
    })
}

/// Extracts simple flow parameters as source wire names.
fn source_wires(function: &ItemFn) -> Result<Vec<Ident>> {
    function
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Typed(argument) => match argument.pat.as_ref() {
                Pat::Wild(wildcard) if wildcard.attrs.is_empty() => None,
                pattern => Some(
                    simple_binding(pattern, "Contour flow parameters").map(|source| source.unraw()),
                ),
            },
            FnArg::Receiver(receiver) => Some(Err(Error::new(
                receiver.span(),
                "#[contour] is supported only on free functions",
            ))),
        })
        .collect()
}

/// Parses every function-body statement as one Contour block.
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
            "a Contour flow requires exactly one End block",
        )),
        [(index, end)] if *index + 1 != blocks.len() => Err(Error::new(
            end.span,
            "the Contour End block must be the final statement",
        )),
        [_] => Ok(blocks),
        [_, (_, duplicate), ..] => Err(Error::new(
            duplicate.span,
            "a Contour flow must not declare more than one End block",
        )),
    }
}

/// Parses one closure-shaped statement and hands it to its kind's parser.
fn parse_block(statement: &Stmt) -> Result<Block> {
    let closure = block_statement(statement)?;
    let (kind, kind_attribute, companions) =
        block_kind(&closure.attrs, closure.inputs_begin.span())?;
    let (inputs, body) = block_closure(closure, kind)?;
    let (outputs, output_span) = block_outputs(closure, kind)?;
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
        format!("`#[{name}]` is not valid on this kind of Contour block"),
    )
}

/// Unwraps the closure expression every Contour block must be written as.
///
/// Rust already requires the semicolon on every block but the last, where it is
/// optional exactly as it is for any tail expression.
fn block_statement(statement: &Stmt) -> Result<&ExprClosure> {
    let Stmt::Expr(expression, _) = statement else {
        return Err(Error::new_spanned(
            statement,
            "a Contour flow body may contain only attributed block statements",
        ));
    };
    let Expr::Closure(closure) = expression else {
        return Err(Error::new_spanned(
            expression,
            "a Contour block must have the form `|inputs| -> outputs { body }`",
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
                    "a Contour block must declare exactly one kind",
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
            "a Contour block must declare its kind, such as `#[action(\"description\")]`",
        ));
    };

    Ok((kind, attribute, companions))
}

/// What one attribute written on a Contour block means.
enum Role {
    Kind(BlockKind),
    Companion,
    Comment,
}

/// Classifies one attribute written on a Contour block.
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
                "Contour blocks support only comments and Contour attributes",
            ));
        }
    })
}

/// Extracts inputs and the authored body from a block's closure-shaped syntax.
fn block_closure(closure: &ExprClosure, kind: BlockKind) -> Result<(Vec<Input>, Expr)> {
    if closure.lifetimes.is_some()
        || closure.constness.is_some()
        || closure.asyncness.is_some()
        || closure.capture.is_some()
    {
        return Err(Error::new_spanned(
            closure,
            "Contour block statements do not support `move`, `async`, `const`, or lifetime modifiers",
        ));
    }
    if closure.inputs.is_empty() && kind != BlockKind::End {
        return Err(Error::new(
            closure.inputs_end.span(),
            "a Contour block requires at least one input",
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
            simple_binding(pattern, "Contour block inputs")?,
            false,
        )),
        Pat::Reference(reference)
            if reference.attrs.is_empty() && reference.mutability.is_none() =>
        {
            Ok(input(
                simple_binding(&reference.pat, "Contour block inputs")?,
                true,
            ))
        }
        _ => Err(Error::new_spanned(
            pattern,
            "Contour block inputs must contain only `name` or `&name`",
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

/// Parses output wire declarations from the closure return position.
fn block_outputs(closure: &ExprClosure, kind: BlockKind) -> Result<(Vec<Ident>, Span)> {
    if kind == BlockKind::End {
        return Ok((Vec::new(), closure.output.span()));
    }
    let ReturnType::Type(_, output) = &closure.output else {
        return Err(Error::new(
            closure.inputs_end.span(),
            "a Contour block must declare its outputs after `->`",
        ));
    };

    let outputs = match output.as_ref() {
        Type::Tuple(tuple) if !tuple.elems.is_empty() => tuple
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
        "Contour block outputs must contain only identifiers",
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
