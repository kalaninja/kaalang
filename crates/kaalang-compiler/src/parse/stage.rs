//! Parses preparation and root stage declarations without unfolding their visits.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Ident, Span};
use syn::ext::IdentExt;
use syn::{Error, Expr, FnArg, ItemFn, Pat, Result, Stmt, parse_quote_spanned, spanned::Spanned};

use super::{
    block_closure, block_outputs, block_statement, description, end, flow_inputs, receiver,
    receiver_captures, statements, structural_block, ungrouped,
};
use crate::model::{Block, BlockKind, Flow, FlowKind, Input};

pub(crate) struct ParsedStaged {
    pub(crate) preparation: Flow,
    pub(crate) stages: Vec<ParsedStage>,
}

pub(crate) struct ParsedStage {
    pub(crate) description: String,
    pub(crate) entry: Ident,
    pub(crate) entry_alias: Ident,
    pub(crate) outputs: Vec<Ident>,
    pub(crate) flow: Flow,
    pub(crate) span: Span,
}

pub(super) fn is_stage(statement: &Stmt) -> bool {
    attributes(statement)
        .is_some_and(|attrs| attrs.iter().any(|attr| attr.path().is_ident("stage")))
}

fn attributes(statement: &Stmt) -> Option<&[syn::Attribute]> {
    match statement {
        Stmt::Local(local) => Some(&local.attrs),
        Stmt::Expr(Expr::Closure(closure), _) => Some(&closure.attrs),
        Stmt::Expr(Expr::Block(block), _) => Some(&block.attrs),
        _ => None,
    }
}

/// Splits a staged function before any executable sequence is resolved.
#[allow(clippy::too_many_lines)] // One pass splits preparation, resolves headers, and builds local bodies.
pub(crate) fn staged(function: &ItemFn) -> Result<Option<ParsedStaged>> {
    let Some(first) = function.block.stmts.iter().position(is_stage) else {
        return Ok(None);
    };
    if let Some(asyncness) = &function.sig.asyncness {
        return Err(Error::new(
            asyncness.span(),
            "kaalang 0.1 does not support async flows",
        ));
    }
    let mut declared = Vec::new();
    let mut names = BTreeSet::new();
    for statement in &function.block.stmts[first..] {
        if !is_stage(statement) {
            return Err(Error::new_spanned(
                statement,
                "only stage declarations may follow the first kaalang stage",
            ));
        }
        let stage = declaration(statement)?;
        if !names.insert(stage.entry.unraw().to_string()) {
            return Err(Error::new(
                stage.entry.span(),
                "duplicate kaalang stage entry",
            ));
        }
        declared.push(stage);
    }
    let targets = declared
        .iter()
        .enumerate()
        .map(|(index, stage)| (stage.entry.unraw().to_string(), index))
        .collect::<BTreeMap<_, _>>();

    let mut preparation = Vec::new();
    statements(&function.block.stmts[..first], None, &mut preparation)?;
    if let Some(block) = preparation
        .iter()
        .find(|block| block.kind == BlockKind::Return)
    {
        return Err(Error::new(
            block.span,
            "a staged kaalang flow returns only from its terminal stage",
        ));
    }
    let parameters = flow_inputs(function)?;
    let direct = parameters
        .iter()
        .cloned()
        .chain(
            preparation
                .iter()
                .filter(|block| block.parent.is_none())
                .flat_map(|block| block.outputs.iter().cloned()),
        )
        .collect::<BTreeSet<_>>();
    let common = direct
        .iter()
        .filter(|name| !names.contains(&name.unraw().to_string()))
        .cloned()
        .collect::<Vec<_>>();
    for (target, stage) in declared.iter().enumerate() {
        if direct.contains(&stage.entry) {
            let alias = preparation
                .iter()
                .rev()
                .find_map(|block| {
                    block
                        .outputs
                        .iter()
                        .position(|name| name == &stage.entry)
                        .map(|position| block.output_binding(position).ident.clone())
                })
                .or_else(|| {
                    function
                        .sig
                        .inputs
                        .iter()
                        .find_map(|argument| match argument {
                            FnArg::Typed(argument) => match argument.pat.as_ref() {
                                Pat::Ident(binding) if binding.ident.unraw() == stage.entry => {
                                    Some(binding.ident.clone())
                                }
                                _ => None,
                            },
                            FnArg::Receiver(_) => None,
                        })
                })
                .unwrap_or_else(|| stage.entry.clone());
            preparation.push(transition(&stage.entry, &alias, target));
        }
    }
    preparation.push(end::block(function));
    let preparation = Flow {
        flow_inputs: parameters,
        blocks: preparation,
        kind: FlowKind::Preparation,
    };
    receiver_captures(&preparation, receiver(function))?;

    let mut returns = 0;
    let mut stages = Vec::new();
    for declaration in declared {
        for output in &declaration.outputs {
            if !names.contains(&output.unraw().to_string()) {
                return Err(Error::new(
                    output.span(),
                    format!("kaalang stage output `{output}` has no matching stage entry"),
                ));
            }
        }
        let mut blocks = Vec::new();
        statements(&declaration.statements, None, &mut blocks)?;
        let authored_returns = blocks
            .iter()
            .filter(|block| block.kind == BlockKind::Return)
            .count();
        returns += authored_returns;
        if returns > 1 || authored_returns > 1 {
            return Err(Error::new(
                declaration.span,
                "a staged kaalang flow may declare at most one structural `return`",
            ));
        }
        if authored_returns > 0 && !declaration.outputs.is_empty() {
            return Err(Error::new(
                declaration.span,
                "a terminal kaalang stage declares no transition outputs",
            ));
        }
        if authored_returns > 0 && stages.len() + 1 != targets.len() {
            return Err(Error::new(
                declaration.span,
                "a terminal kaalang stage must be declared last",
            ));
        }
        if authored_returns == 0 {
            for (output, name) in declaration.outputs.iter().enumerate() {
                if !blocks
                    .iter()
                    .any(|block| block.parent.is_none() && block.outputs.contains(name))
                {
                    return Err(Error::new(
                        name.span(),
                        format!(
                            "a declared kaalang stage output needs a producer named `{name}` in its body"
                        ),
                    ));
                }
                let target = targets[&name.unraw().to_string()];
                blocks.push(transition(
                    name,
                    &declaration.output_bindings[output],
                    target,
                ));
            }
        }
        blocks.push(end::block(function));
        let mut inputs = common.clone();
        inputs.push(declaration.entry.clone());
        let flow = Flow {
            flow_inputs: inputs,
            blocks,
            kind: FlowKind::Stage {
                entry: declaration.entry.clone(),
                self_output: declaration.outputs.contains(&declaration.entry),
            },
        };
        receiver_captures(&flow, receiver(function))?;
        stages.push(ParsedStage {
            description: declaration.description,
            entry: declaration.entry,
            entry_alias: declaration.entry_alias,
            outputs: declaration.outputs,
            flow,
            span: declaration.span,
        });
    }
    Ok(Some(ParsedStaged {
        preparation,
        stages,
    }))
}

struct Declaration {
    description: String,
    entry: Ident,
    entry_alias: Ident,
    outputs: Vec<Ident>,
    output_bindings: Vec<Ident>,
    statements: Vec<Stmt>,
    span: Span,
}

#[allow(clippy::too_many_lines)] // Keeps the header's syntax checks beside its parsed fields.
fn declaration(statement: &Stmt) -> Result<Declaration> {
    let (attrs, pattern, closure) = block_statement(statement)?;
    let kind = attrs
        .iter()
        .find(|attr| attr.path().is_ident("stage"))
        .ok_or_else(|| {
            Error::new_spanned(
                statement,
                "a kaalang stage needs `#[stage(\"description\")]`",
            )
        })?;
    if attrs
        .iter()
        .filter(|attr| attr.path().is_ident("stage"))
        .count()
        != 1
        || attrs
            .iter()
            .any(|attr| !attr.path().is_ident("stage") && !attr.path().is_ident("doc"))
    {
        return Err(Error::new_spanned(
            statement,
            "a kaalang stage declares exactly one kind",
        ));
    }
    let description = description(kind, "kaalang stage")?;
    let Some(closure) = closure else {
        return Err(Error::new_spanned(
            statement,
            "a kaalang stage requires a header and braced body",
        ));
    };
    let original = match statement {
        Stmt::Local(local) => local.init.as_ref().map(|init| ungrouped(&init.expr)),
        Stmt::Expr(expression, _) => Some(ungrouped(expression)),
        _ => None,
    };
    if !matches!(original, Some(Expr::Closure(_)))
        || !matches!(closure.body.as_ref(), Expr::Block(_))
    {
        return Err(Error::new_spanned(
            statement,
            "a kaalang stage requires a header and braced body",
        ));
    }
    let (inputs, body) = block_closure(&closure)?;
    let [entry] = inputs.as_slice() else {
        return Err(Error::new_spanned(
            &closure.inputs,
            "a kaalang stage header names exactly one entry",
        ));
    };
    if entry.borrowed || entry.mutable {
        return Err(Error::new(
            entry.alias.span(),
            "a kaalang stage entry is a plain wire name",
        ));
    }
    if entry.ident == "self" {
        return Err(Error::new(
            entry.alias.span(),
            "a kaalang stage entry cannot be the receiver `self`; produce a named wire in preparation",
        ));
    }
    let outputs = block_outputs(&pattern)?;
    let output_bindings = (0..outputs.len())
        .map(|index| {
            let binding = match &pattern {
                Pat::Tuple(tuple) => &tuple.elems[index],
                other => other,
            };
            let Pat::Ident(binding) = binding else {
                unreachable!("validated stage output")
            };
            if binding.mutability.is_some() {
                return Err(Error::new(
                    binding.ident.span(),
                    "a kaalang stage output cannot declare `mut`",
                ));
            }
            Ok(binding.ident.clone())
        })
        .collect::<Result<Vec<_>>>()?;
    let mut seen = BTreeSet::new();
    for output in &outputs {
        if !seen.insert(output.unraw().to_string()) {
            return Err(Error::new(
                output.span(),
                "a kaalang stage declares each output once",
            ));
        }
    }
    let Expr::Block(body) = body else {
        unreachable!("stage body is braced")
    };
    if matches!(statement, Stmt::Expr(_, None)) {
        return Err(Error::new_spanned(
            statement,
            "a kaalang stage requires a trailing semicolon",
        ));
    }
    Ok(Declaration {
        description,
        entry: entry.ident.clone(),
        entry_alias: entry.alias.clone(),
        outputs,
        output_bindings,
        statements: body.block.stmts,
        span: kind.span(),
    })
}

fn transition(name: &Ident, alias: &Ident, target: usize) -> Block {
    let span = alias.span();
    let mut block = structural_block(
        BlockKind::Return,
        span,
        vec![Input {
            borrowed: false,
            mutable: false,
            ident: name.clone(),
            alias: alias.clone(),
            derived: false,
        }],
    );
    block.body = parse_quote_spanned!(span=> #alias);
    block.transition_target = Some(target);
    block
}
