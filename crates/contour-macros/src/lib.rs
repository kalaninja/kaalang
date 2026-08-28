use std::collections::{HashMap, HashSet};

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{
    Attribute, Error, Expr, FnArg, ItemFn, LitStr, MacroDelimiter, Meta, Pat, PathArguments,
    Result, ReturnType, Stmt, Type, parse::Parser, parse_macro_input, spanned::Spanned,
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
    pattern: Pat,
    outputs: Vec<Ident>,
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
    let (pattern, outputs, tuple_output) = block_outputs(closure)?;
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

    Ok(Node {
        kind,
        pattern,
        outputs,
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

fn block_outputs(closure: &syn::ExprClosure) -> Result<(Pat, Vec<Ident>, bool)> {
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
    let pattern = if tuple {
        Pat::parse_single.parse2(quote_spanned!(output.span()=> (#(#outputs,)*)))?
    } else {
        let output = &outputs[0];
        Pat::parse_single.parse2(quote!(#output))?
    };

    Ok((pattern, outputs, tuple))
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
        return finish_path(nodes, &state);
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

    let bindings = capture_bindings(&node.inputs);
    let body = block_body(&node.body);

    match node.kind {
        NodeKind::Action => {
            for output in &node.outputs {
                next.available.insert(output.to_string());
            }
            let continuation = lower_path(nodes, next, visited)?;
            let pattern = &node.pattern;
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
            let yes_path = lower_path(nodes, yes_state, visited)?;
            let no_path = lower_path(nodes, no_state, visited)?;

            Ok(quote_spanned! {node.span=>
                if {
                    #bindings
                    #body
                } {
                    let #yes = ();
                    #yes_path
                } else {
                    let #no = ();
                    #no_path
                }
            })
        }
        NodeKind::Choice => {
            let choice_type = Ident::new(&format!("__ContourChoice{index}"), Span::mixed_site());
            let selected = Ident::new(
                &format!("__contour_selected_choice_{index}"),
                Span::mixed_site(),
            );
            let variants = (0..node.outputs.len())
                .map(|case| Ident::new(&format!("Case{case}"), Span::mixed_site()))
                .collect::<Vec<_>>();
            let mut paths = Vec::with_capacity(node.outputs.len());
            for output in &node.outputs {
                let mut branch_state = next.clone();
                branch_state.available.insert(output.to_string());
                paths.push(lower_path(nodes, branch_state, visited)?);
            }
            let outputs = &node.outputs;

            Ok(quote_spanned! {node.span=>
                #[allow(dead_code)]
                #[derive(Clone, Copy)]
                enum #choice_type {
                    #(#variants,)*
                }

                let #selected: #choice_type = {
                    #bindings
                    #(
                        #[allow(unused_variables)]
                        let #outputs = #choice_type::#variants;
                    )*
                    #body
                };

                match #selected {
                    #(
                        #choice_type::#variants => {
                            let #outputs = ();
                            #paths
                        }
                    ),*
                }
            })
        }
    }
}

fn finish_path(nodes: &[Node], state: &PathState) -> Result<TokenStream2> {
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
    Ok(quote_spanned!(output.span()=> #output))
}

fn capture_bindings(inputs: &[Capture]) -> TokenStream2 {
    let bindings = inputs.iter().map(|input| {
        let ident = &input.ident;
        if input.borrowed {
            quote_spanned!(ident.span()=>
                #[allow(unused_variables)]
                let #ident = &#ident;
            )
        } else {
            quote_spanned!(ident.span()=>
                #[allow(unused_variables, clippy::let_unit_value, clippy::redundant_locals)]
                let #ident = #ident;
            )
        }
    });

    quote!(#(#bindings)*)
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
