//! Renders one flow: derive its captions, lay out its compacted arrangement,
//! and serialize the scene.

use std::rc::Rc;

use kaalang_compiler::FlowItem;
use syn::{Item, Meta, ReturnType, Signature, spanned::Spanned};

use crate::{
    RenderError, RenderOptions, location, parse_configured, parser_source, validate_description,
    validate_label,
};

mod captions;
pub(crate) mod layout;
mod svg;

/// Renders the flow at `position` among the file's flows.
pub(crate) fn render(
    source: &str,
    position: usize,
    options: RenderOptions<'_>,
) -> Result<String, RenderError> {
    let file = parse_configured(source, options.target)?;
    let parser_source = parser_source(source, &file);
    let flow = select_flow(&file.items, position)?;
    let flow_path = flow.path.as_str();
    let function = &flow.function;
    let mut model = kaalang_compiler::build_with_options(function, options.collapse_cycles)
        .map_err(|error| invalid_flow(flow_path, &error))?;
    kaalang_render::compact_arrangement(&mut model);
    for stage in &mut model.stages {
        kaalang_render::compact_arrangement(stage);
    }
    // A trait's method has its trait's visibility, which the file may not tell.
    let visibility = (!flow.trait_method).then_some(&function.vis);
    let start = layout::start_text(parser_source, &function.sig, visibility);
    let parameters = layout::parameter_text(parser_source, &function.sig);
    let return_type = layout::return_text(parser_source, &function.sig.output);
    validate_labels(&model, &function.sig, &start, &parameters, &return_type)?;
    let unroutable = |reason| RenderError::UnroutableTopology {
        name: flow_path.to_owned(),
        reason,
    };
    if !model.stages.is_empty() {
        for (stage, analysis) in model.stages.iter().zip(&model.analysis.stages) {
            let span = analysis.entry.span();
            validate_description(&analysis.description, span, "stage description")?;
            validate_labels(stage, &function.sig, &start, &[], &return_type)?;
        }
        let scene =
            layout::layout_staged(&model, &start, &parameters, &return_type).map_err(unroutable)?;
        return Ok(svg::serialize_staged(&scene, &model.analysis));
    }
    // Each formula is laid out once, here rather than per layout attempt.
    // Validation above has already parsed the same descriptions, without their
    // math, to read the characters a reader would see.
    let captions = Rc::new(captions::derive(&model, &start, &return_type));
    let scene = layout::layout(&model, &captions, &parameters).map_err(unroutable)?;

    Ok(svg::serialize(
        &scene,
        &model.analysis.name.to_string(),
        None,
    ))
}

/// The flow at `path` among `items`, as `flow_names` lists it, for tests that
/// name a flow rather than count it.
#[cfg(test)]
pub(crate) fn select_named(items: &[Item], path: &str) -> Result<FlowItem, RenderError> {
    let position = kaalang_compiler::declarations(items)
        .flows
        .iter()
        .position(|flow| flow.path == path)
        .ok_or_else(|| RenderError::FlowNotFound(path.to_owned()))?;
    select_flow(items, position)
}

fn select_flow(items: &[Item], position: usize) -> Result<FlowItem, RenderError> {
    let Some(flow) = kaalang_compiler::declarations(items)
        .flows
        .into_iter()
        .nth(position)
    else {
        return Err(RenderError::FlowNotFound(position.to_string()));
    };
    if let Some(attribute) = flow
        .function
        .attrs
        .iter()
        .filter(|attribute| kaalang_compiler::is_kaalang_attribute(attribute))
        .find(|attribute| match &attribute.meta {
            Meta::Path(_) => false,
            Meta::List(arguments) => !arguments.tokens.is_empty(),
            Meta::NameValue(_) => true,
        })
    {
        return Err(invalid_flow(
            &flow.path,
            &syn::Error::new_spanned(attribute, "#[kaa] on a function does not accept arguments"),
        ));
    }

    Ok(flow)
}

/// Reports a flow-level error at its source position.
fn invalid_flow(flow_name: &str, error: &syn::Error) -> RenderError {
    let (line, column) = location(error.span());
    RenderError::InvalidFlow {
        name: flow_name.to_owned(),
        line,
        column,
        message: error.to_string(),
    }
}

/// Checks authored descriptions and source-derived labels for invalid XML characters.
fn validate_labels(
    model: &kaalang_compiler::SemanticModel,
    signature: &Signature,
    start: &str,
    parameters: &[String],
    return_type: &str,
) -> Result<(), RenderError> {
    for block in &model.analysis.flow.blocks {
        if block.kind == kaalang_compiler::BlockKind::Call && block.description.is_none() {
            validate_label(&block.callee(), block.span, "call label")?;
        }
        let description = block
            .description
            .as_deref()
            .map(|text| (text, "block description".to_owned()));
        let cases = block
            .case_descriptions
            .iter()
            .enumerate()
            .map(|(case, text)| (text.as_str(), format!("case {} description", case + 1)));
        let branches = block
            .question_branches
            .iter()
            .enumerate()
            .filter_map(|(branch, answer)| {
                answer
                    .description
                    .as_deref()
                    .map(|text| (text, format!("question branch {} description", branch + 1)))
            });
        for (label, context) in description.into_iter().chain(cases).chain(branches) {
            validate_description(label, block.span, context)?;
        }
    }
    validate_label(start, signature.span(), "flow header")?;
    for (index, (parameter, text)) in signature.inputs.iter().zip(parameters).enumerate() {
        validate_label(text, parameter.span(), format!("parameter {}", index + 1))?;
    }
    if let ReturnType::Type(_, ty) = &signature.output {
        validate_label(return_type, ty.span(), "return type")?;
    }

    Ok(())
}
