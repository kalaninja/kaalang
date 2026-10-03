//! Parses a described cycle: a loop with its optional entry gate and declared
//! outputs, or a for cycle with its header captures, item and completion.

use proc_macro2::{Ident, Span};
use syn::{
    Error, Expr, ExprBlock, ExprBreak, ExprForLoop, ExprLoop, Pat, Result, parse_quote,
    parse_quote_spanned,
};

use super::{
    BlockSyntax, block_outputs, decorated_body, description, reject_control_transfers,
    structural_block, structural_expression, ungrouped,
};
use crate::model::{Block, BlockKind, Input, Iteration, UNNAMED};

pub(super) fn parse(mut syntax: BlockSyntax<'_>) -> Result<Block> {
    let iteration = match ungrouped(structural_expression(&syntax.body)).clone() {
        Expr::Loop(body) => {
            if body.label.is_some() || !body.attrs.is_empty() {
                return Err(decorated_body(body));
            }
            // The statements inside the loop are the body; `loop` only names the form.
            syntax.body = braced(body.body);
            None
        }
        Expr::ForLoop(body) => {
            if body.label.is_some() || !body.attrs.is_empty() {
                return Err(decorated_body(body));
            }
            if !matches!(&*body.pat, Pat::Wild(_))
                && !matches!(&*body.pat, Pat::Ident(item) if item.by_ref.is_none() && item.subpat.is_none())
            {
                return Err(Error::new_spanned(
                    &body.pat,
                    "a kaalang for cycle names its item with one binding or `_`",
                ));
            }
            // The iterated expression runs once, before the first item.
            reject_control_transfers(&body.expr)?;
            if syntax.outputs.len() > 1 {
                return Err(Error::new(
                    syntax.output_span,
                    "a kaalang for cycle declares at most one output, the signal that its items ran out",
                ));
            }
            syntax.body = braced(body.body);
            Some(Iteration {
                item: *body.pat,
                items: *body.expr,
            })
        }
        _ => {
            return Err(Error::new_spanned(
                &syntax.body,
                "a kaalang cycle body is a `loop { ... }` or `for ... in ... { ... }` block",
            ));
        }
    };
    let description = description(syntax.kind_attribute, "kaalang cycle")?;
    syntax.reject_companions()?;
    if iteration.is_none() {
        // The header only gates entry; the body captures outer data itself.
        if let Some(extra) = syntax.inputs.get(1) {
            return Err(Error::new(
                extra.alias.span(),
                "a kaalang cycle header names at most one gate",
            ));
        }
        if let Some(gate) = syntax
            .inputs
            .first()
            .filter(|gate| gate.borrowed || gate.mutable)
        {
            return Err(Error::new(
                gate.alias.span(),
                "a kaalang cycle gate is a plain wire name, without `mut` or `&`",
            ));
        }
    }
    let mut block = syntax.into_block(Some(description), Vec::new());
    block.iteration = iteration;
    Ok(block)
}

fn braced(block: syn::Block) -> Expr {
    Expr::Block(ExprBlock {
        attrs: Vec::new(),
        label: None,
        block,
    })
}

/// Opens the body of the for cycle at `header` with the hidden choice that
/// takes its next item, or selects its completion once none is left. A for
/// cycle always completes, so one without a declared output gets a hidden one.
pub(super) fn open_iteration(blocks: &mut Vec<Block>, header: usize) -> Result<()> {
    let cycle = &mut blocks[header];
    let span = cycle.span;
    if cycle.outputs.is_empty() {
        let done = Ident::new(&format!("{UNNAMED}done_{header}"), span);
        cycle.output_pattern = parse_quote!(#done);
        cycle.outputs.push(done);
    }
    // The authored spelling keeps `r#`, so an output named after a keyword binds.
    let done = cycle.output_binding(0).ident.clone();
    let item = match &cycle.iteration.as_ref().expect("a for cycle").item {
        Pat::Wild(_) => {
            let item = Ident::new(&format!("{UNNAMED}item_{header}"), span);
            parse_quote!(#item)
        }
        item => item.clone(),
    };
    let pattern: Pat = parse_quote!((#item, #done));
    let items = Iteration::iterator();
    let value = Ident::new("item", Span::mixed_site());
    let mut next = structural_block(BlockKind::Choice, span, Vec::new());
    next.description.clone_from(&cycle.description);
    next.case_descriptions = vec![
        "Take the next item.".to_owned(),
        "No item is left.".to_owned(),
    ];
    next.outputs = block_outputs(&pattern)?;
    next.output_pattern = pattern;
    next.body = parse_quote_spanned! {span=> {
        match #items.next() {
            ::core::option::Option::Some(#value) => #value,
            ::core::option::Option::None => (),
        }
    }};
    next.parent = Some(header);
    blocks.push(next);
    Ok(())
}

/// Appends one hidden boundary consumer per declared output to the body that
/// ends at `blocks.len()`, in declaration order. A consumer
/// captures its body-local output; the route that reaches it with that output
/// exports it after all of its remaining work. A for cycle's body first ends
/// in a hidden `continue`, which every route with an item reaches.
pub(super) fn exports(blocks: &mut Vec<Block>, header: usize) -> Result<()> {
    let cycle = &blocks[header];
    for (position, output) in cycle.outputs.iter().enumerate() {
        if cycle.outputs[..position].contains(output) {
            return Err(Error::new(
                output.span(),
                "a kaalang cycle declares each output once",
            ));
        }
        let mut producers = blocks[header + 1..]
            .iter()
            .filter(|block| block.parent == Some(header) && block.outputs.contains(output));
        if cycle.iteration.is_some() {
            if let Some(producer) = producers.nth(1) {
                return Err(Error::new(
                    producer.outputs[producer
                        .outputs
                        .iter()
                        .position(|name| name == output)
                        .expect("the producer declares it")]
                    .span(),
                    format!(
                        "the body of a kaalang for cycle cannot produce its output `{output}`; the cycle provides it once its items run out"
                    ),
                ));
            }
        } else if producers.next().is_none() {
            return Err(Error::new(
                output.span(),
                format!(
                    "a declared kaalang cycle output needs a producer named `{output}` in the cycle body"
                ),
            ));
        }
    }
    if cycle.iteration.is_some() {
        let mut repeat = structural_block(BlockKind::Continue, cycle.span, Vec::new());
        repeat.parent = Some(header);
        blocks.push(repeat);
    }
    let cycle = &blocks[header];
    let consumers = (0..cycle.outputs.len())
        .map(|position| {
            let ident = cycle.outputs[position].clone();
            let alias = cycle.output_binding(position).ident.clone();
            let span = alias.span();
            let mut consumer = structural_block(
                BlockKind::Export,
                span,
                vec![Input {
                    borrowed: false,
                    mutable: false,
                    ident,
                    alias: alias.clone(),
                    derived: false,
                }],
            );
            consumer.body = parse_quote_spanned!(span=> #alias);
            consumer.parent = Some(header);
            consumer.export_target = Some(header);
            consumer
        })
        .collect::<Vec<_>>();
    blocks.extend(consumers);
    Ok(())
}

/// A cycle completes by producing a declared output, never by an authored break.
pub(super) fn structural_break(expression: &ExprBreak) -> Error {
    Error::new_spanned(
        expression,
        "a kaalang cycle has no structural `break`; it completes when a route reaches the end of its body with one of its declared outputs",
    )
}

pub(super) fn structural_for(expression: &ExprForLoop) -> Error {
    Error::new_spanned(
        expression,
        "a `for` in a kaalang flow is a cycle and needs `#[cycle(\"description\")]`",
    )
}

pub(super) fn structural_loop(expression: &ExprLoop) -> Error {
    Error::new_spanned(
        expression,
        "a `loop` in a kaalang flow is a cycle and needs `#[cycle(\"description\")]`",
    )
}
