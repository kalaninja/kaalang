use std::collections::{HashMap, HashSet};

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{
    Attribute, Error, Expr, FnArg, ItemFn, LitStr, MacroDelimiter, Meta, Pat, PathArguments,
    Result, ReturnType, Stmt, Type, parse_macro_input, spanned::Spanned,
};

/// Parses and lowers an ordinary Rust function containing a Contour graph.
#[proc_macro_attribute]
pub fn contour(attributes: TokenStream, item: TokenStream) -> TokenStream {
    if !attributes.is_empty() {
        return Error::new(Span::call_site(), "#[contour] does not accept arguments")
            .into_compile_error()
            .into();
    }

    let mut function = parse_macro_input!(item as ItemFn);
    match expand(&mut function) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand(function: &mut ItemFn) -> Result<TokenStream2> {
    let sources = source_wires(function)?;
    let mut nodes = parse_nodes(&function.block.stmts)?;
    validate_graph(&sources, &mut nodes)?;
    let wires = internal_wires(&sources, &nodes);
    rename_source_bindings(function, &sources, &wires);

    let mut visited = HashSet::new();
    let body = lower_path(
        &nodes,
        PathState {
            available: sources
                .iter()
                .map(ToString::to_string)
                .collect::<HashSet<_>>(),
            executed: HashSet::new(),
            last: None,
        },
        &mut visited,
        &wires,
    )?;

    if let Some(unreachable) = nodes
        .iter()
        .enumerate()
        .find(|(index, _)| !visited.contains(index))
        .map(|(_, node)| node)
    {
        return Err(Error::new(
            unreachable.span,
            "this Contour block is unreachable",
        ));
    }

    *function.block = syn::parse2(quote!({ #body }))?;

    Ok(quote! {
        #[allow(unreachable_code)]
        #function
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Action,
    Question,
    Choice,
}

struct Node {
    kind: NodeKind,
    outputs: Vec<Ident>,
    tuple_output: bool,
    output_span: Span,
    inputs: Vec<Capture>,
    body: Expr,
    terminal: bool,
    span: Span,
}

#[derive(Clone)]
struct Capture {
    borrowed: bool,
    ident: Ident,
}

fn source_wires(function: &ItemFn) -> Result<Vec<Ident>> {
    function
        .sig
        .inputs
        .iter()
        .map(|argument| match argument {
            FnArg::Typed(argument) => simple_binding(&argument.pat, "function parameters"),
            FnArg::Receiver(receiver) => Err(Error::new(
                receiver.span(),
                "#[contour] is supported only on free functions",
            )),
        })
        .collect()
}

fn parse_nodes(statements: &[Stmt]) -> Result<Vec<Node>> {
    if statements.is_empty() {
        return Err(Error::new(
            Span::call_site(),
            "a Contour flow requires at least one block",
        ));
    }

    statements.iter().map(parse_node).collect()
}

fn parse_node(statement: &Stmt) -> Result<Node> {
    let Stmt::Expr(expression, semicolon) = statement else {
        return Err(Error::new_spanned(
            statement,
            "a Contour body may contain only attributed closure statements",
        ));
    };
    let Expr::Closure(closure) = expression else {
        return Err(Error::new_spanned(
            expression,
            "a Contour block must have the form `|inputs| -> outputs { body };`",
        ));
    };
    if semicolon.is_none() {
        return Err(Error::new_spanned(
            closure,
            "a Contour block must end with a semicolon",
        ));
    }

    let (kind, marker, case_count) = block_marker(&closure.attrs, closure.inputs_begin.span())?;
    let (inputs, body) = block_closure(closure)?;
    let (outputs, tuple_output, output_span) = block_outputs(closure)?;
    match kind {
        NodeKind::Question if !tuple_output || outputs.len() != 2 => {
            return Err(Error::new_spanned(
                &closure.output,
                "a question must declare exactly two outputs",
            ));
        }
        NodeKind::Choice if !tuple_output => {
            return Err(Error::new_spanned(
                &closure.output,
                "a choice must declare its outputs as a tuple",
            ));
        }
        NodeKind::Choice if case_count < 2 => {
            return Err(Error::new_spanned(
                marker,
                "a choice requires at least two `#[case(\"description\")]` attributes",
            ));
        }
        NodeKind::Choice if case_count != outputs.len() => {
            return Err(Error::new_spanned(
                &closure.output,
                "a choice must declare exactly one output for each case",
            ));
        }
        _ => {}
    }
    if kind == NodeKind::Choice {
        validate_choice_body(&body, &outputs)?;
    }

    Ok(Node {
        kind,
        outputs,
        tuple_output,
        output_span,
        inputs,
        body,
        terminal: false,
        span: marker.span(),
    })
}

fn block_closure(closure: &syn::ExprClosure) -> Result<(Vec<Capture>, Expr)> {
    if closure.lifetimes.is_some()
        || closure.constness.is_some()
        || closure.asyncness.is_some()
        || closure.capture.is_some()
    {
        return Err(Error::new_spanned(
            closure,
            "Contour block closures do not support `move`, `async`, `const`, or lifetime modifiers",
        ));
    }
    if closure.inputs.is_empty() {
        return Err(Error::new(
            closure.inputs_end.span(),
            "a Contour block requires at least one input",
        ));
    }

    let inputs = closure
        .inputs
        .iter()
        .map(block_capture)
        .collect::<Result<_>>()?;
    Ok((inputs, closure.body.as_ref().clone()))
}

fn block_capture(pattern: &Pat) -> Result<Capture> {
    match pattern {
        Pat::Ident(_) => Ok(Capture {
            borrowed: false,
            ident: simple_binding(pattern, "block inputs")?,
        }),
        Pat::Reference(reference)
            if reference.attrs.is_empty() && reference.mutability.is_none() =>
        {
            Ok(Capture {
                borrowed: true,
                ident: simple_binding(&reference.pat, "block inputs")?,
            })
        }
        _ => Err(Error::new_spanned(
            pattern,
            "block inputs must contain only `name` or `&name`",
        )),
    }
}

fn block_outputs(closure: &syn::ExprClosure) -> Result<(Vec<Ident>, bool, Span)> {
    let ReturnType::Type(_, output) = &closure.output else {
        return Err(Error::new(
            closure.inputs_end.span(),
            "a Contour block must declare its outputs after `->`",
        ));
    };

    let (outputs, tuple) = match output.as_ref() {
        Type::Tuple(tuple) if !tuple.elems.is_empty() => (
            tuple
                .elems
                .iter()
                .map(output_ident)
                .collect::<Result<Vec<_>>>()?,
            true,
        ),
        output => (vec![output_ident(output)?], false),
    };
    Ok((outputs, tuple, output.span()))
}

fn output_ident(output: &Type) -> Result<Ident> {
    let Type::Path(path) = output else {
        return Err(Error::new_spanned(
            output,
            "block outputs must contain only identifiers",
        ));
    };
    let Some(segment) = path.path.segments.first() else {
        return Err(Error::new_spanned(
            output,
            "block outputs must contain only identifiers",
        ));
    };
    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 1
        || !matches!(segment.arguments, PathArguments::None)
    {
        return Err(Error::new_spanned(
            output,
            "block outputs must contain only identifiers",
        ));
    }

    Ok(segment.ident.clone())
}

fn block_marker(
    attributes: &[Attribute],
    fallback_span: Span,
) -> Result<(NodeKind, &Attribute, usize)> {
    let mut marker = None;
    let mut cases = Vec::new();

    for attribute in attributes {
        let kind = if attribute.path().is_ident("action") {
            Some(NodeKind::Action)
        } else if attribute.path().is_ident("question") {
            Some(NodeKind::Question)
        } else if attribute.path().is_ident("choice") {
            Some(NodeKind::Choice)
        } else if attribute.path().is_ident("case") {
            if marker.is_none() {
                return Err(Error::new_spanned(
                    attribute,
                    "a `#[case(\"description\")]` attribute must follow `#[choice(\"description\")]`",
                ));
            }
            cases.push(attribute);
            continue;
        } else if attribute.path().is_ident("doc") {
            None
        } else {
            return Err(Error::new_spanned(
                attribute,
                "Contour blocks support only comments and action, question, choice, or case attributes",
            ));
        };

        let Some(kind) = kind else {
            continue;
        };
        if marker.replace((kind, attribute)).is_some() {
            return Err(Error::new_spanned(
                attribute,
                "a Contour block requires exactly one action, question, or choice attribute",
            ));
        }
    }

    let Some((kind, attribute)) = marker else {
        return Err(Error::new(
            fallback_span,
            "a Contour block requires an action, question, or choice attribute",
        ));
    };
    block_description(attribute, "Contour block")?;

    if kind != NodeKind::Choice {
        if let Some(case) = cases.first() {
            return Err(Error::new_spanned(
                case,
                "`#[case(\"description\")]` is valid only on a choice block",
            ));
        }
    } else {
        for case in &cases {
            block_description(case, "choice case")?;
        }
    }

    Ok((kind, attribute, cases.len()))
}

fn block_description(attribute: &Attribute, role: &str) -> Result<LitStr> {
    let Meta::List(description) = &attribute.meta else {
        return Err(Error::new_spanned(
            attribute,
            format!("a {role} attribute requires a parenthesized description"),
        ));
    };
    if !matches!(description.delimiter, MacroDelimiter::Paren(_)) {
        return Err(Error::new_spanned(
            attribute,
            format!("a {role} description must use parentheses"),
        ));
    }
    let description = attribute.parse_args::<LitStr>().map_err(|_| {
        Error::new_spanned(
            attribute,
            format!("a {role} description must be a string literal"),
        )
    })?;
    if description.value().trim().is_empty() {
        return Err(Error::new_spanned(
            description,
            format!("a {role} requires a non-empty description"),
        ));
    }

    Ok(description)
}

fn simple_binding(pattern: &Pat, role: &str) -> Result<Ident> {
    let Pat::Ident(binding) = pattern else {
        return Err(Error::new_spanned(
            pattern,
            format!("{role} must use simple identifiers"),
        ));
    };
    if !binding.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
    {
        return Err(Error::new_spanned(
            pattern,
            format!("{role} must use simple identifiers"),
        ));
    }

    Ok(binding.ident.clone())
}

fn validate_graph(sources: &[Ident], nodes: &mut [Node]) -> Result<()> {
    let mut producers = HashSet::new();
    for source in sources {
        if !producers.insert(source.to_string()) {
            return Err(Error::new(source.span(), "duplicate source wire"));
        }
    }

    for node in nodes.iter() {
        let mut captured = HashSet::new();
        for input in &node.inputs {
            let name = input.ident.to_string();
            if !captured.insert(name.clone()) {
                return Err(Error::new(input.ident.span(), "duplicate block input"));
            }
            if !producers.contains(&name) {
                return Err(Error::new(
                    input.ident.span(),
                    "input must reference a function parameter or an earlier block output",
                ));
            }
        }

        for output in &node.outputs {
            if !producers.insert(output.to_string()) {
                return Err(Error::new(output.span(), "duplicate wire name"));
            }
        }
    }

    let mut consumers = HashMap::<String, usize>::new();
    for node in nodes.iter() {
        for input in &node.inputs {
            *consumers.entry(input.ident.to_string()).or_default() += 1;
        }
    }

    for node in nodes.iter_mut() {
        let unconsumed = node
            .outputs
            .iter()
            .filter(|output| !consumers.contains_key(&output.to_string()))
            .collect::<Vec<_>>();

        match node.kind {
            NodeKind::Question if !unconsumed.is_empty() => {
                return Err(Error::new(
                    unconsumed[0].span(),
                    "every question branch must have a consumer",
                ));
            }
            NodeKind::Choice if !unconsumed.is_empty() => {
                return Err(Error::new(
                    unconsumed[0].span(),
                    "every choice case must have a consumer",
                ));
            }
            NodeKind::Action if unconsumed.is_empty() => {}
            NodeKind::Action if node.outputs.len() == 1 => node.terminal = true,
            NodeKind::Action => {
                return Err(Error::new(
                    unconsumed[0].span(),
                    "a terminal action must have exactly one output",
                ));
            }
            NodeKind::Question | NodeKind::Choice => {}
        }
    }

    Ok(())
}

fn internal_wires(sources: &[Ident], nodes: &[Node]) -> HashMap<String, Ident> {
    // Source spellings are reserved for aliases inside the blocks that capture them.
    sources
        .iter()
        .chain(nodes.iter().flat_map(|node| &node.outputs))
        .enumerate()
        .map(|(index, wire)| {
            (
                wire.to_string(),
                Ident::new(
                    &format!("__contour_wire_{index}"),
                    Span::mixed_site().located_at(wire.span()),
                ),
            )
        })
        .collect()
}

fn rename_source_bindings(
    function: &mut ItemFn,
    sources: &[Ident],
    wires: &HashMap<String, Ident>,
) {
    for (argument, source) in function.sig.inputs.iter_mut().zip(sources) {
        let FnArg::Typed(argument) = argument else {
            unreachable!("source_wires rejects method receivers")
        };
        let Pat::Ident(binding) = argument.pat.as_mut() else {
            unreachable!("source_wires accepts only simple parameter bindings")
        };
        binding.ident = wire_binding(wires, source).clone();
    }
}

fn validate_choice_body(body: &Expr, outputs: &[Ident]) -> Result<()> {
    if is_todo_body(body) {
        return Ok(());
    }
    let Some(choice) = choice_match(body) else {
        return Err(Error::new_spanned(
            body,
            "a choice body must contain exactly one `match` expression or `todo!()`",
        ));
    };
    if choice.arms.len() != outputs.len() {
        return Err(Error::new_spanned(
            choice,
            "a choice match must contain exactly one arm for each output",
        ));
    }
    if let Some(arm) = choice
        .arms
        .iter()
        .find(|arm| is_todo_macro(&arm.body) || is_todo_body(&arm.body))
    {
        return Err(Error::new_spanned(
            &arm.body,
            "`todo!()` is supported only as the whole choice body",
        ));
    }

    Ok(())
}

fn choice_match(body: &Expr) -> Option<&syn::ExprMatch> {
    let expression = single_body_expression(body)?;
    let Expr::Match(choice) = expression else {
        return None;
    };
    Some(choice)
}

fn is_todo_body(body: &Expr) -> bool {
    let Some(expression) = single_body_expression(body) else {
        return false;
    };
    is_todo_macro(expression)
}

fn is_todo_macro(expression: &Expr) -> bool {
    let Expr::Macro(expression) = expression else {
        return false;
    };
    expression.attrs.is_empty()
        && expression.mac.path.is_ident("todo")
        && expression.mac.tokens.is_empty()
}

fn single_body_expression(body: &Expr) -> Option<&Expr> {
    let Expr::Block(block) = body else {
        return None;
    };
    if !block.attrs.is_empty() || block.label.is_some() {
        return None;
    }
    let [Stmt::Expr(expression, None)] = block.block.stmts.as_slice() else {
        return None;
    };
    Some(expression)
}

#[derive(Clone)]
struct PathState {
    available: HashSet<String>,
    executed: HashSet<usize>,
    last: Option<usize>,
}

fn lower_path(
    nodes: &[Node],
    state: PathState,
    visited: &mut HashSet<usize>,
    wires: &HashMap<String, Ident>,
) -> Result<TokenStream2> {
    let ready = nodes
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            !state.executed.contains(index)
                && node
                    .inputs
                    .iter()
                    .all(|input| state.available.contains(&input.ident.to_string()))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    if ready.len() > 1 {
        return Err(Error::new(
            nodes[ready[1]].span,
            "multiple Contour blocks are ready at once; add an explicit dependency",
        ));
    }
    let Some(index) = ready.first().copied() else {
        return finish_path(nodes, &state, wires);
    };

    visited.insert(index);
    let node = &nodes[index];
    let mut next = state;
    next.executed.insert(index);
    next.last = Some(index);
    for input in &node.inputs {
        if !input.borrowed {
            next.available.remove(&input.ident.to_string());
        }
    }

    let bindings = capture_bindings(&node.inputs, wires);
    let body = block_body(&node.body);

    match node.kind {
        NodeKind::Action => {
            for output in &node.outputs {
                next.available.insert(output.to_string());
            }
            let continuation = lower_path(nodes, next, visited, wires)?;
            let output_wires = node
                .outputs
                .iter()
                .map(|output| wire_binding(wires, output))
                .collect::<Vec<_>>();
            let pattern = if node.tuple_output {
                quote_spanned!(node.output_span=> (#(#output_wires,)*))
            } else {
                let output = output_wires[0];
                quote_spanned!(node.output_span=> #output)
            };
            Ok(quote_spanned! {node.span=>
                let #pattern = {
                    #bindings
                    #body
                };
                #continuation
            })
        }
        NodeKind::Question => {
            let yes = &node.outputs[0];
            let no = &node.outputs[1];
            let mut yes_state = next.clone();
            yes_state.available.insert(yes.to_string());
            let mut no_state = next;
            no_state.available.insert(no.to_string());
            let yes_path = lower_path(nodes, yes_state, visited, wires)?;
            let no_path = lower_path(nodes, no_state, visited, wires)?;
            let yes_wire = wire_binding(wires, yes);
            let no_wire = wire_binding(wires, no);

            Ok(quote_spanned! {node.span=>
                if {
                    #bindings
                    #body
                } {
                    let #yes_wire = ();
                    #yes_path
                } else {
                    let #no_wire = ();
                    #no_path
                }
            })
        }
        NodeKind::Choice => {
            let mut paths = Vec::with_capacity(node.outputs.len());
            for output in &node.outputs {
                let mut branch_state = next.clone();
                branch_state.available.insert(output.to_string());
                paths.push(lower_path(nodes, branch_state, visited, wires)?);
            }
            let continuations = node
                .outputs
                .iter()
                .enumerate()
                .map(|(case, _)| {
                    Ident::new(
                        &format!("__contour_continue_{index}_{case}"),
                        Span::mixed_site(),
                    )
                })
                .collect::<Vec<_>>();
            let capability_type = Ident::new(
                &format!("__ContourContinuationCapability{index}"),
                Span::mixed_site(),
            );
            let capability = Ident::new(
                &format!("__contour_continuation_capability_{index}"),
                Span::mixed_site(),
            );
            let consumed_capability = Ident::new(
                &format!("__contour_consumed_capability_{index}"),
                Span::mixed_site(),
            );
            // Definition-site hygiene keeps match bindings out of downstream blocks.
            let definitions = node.outputs.iter().zip(&paths).zip(&continuations).map(
                |((output, path), continuation)| {
                    let output_wire = wire_binding(wires, output);
                    quote! {
                        macro_rules! #continuation {
                            ($value:expr) => {{
                                let #consumed_capability: #capability_type = #capability;
                                let #output_wire = $value;
                                #path
                            }};
                        }
                    }
                },
            );

            let dispatch = if is_todo_body(&node.body) {
                let numbered = continuations[..continuations.len() - 1]
                    .iter()
                    .enumerate()
                    .map(|(case, continuation)| quote!(#case => #continuation!(todo!()),));
                let last = continuations
                    .last()
                    .expect("a choice has at least two cases");
                quote! {
                    #[allow(clippy::diverging_sub_expression)]
                    match {
                        #bindings
                        #body
                    } {
                        #(#numbered)*
                        _ => #last!(todo!()),
                    }
                }
            } else {
                let choice = choice_match(&node.body).expect("choice bodies are validated");
                let match_attrs = &choice.attrs;
                let scrutinee = &choice.expr;
                let arms = choice
                    .arms
                    .iter()
                    .zip(&continuations)
                    .map(|(arm, continuation)| {
                        let attrs = &arm.attrs;
                        let pattern = &arm.pat;
                        let value = &arm.body;
                        quote! {
                            #(#attrs)*
                            #pattern => #continuation!(#value),
                        }
                    });
                quote! {
                    {
                        #bindings
                        #(#match_attrs)*
                        match #scrutinee {
                            #(#arms)*
                        }
                    }
                }
            };

            Ok(quote_spanned! {node.span=>
                struct #capability_type;
                let #capability = #capability_type;
                #(#definitions)*
                #dispatch
            })
        }
    }
}

fn finish_path(
    nodes: &[Node],
    state: &PathState,
    wires: &HashMap<String, Ident>,
) -> Result<TokenStream2> {
    let last = state
        .last
        .expect("the first block is always ready, so a finished path executed one");
    let node = &nodes[last];
    if !node.terminal {
        return Err(Error::new(
            node.span,
            "this path does not terminate in an action",
        ));
    }

    let output = &node.outputs[0];
    let output_wire = wire_binding(wires, output);
    Ok(quote_spanned!(output.span()=> #output_wire))
}

fn capture_bindings(inputs: &[Capture], wires: &HashMap<String, Ident>) -> TokenStream2 {
    let bindings = inputs.iter().map(|input| {
        let ident = &input.ident;
        let wire = wire_binding(wires, ident);
        if input.borrowed {
            quote_spanned!(ident.span()=>
                #[allow(unused_variables)]
                let #ident = &#wire;
            )
        } else {
            quote_spanned!(ident.span()=>
                #[allow(unused_variables, clippy::let_unit_value)]
                let #ident = #wire;
            )
        }
    });

    quote!(#(#bindings)*)
}

fn wire_binding<'a>(wires: &'a HashMap<String, Ident>, wire: &Ident) -> &'a Ident {
    wires
        .get(&wire.to_string())
        .expect("validated wires have internal bindings")
}

/// Splices the statements of a block body so lowering adds no extra braces.
fn block_body(body: &Expr) -> TokenStream2 {
    match body {
        Expr::Block(block) if block.attrs.is_empty() && block.label.is_none() => {
            let statements = &block.block.stmts;
            quote!(#(#statements)*)
        }
        body => quote!(#body),
    }
}
