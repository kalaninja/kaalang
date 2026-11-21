use std::rc::Rc;

use kaalang_compiler::{FlowItem, SourceFlow};
use syn::{ReturnType, Signature, spanned::Spanned};

use crate::{Hotspot, RenderError, validate_description, validate_label, with_visibility};

mod captions;
pub(crate) mod layout;
mod navigation;
mod svg;

/// Renders a flow and lists its hotspots.
pub(crate) fn render(flow: SourceFlow) -> Result<(String, Vec<Hotspot>), RenderError> {
    let SourceFlow {
        item,
        mut model,
        header,
        parameters,
        return_type,
        ..
    } = flow;
    let signature = &item.function.sig;
    kaalang_render::compact_arrangement(&mut model);
    for stage in &mut model.stages {
        kaalang_render::compact_arrangement(stage);
    }
    let direct_entry = kaalang_render::keep_header(&mut model);
    let mut hotspots = Vec::new();
    let start = start_text(&item, &header);
    validate_labels(&model, signature, &start, &parameters, &return_type)?;
    let unroutable = |reason| RenderError::UnroutableTopology {
        name: item.path.clone(),
        reason,
    };
    if !model.stages.is_empty() {
        for (stage, analysis) in model.stages.iter().zip(&model.analysis.stages) {
            let span = analysis.entry.span();
            validate_description(&analysis.description, span, "stage description")?;
            validate_labels(stage, signature, &start, &[], &return_type)?;
        }
        let scene = layout::layout_staged(&model, direct_entry, &start, &parameters, &return_type)
            .map_err(unroutable)?;
        let parts = std::iter::once(None).chain((0..model.stages.len()).map(Some));
        for ((local, placement), part) in scene.parts.iter().zip(parts) {
            navigation::collect(local, &model, part, *placement, &mut hotspots);
        }
        return Ok((svg::serialize_staged(&scene, &model.analysis), hotspots));
    }
    // Each formula is laid out once, here rather than per layout attempt.
    // Validation above has already parsed the same descriptions, without their
    // math, to read the characters a reader would see.
    let captions = Rc::new(captions::derive(&model, &start, &return_type));
    let scene = layout::layout(&model, &captions, &parameters).map_err(unroutable)?;
    let origin = layout::Point { x: 0, y: 0 };
    navigation::collect(&scene, &model, None, origin, &mut hotspots);

    Ok((
        svg::serialize(&scene, &model.analysis.name.to_string(), None),
        hotspots,
    ))
}

/// The start node's text: the authored header, behind its marker when the
/// function has a visibility of its own. A trait's method has its trait's,
/// which the file may not tell.
pub(crate) fn start_text(item: &FlowItem, header: &str) -> String {
    if item.trait_method {
        header.to_owned()
    } else {
        with_visibility(&item.function.vis, header)
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
