use std::fmt::Write;

use crate::layout::{
    CONNECTION_LABEL_FONT, CONNECTION_LABEL_HALO, CYCLE_CAPTION_FONT, Connection, LABEL_FONT,
    LINE_HEIGHT, Label, LabelKind, MERGE_RADIUS, NODE_LABEL_PADDING_X, NODE_LABEL_PADDING_Y, Node,
    ParameterPanel, Point, Scene,
};
use crate::text::{
    Color, FORMULA_STRIKETHROUGH_OFFSET, FORMULA_UNDERLINE_OFFSET, FORMULA_UNDERLINE_PADDING,
    Formula, RichText, SCRIPT_FONT_SCALE, SUBSCRIPT_SHIFT, SUPERSCRIPT_SHIFT, Script, Style,
    TEXT_ADVANCE_SCALE, block_metrics, line_ink, shaping_spans, svg_dimension,
};
use kaalang_compiler::topology::{Destination, ExitId, NodeId, NodeKind, Source};
use latex_rust::Dim;
use unicode_segmentation::UnicodeSegmentation;

/// Appends one line to the SVG. Writing to a `String` cannot fail.
macro_rules! emit {
    ($svg:expr, $($argument:tt)*) => {
        writeln!($svg, $($argument)*).expect("writing to a String cannot fail")
    };
}

/// Appends to the SVG without ending the line. Under `xml:space="preserve"` the
/// whitespace between `<text>` and its `<tspan>` children is rendered content,
/// so a label is written as one unbroken line.
macro_rules! emit_inline {
    ($svg:expr, $($argument:tt)*) => {
        write!($svg, $($argument)*).expect("writing to a String cannot fail")
    };
}

mod action;
mod call;
mod choice;
mod cycle;
mod question;
mod stage;
mod staged;

pub(crate) use staged::serialize_staged;

const STROKE_WIDTH: f64 = 1.75;
const CYCLE_NODE_STROKE_WIDTH: i32 = 2;
const LOOP_MARKER_FONT: i32 = 20;
const CYCLE_STROKE_WIDTH: f64 = 1.5;
const CYCLE_DASH_LENGTH: i32 = 7;
const CYCLE_DASH_GAP: i32 = 5;
const CYCLE_CAPTION_HALO: i32 = 4;
const FONT_WEIGHT_REGULAR: i32 = 400;
const FONT_WEIGHT_MEDIUM: i32 = 500;
const FONT_WEIGHT_SEMIBOLD: i32 = 600;
const FONT_WEIGHT_BOLD: i32 = 700;
const FONT_WEIGHT_HEAVY: i32 = 800;
const FORMULA_BOLD_STROKE: f64 = 0.7;
const FORMULA_DECORATION_STROKE: i32 = 1;
const FORMULA_CLIP_PADDING: i32 = 1;
// The skew angle matching text::FORMULA_SLANT, rounded for SVG output.
const FORMULA_SKEW_DEGREES: f64 = -8.5308;
const ARROW_SIZE: i32 = 10;
const ARROW_MIDPOINT: i32 = ARROW_SIZE / 2;
const BACK_EDGE_ARROW_TIP_INSET: i32 = 1;
const BACK_EDGE_ARROW_REF_X: i32 = ARROW_SIZE - BACK_EDGE_ARROW_TIP_INSET;

pub(crate) fn serialize(scene: &Scene, flow_name: &str) -> String {
    serialize_with_ids(scene, flow_name, None)
}

#[allow(clippy::too_many_lines)] // The node and route groups write one complete local SVG.
fn serialize_with_ids(scene: &Scene, flow_name: &str, part: Option<usize>) -> String {
    let mut svg = String::new();
    let (title_id, description_id, back_edge_arrow_id) = part.map_or_else(
        || {
            (
                "kaalang-title".to_owned(),
                "kaalang-description".to_owned(),
                "loop-arrow".to_owned(),
            )
        },
        |part| {
            (
                format!("kaalang-part-{part}-title"),
                format!("kaalang-part-{part}-description"),
                format!("kaalang-part-{part}-loop-arrow"),
            )
        },
    );
    let markdown_styles = markdown_styles(scene);
    let background = if part.is_none() {
        "  <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n"
    } else {
        ""
    };
    let stage_styles = if scene
        .topology
        .nodes
        .iter()
        .any(|node| matches!(node.kind, NodeKind::StageEntry | NodeKind::Transition))
    {
        format!(
            "      .stage-entry .node-shape, .transition .node-shape {{ fill: #f5f3ff; }}\n      .stage-entry .label, .transition .label {{ font-weight: {FONT_WEIGHT_SEMIBOLD}; }}\n      .stage-marker {{ fill: currentColor; }}\n"
        )
    } else {
        String::new()
    };
    emit!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img" aria-labelledby="{title_id}" aria-describedby="{description_id}">"#,
        scene.width,
        scene.height,
        scene.width,
        scene.height
    );
    emit!(
        svg,
        "  <title id=\"{title_id}\">kaalang diagram for {}</title>",
        escape(flow_name)
    );
    emit!(
        svg,
        "  <desc id=\"{description_id}\">{}</desc>",
        escape(&describe(scene))
    );
    emit_inline!(
        svg,
        r#"  <defs>
    <style>
      svg {{ color: #1f2937; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; text-rendering: optimizeLegibility; }}
      .connection {{ fill: none; stroke: currentColor; stroke-width: {STROKE_WIDTH}; stroke-linecap: square; stroke-linejoin: round; }}
      .connection-label {{ fill: #64748b; font-size: {CONNECTION_LABEL_FONT}px; font-weight: {FONT_WEIGHT_REGULAR}; paint-order: stroke; stroke: #ffffff; stroke-width: {CONNECTION_LABEL_HALO}px; stroke-linejoin: round; text-anchor: start; }}
      .branch-label {{ fill: currentColor; font-weight: {FONT_WEIGHT_MEDIUM}; }}
      .parameter-link {{ fill: none; stroke: currentColor; stroke-width: {STROKE_WIDTH}; }}
      .node-shape {{ fill: #ffffff; stroke: currentColor; stroke-width: {STROKE_WIDTH}; }}
      .start .node-shape, .end .node-shape, .parameter-panel .node-shape {{ fill: #f0f9ff; }}
      .action .node-shape, .call .node-shape {{ fill: #f8fafc; }}
      .call-bars {{ fill: none; stroke: currentColor; stroke-width: {STROKE_WIDTH}; }}
      .loop .node-shape {{ fill: #f0fdf4; stroke-width: {CYCLE_NODE_STROKE_WIDTH}; }}
      .loop-marker {{ fill: #15803d; font-size: {LOOP_MARKER_FONT}px; font-weight: {FONT_WEIGHT_BOLD}; text-anchor: middle; }}
      .question .node-shape {{ fill: #fffbeb; }}
      .select .node-shape, .case .node-shape {{ fill: #f5f3ff; }}
      .label {{ fill: currentColor; font-size: {LABEL_FONT}px; text-anchor: middle; }}
      .start .label, .question .label, .select .label, .case .label, .end .label {{ font-weight: {FONT_WEIGHT_SEMIBOLD}; }}
      .action .label, .call .label, .loop .label {{ font-weight: {FONT_WEIGHT_MEDIUM}; text-anchor: start; }}
      .parameter-panel .label {{ font-weight: {FONT_WEIGHT_REGULAR}; text-anchor: start; }}
      .cycle-boundary {{ fill: #f0fdf433; stroke: #15803d; stroke-width: {CYCLE_STROKE_WIDTH}; stroke-dasharray: {CYCLE_DASH_LENGTH} {CYCLE_DASH_GAP}; }}
      .cycle-caption {{ fill: #166534; font-size: {CYCLE_CAPTION_FONT}px; font-weight: {FONT_WEIGHT_SEMIBOLD}; paint-order: stroke; stroke: #ffffff; stroke-width: {CYCLE_CAPTION_HALO}px; }}
{markdown_styles}{stage_styles}    </style>
  </defs>
{background}  <g class="cycle-regions">
"#
    );
    for region in &scene.cycle_regions {
        cycle::write(&mut svg, region);
    }
    svg.push_str("  </g>\n  <g class=\"connections\">\n");
    if !scene.topology.back_edges.is_empty() {
        emit!(
            svg,
            "    <defs><marker id=\"{back_edge_arrow_id}\" markerWidth=\"{ARROW_SIZE}\" markerHeight=\"{ARROW_SIZE}\" refX=\"{BACK_EDGE_ARROW_REF_X}\" refY=\"{ARROW_MIDPOINT}\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M 0 0 L {ARROW_SIZE} {ARROW_MIDPOINT} L 0 {ARROW_SIZE} Z\" fill=\"currentColor\"/></marker></defs>"
        );
    }
    // One stroke paints shared distributors and merge rails only once.
    svg.push_str("    <path class=\"connection\" d=\"\n");
    for connection in &scene.connections {
        if !scene.is_back_edge(connection) {
            write_connection(&mut svg, connection);
        }
    }
    svg.push_str("    \"/>\n");
    for connection in &scene.connections {
        if scene.is_back_edge(connection) {
            svg.push_str("    <path class=\"connection\" d=\"\n");
            write_connection(&mut svg, connection);
            emit!(svg, "    \" marker-end=\"url(#{back_edge_arrow_id})\"/>");
        }
    }
    // After the routes, so a label's halo covers the connections it crosses.
    for label in &scene.labels {
        write_connection_label(&mut svg, label);
    }
    svg.push_str("  </g>\n  <g class=\"nodes\">\n");
    if let Some(parameters) = &scene.parameters {
        write_parameter_panel(&mut svg, scene.node(NodeId::Start), parameters);
    }
    for junction in 0..scene.topology.junctions.len() {
        if !scene.topology.junctions[junction].merges.is_empty() {
            write_merge(&mut svg, scene, junction);
        }
    }
    for node in &scene.nodes {
        write_node(&mut svg, scene, node);
    }
    svg.push_str("  </g>\n</svg>\n");

    svg
}

fn markdown_styles(scene: &Scene) -> String {
    if scene
        .nodes
        .iter()
        .flat_map(|node| &node.lines)
        .chain(scene.labels.iter().flat_map(|label| &label.lines))
        .chain(
            scene
                .cycle_regions
                .iter()
                .flat_map(|region| &region.caption),
        )
        .any(|line| {
            line.spans()
                .iter()
                .any(|span| span.style != Style::default() || span.formula.is_some())
        })
    {
        format!(
            r#"      .md-code {{ font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace; font-weight: {FONT_WEIGHT_SEMIBOLD}; }}
      .md-bold {{ font-weight: {FONT_WEIGHT_HEAVY}; }}
      .md-italic {{ font-style: italic; }}
      .md-quote {{ font-style: italic; }}
      .md-strikethrough {{ text-decoration: line-through; }}
      .md-underline {{ text-decoration: underline; }}
      .md-strikethrough.md-underline {{ text-decoration: underline line-through; }}
      .md-highlight-box {{ fill: #fef08a; stroke: none; }}
      .md-superscript, .md-subscript {{ font-size: {script_font_percent}%; }}
      .md-superscript {{ baseline-shift: {superscript_shift}em; }}
      .md-subscript {{ baseline-shift: -{subscript_shift}em; }}
      .md-math {{ color: inherit; overflow: visible; }}
      .md-math path, .md-math rect:not([stroke]) {{ stroke: none; vector-effect: non-scaling-stroke; }}
      .md-math.md-bold path {{ stroke: currentColor; stroke-width: {FORMULA_BOLD_STROKE}px; stroke-linejoin: round; }}
      .md-math.md-bold g[stroke] path {{ stroke: inherit; }}
      .cycle-caption .md-math {{ color: #166534; }}
      .md-color-red, .md-math.md-color-red {{ fill: #b91c1c; color: #b91c1c; }}
      .md-color-green, .md-math.md-color-green {{ fill: #166534; color: #166534; }}
      .md-color-blue, .md-math.md-color-blue {{ fill: #1d4ed8; color: #1d4ed8; }}
      .md-color-purple, .md-math.md-color-purple {{ fill: #6d28d9; color: #6d28d9; }}
      .md-color-muted, .md-math.md-color-muted {{ fill: #475569; color: #475569; }}
"#,
            script_font_percent = SCRIPT_FONT_SCALE.0 * 100 / SCRIPT_FONT_SCALE.1,
            superscript_shift = f64::from(SUPERSCRIPT_SHIFT.0) / f64::from(SUPERSCRIPT_SHIFT.1),
            subscript_shift = f64::from(SUBSCRIPT_SHIFT.0) / f64::from(SUBSCRIPT_SHIFT.1),
        )
    } else {
        String::new()
    }
}

fn write_connection(svg: &mut String, connection: &Connection) {
    let first = connection
        .points
        .first()
        .expect("a routed connection has at least two points");
    emit_inline!(svg, "      M {} {}", first.x, first.y);
    for point in &connection.points[1..] {
        emit_inline!(svg, " L {} {}", point.x, point.y);
    }
    svg.push('\n');
}

fn write_connection_label(svg: &mut String, label: &Label) {
    let Point { x, y } = label.at;
    let class = match label.kind {
        LabelKind::Wire => "connection-label",
        LabelKind::Branch => "connection-label branch-label",
    };
    if needs_composed_lines(&label.lines) {
        let style = matches!(label.kind, LabelKind::Branch)
            .then(|| format!("font-size: {}px", label.kind.font_size()));
        write_composed_lines(
            svg,
            "    ",
            class,
            style.as_deref(),
            &label.lines,
            x,
            y,
            label.kind.font_size(),
            label.kind.line_height(),
            TextAnchor::Start,
        );
        return;
    }
    emit_inline!(svg, "    <text class=\"{class}\"");
    if matches!(label.kind, LabelKind::Branch) {
        emit_inline!(svg, " style=\"font-size: {}px\"", label.kind.font_size());
    }
    emit_inline!(svg, " x=\"{x}\" y=\"{y}\" xml:space=\"preserve\">");
    write_lines(
        svg,
        &label.lines,
        x,
        label.kind.font_size(),
        label.kind.line_height(),
    );
}

fn write_merge(svg: &mut String, scene: &Scene, junction: usize) {
    let point = scene
        .junction_at(junction)
        .expect("a merge has an incident route");
    emit!(
        svg,
        "    <g class=\"node merge\" transform=\"translate({} {})\">",
        point.x,
        point.y
    );
    emit!(
        svg,
        "      <title xml:space=\"preserve\">{}</title>",
        escape(&merge_name(scene, junction))
    );
    emit!(
        svg,
        "      <circle class=\"node-shape\" r=\"{MERGE_RADIUS}\" style=\"fill: currentColor\"/>"
    );
    svg.push_str("    </g>\n");
}

/// Writes the lines of one `<text>` as `<tspan>`s and closes it. The first line
/// sits on the text's own baseline; each later one drops by `line_height`.
fn write_lines(svg: &mut String, lines: &[RichText], x: i32, font_size: i32, line_height: i32) {
    let metrics = block_metrics(lines, font_size, line_height);
    for (index, line) in lines.iter().enumerate() {
        let dy = if index == 0 {
            0
        } else {
            metrics.baselines[index] - metrics.baselines[index - 1]
        };
        emit_inline!(svg, "<tspan x=\"{x}\" dy=\"{dy}\">");
        for (span, _, _) in shaping_spans(line, font_size) {
            write_span(svg, &span.text, span.style);
        }
        svg.push_str("</tspan>");
    }
    emit!(svg, "</text>");
}

fn write_span(svg: &mut String, text: &str, style: Style) {
    let classes = [
        (style.bold, "md-bold"),
        (style.italic, "md-italic"),
        (style.quote, "md-quote"),
        (style.strikethrough, "md-strikethrough"),
        (style.underline, "md-underline"),
        (style.color.is_some(), style.color.map_or("", Color::class)),
        (style.code, "md-code"),
        (style.script == Some(Script::Superscript), "md-superscript"),
        (style.script == Some(Script::Subscript), "md-subscript"),
    ]
    .into_iter()
    .filter_map(|(on, class)| on.then_some(class))
    .collect::<Vec<_>>();
    // A highlight is the rect behind the run, so a span carrying only that
    // effect, like an unstyled one, needs no element of its own.
    if classes.is_empty() {
        svg.push_str(&escape(text));
        return;
    }
    emit_inline!(
        svg,
        "<tspan class=\"{}\">{}</tspan>",
        classes.join(" "),
        escape(text)
    );
}

#[derive(Clone, Copy)]
enum TextAnchor {
    Start,
    Middle,
    End,
}

fn needs_composed_lines(lines: &[RichText]) -> bool {
    lines
        .iter()
        .flat_map(RichText::spans)
        .any(|span| span.formula.is_some() || span.style.highlight)
}

#[allow(clippy::too_many_arguments)]
fn write_composed_lines(
    svg: &mut String,
    indent: &str,
    class: &str,
    style: Option<&str>,
    lines: &[RichText],
    x: i32,
    first_y: i32,
    font_size: i32,
    line_height: i32,
    anchor: TextAnchor,
) {
    emit_inline!(svg, "{indent}<g class=\"{class}\"");
    if let Some(style) = style {
        emit_inline!(svg, " style=\"{style}\"");
    }
    emit!(svg, ">");
    let metrics = block_metrics(lines, font_size, line_height);
    let first_baseline = metrics.baselines.first().copied().unwrap_or_default();
    for (index, line) in lines.iter().enumerate() {
        let baseline = first_y + metrics.baselines[index] - first_baseline;
        let shaped = shaping_spans(line, font_size);
        let width = Dim::ratio(
            shaped.iter().map(|(_, advance, _)| *advance).sum(),
            TEXT_ADVANCE_SCALE,
        );
        let x = Dim::from_i64(i64::from(x));
        let start = match anchor {
            TextAnchor::Start => x,
            TextAnchor::Middle => x - width / Dim::from_i64(2),
            TextAnchor::End => x - width,
        };
        let mut decorated_x = start.clone();
        for (span, advance, _) in &shaped {
            let advance = Dim::ratio(*advance, TEXT_ADVANCE_SCALE);
            if span.style.highlight && !advance.is_zero() {
                let (ascent, descent) = line_ink(std::slice::from_ref(span), font_size);
                let ascent = Dim::from_i64(i64::from(ascent));
                let descent = Dim::from_i64(i64::from(descent));
                let top = Dim::from_i64(i64::from(baseline)) - &ascent;
                let height = ascent + descent;
                emit!(
                    svg,
                    "{indent}  <rect class=\"md-highlight-box\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                    svg_dimension(&decorated_x),
                    svg_dimension(&top),
                    svg_dimension(&advance),
                    svg_dimension(&height)
                );
            }
            decorated_x = decorated_x + advance;
        }
        let mut cursor = start;
        for run in shaped.chunk_by(|(left, _, left_index), (right, _, right_index)| {
            left.formula.is_none()
                && right.formula.is_none()
                && left.style.highlight == right.style.highlight
                && (!left.style.highlight || left_index == right_index)
        }) {
            let width = Dim::ratio(
                run.iter().map(|(_, advance, _)| *advance).sum(),
                TEXT_ADVANCE_SCALE,
            );
            let first = &run[0].0;
            if let Some(formula) = &first.formula {
                write_formula(
                    svg,
                    indent,
                    formula,
                    first.style,
                    &cursor,
                    baseline,
                    font_size,
                );
            } else {
                emit_inline!(svg, "{indent}  <text");
                if first.style.highlight {
                    emit_inline!(svg, " style=\"stroke: none\"");
                }
                // The run must occupy exactly the width it was measured at, or
                // the next run and the highlight box drift off its ink. The
                // estimate errs wide, so spacing uses the gaps when there
                // are any; a single grapheme must scale its outline instead.
                emit_inline!(
                    svg,
                    " x=\"{}\" y=\"{baseline}\" text-anchor=\"start\" textLength=\"{}\" lengthAdjust=\"{}\" xml:space=\"preserve\">",
                    svg_dimension(&cursor),
                    svg_dimension(&width),
                    if run.len() == 1 && run[0].0.text.graphemes(true).count() == 1 {
                        "spacingAndGlyphs"
                    } else {
                        "spacing"
                    }
                );
                for (span, _, _) in run {
                    write_span(svg, &span.text, span.style);
                }
                emit!(svg, "</text>");
            }
            cursor = cursor + width;
        }
    }
    emit!(svg, "{indent}</g>");
}

#[allow(clippy::too_many_lines)] // Emits the formula viewport, styles and decorations together.
fn write_formula(
    svg: &mut String,
    indent: &str,
    formula: &Formula,
    style: Style,
    x: &Dim,
    baseline: i32,
    font_size: i32,
) {
    let width = formula.width(font_size, style);
    let visible_width = formula.visible_width(font_size, style);
    let ascent = formula.ascent(font_size, style);
    let height = &ascent + &formula.descent(font_size, style);
    let top = &Dim::from_i64(i64::from(baseline)) - &ascent;
    let top_padding = i32::from(formula.is_clipped()) * FORMULA_CLIP_PADDING;
    let bottom_padding = if formula.is_clipped() {
        FORMULA_CLIP_PADDING
            + if style.underline {
                FORMULA_UNDERLINE_PADDING
            } else {
                0
            }
    } else {
        0
    };
    let mut class = style.color.map_or_else(
        || "md-math".to_owned(),
        |color| format!("md-math {}", color.class()),
    );
    if style.bold {
        class.push_str(" md-bold");
    }
    let overflow = if formula.is_clipped() {
        " style=\"overflow: hidden\""
    } else {
        ""
    };
    let stroke = if style.highlight {
        " stroke=\"none\""
    } else {
        ""
    };
    if height.is_zero() {
        let end = x + &visible_width;
        for (draw, y) in [
            (
                style.underline,
                &top + Dim::ratio(
                    i64::from(font_size) * i64::from(FORMULA_UNDERLINE_OFFSET.0),
                    FORMULA_UNDERLINE_OFFSET.1.into(),
                ),
            ),
            (
                style.strikethrough,
                &top - Dim::ratio(
                    i64::from(font_size) * i64::from(FORMULA_STRIKETHROUGH_OFFSET.0),
                    FORMULA_STRIKETHROUGH_OFFSET.1.into(),
                ),
            ),
        ] {
            if draw && !visible_width.is_zero() {
                emit!(
                    svg,
                    r#"{indent}  <line class="{class}" x1="{}" y1="{}" x2="{}" y2="{}" stroke="currentColor" stroke-width="{FORMULA_DECORATION_STROKE}"/>"#,
                    svg_dimension(x),
                    svg_dimension(&y),
                    svg_dimension(&end),
                    svg_dimension(&y)
                );
            }
        }
    } else if !visible_width.is_zero() {
        emit!(
            svg,
            "{indent}  <svg class=\"{class}\"{overflow}{stroke} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"{}\" aria-hidden=\"true\">",
            svg_dimension(x),
            svg_dimension(&(&top - Dim::from_i64(i64::from(top_padding)))),
            svg_dimension(&visible_width),
            svg_dimension(&(&height + Dim::from_i64(i64::from(top_padding + bottom_padding)))),
            formula.view_box(font_size, style, top_padding, bottom_padding)
        );
        if style.italic || style.quote {
            emit!(
                svg,
                "{indent}    <g transform=\"translate({} 0) skewX({FORMULA_SKEW_DEGREES})\">",
                svg_dimension(&formula.slant(style))
            );
        }
        // The body is renderer-owned markup: latex-rust emits only shapes with
        // numeric attributes, and no TeX source reaches it.
        svg.push_str(formula.svg_body());
        if style.italic || style.quote {
            emit!(svg, "{indent}    </g>");
        }
        if style.underline {
            let below = svg_dimension(
                &(formula.view_box_height()
                    + Dim::ratio(
                        FORMULA_UNDERLINE_OFFSET.0.into(),
                        FORMULA_UNDERLINE_OFFSET.1.into(),
                    )),
            );
            let x2 = formula.view_box_width(style);
            emit!(
                svg,
                "{indent}    <line x1=\"0\" y1=\"{below}\" x2=\"{}\" y2=\"{below}\" stroke=\"currentColor\" stroke-width=\"{FORMULA_DECORATION_STROKE}\" vector-effect=\"non-scaling-stroke\"/>",
                svg_dimension(&x2)
            );
        }
        if style.strikethrough {
            let across = svg_dimension(&(formula.view_box_height() * Dim::ratio(1, 2)));
            emit!(
                svg,
                "{indent}    <line x1=\"0\" y1=\"{across}\" x2=\"{}\" y2=\"{across}\" stroke=\"currentColor\" stroke-width=\"{FORMULA_DECORATION_STROKE}\" vector-effect=\"non-scaling-stroke\"/>",
                svg_dimension(&formula.view_box_width(style))
            );
        }
        emit!(svg, "{indent}  </svg>");
    }
    if formula.is_clipped() {
        emit!(
            svg,
            "{indent}  <text class=\"md-math-ellipsis\"{stroke} x=\"{}\" y=\"{baseline}\" text-anchor=\"start\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\" aria-hidden=\"true\">…</text>",
            svg_dimension(&(x + &visible_width)),
            svg_dimension(&(&width - &visible_width))
        );
    }
}

/// Describes the diagram once: each node with the labels it and its exits own,
/// then each connection as the pair of ends it joins.
fn describe(scene: &Scene) -> String {
    let topology = &scene.topology;
    let mut nodes = topology
        .nodes
        .iter()
        .map(|node| {
            let mut described = node_name(scene, node.id);
            if node.id == NodeId::Start
                && let Some(parameters) = &scene.parameters
            {
                push_phrase(
                    &mut described,
                    " with parameters ",
                    &parameters.parameters,
                    "; ",
                );
            }
            let capture = scene.captions.capture_label(node.id);
            if capture == ["()"] {
                described.push_str(" capturing nothing");
            } else {
                push_phrase(&mut described, " capturing ", capture, ", ");
            }
            let (mut handovers, mut branches) = (Vec::new(), Vec::new());
            for exit in topology.exits.iter().filter(|exit| exit.id.node == node.id) {
                let handover = scene.captions.handover(exit.id);
                if !handover.is_empty() {
                    handovers.push(handover.join(", "));
                }
                if let Some(description) = scene.captions.branch_description(exit.id) {
                    branches.push(format!(
                        "branch {}: {}",
                        exit.id
                            .branch
                            .expect("only question branches have descriptions")
                            + 1,
                        description.as_ref()
                    ));
                }
            }
            push_phrase(&mut described, " handing over ", &handovers, " / ");
            push_phrase(&mut described, " described as ", &branches, " / ");
            described
        })
        .collect::<Vec<_>>();
    nodes.extend(
        topology
            .junctions
            .iter()
            .enumerate()
            .filter(|(_, junction)| !junction.merges.is_empty())
            .map(|(junction, _)| {
                format!(
                    "Merge: {}",
                    scene.captions.junction_wires(junction).join(", ")
                )
            }),
    );
    let nodes = nodes.join("; ");
    let connections = scene
        .connections
        .iter()
        .map(|connection| {
            format!(
                "{}{} to {}",
                if scene.is_back_edge(connection) {
                    "Repeat from "
                } else {
                    ""
                },
                source_name(scene, connection.source),
                destination_name(scene, connection.destination)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let cycles = scene
        .cycle_regions
        .iter()
        .map(|region| {
            format!(
                "{} with inputs {} and outputs {}",
                region.description, region.inputs, region.outputs
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    if cycles.is_empty() && connections.is_empty() {
        format!("Nodes: {nodes}.")
    } else if cycles.is_empty() {
        format!("Nodes: {nodes}. Connections: {connections}.")
    } else {
        format!("Cycles: {cycles}. Nodes: {nodes}. Connections: {connections}.")
    }
}

/// Appends `lead` and the joined `items`, or nothing when there are none.
fn push_phrase(described: &mut String, lead: &str, items: &[String], separator: &str) {
    if items.is_empty() {
        return;
    }
    described.push_str(lead);
    described.push_str(&items.join(separator));
}

fn write_parameter_panel(svg: &mut String, start: &Node, parameters: &ParameterPanel) {
    let left = parameters.x - parameters.width / 2;
    let right = start.x + start.width / 2;
    emit!(
        svg,
        "    <path class=\"parameter-link\" d=\"M {right} {} L {left} {}\"/>",
        start.y,
        parameters.y
    );
    emit!(
        svg,
        "    <g class=\"parameter-panel\" transform=\"translate({} {})\">",
        parameters.x,
        parameters.y
    );
    emit!(
        svg,
        "      <rect class=\"node-shape\" x=\"-{}\" y=\"-{}\" width=\"{}\" height=\"{}\"/>",
        parameters.width / 2,
        parameters.height / 2,
        parameters.width,
        parameters.height
    );
    let first_y = NODE_LABEL_PADDING_Y - parameters.lines.len() as i32 * LINE_HEIGHT / 2;
    emit_inline!(
        svg,
        "      <text class=\"label\" x=\"{}\" y=\"{first_y}\" xml:space=\"preserve\">",
        NODE_LABEL_PADDING_X - parameters.width / 2
    );
    write_lines(
        svg,
        &parameters.lines,
        NODE_LABEL_PADDING_X - parameters.width / 2,
        LABEL_FONT,
        LINE_HEIGHT,
    );
    svg.push_str("    </g>\n");
}

fn source_name(scene: &Scene, source: Source) -> String {
    match source {
        // A collapsed cycle's branch exits are its declared outputs.
        Source::Exit(ExitId {
            node: node @ NodeId::Block(block),
            branch: Some(output),
        }) if scene.topology.node(node).kind == NodeKind::Cycle => format!(
            "{} output {}",
            node_name(scene, node),
            scene.captions.cycle_output(block, output)
        ),
        Source::Exit(ExitId {
            node,
            branch: Some(branch),
        }) => format!("{} branch {}", node_name(scene, node), branch + 1),
        Source::Exit(exit) => node_name(scene, exit.node),
        Source::Junction(junction) => merge_name(scene, junction),
    }
}

fn destination_name(scene: &Scene, destination: Destination) -> String {
    match destination {
        Destination::Node(node) => node_name(scene, node),
        Destination::Junction(junction) => merge_name(scene, junction),
    }
}

fn merge_name(scene: &Scene, junction: usize) -> String {
    let wires = scene.captions.junction_wires(junction);
    if let Some(cycle) = scene
        .topology
        .cycles
        .iter()
        .find(|cycle| cycle.tail == junction || cycle.entry == junction)
    {
        let owner = if scene
            .topology
            .nodes
            .iter()
            .any(|node| node.id == NodeId::Block(cycle.header))
        {
            node_name(scene, NodeId::Block(cycle.header))
        } else {
            "the cycle".to_owned()
        };
        return if cycle.tail != junction {
            format!("the entry of {owner}")
        } else if wires.is_empty() {
            format!("the iteration tail of {owner}")
        } else {
            // A merge that feeds only the continue is drawn as the tail.
            format!(
                "the {} merge at the iteration tail of {owner}",
                wires.join(" and ")
            )
        };
    }
    // A cycle with several outputs names the one each result hands over.
    if let Some((boundary, output)) = scene.topology.cycle_boundaries.iter().find_map(|boundary| {
        let output = boundary
            .results
            .iter()
            .position(|&result| result == Source::Junction(junction))?;
        (boundary.results.len() > 1).then_some((boundary, output))
    }) {
        let name = scene.captions.cycle_output(boundary.header, output);
        return if wires.is_empty() {
            format!("the {name} result of the cycle")
        } else {
            format!(
                "the {} merge at the {name} result of the cycle",
                wires.join(" and ")
            )
        };
    }
    let junction = &scene.topology.junctions[junction];
    if junction.is_cycle_result && !wires.is_empty() {
        format!("the {} merge at the cycle result", wires.join(" and "))
    } else if junction.is_cycle_result {
        "the cycle result".to_owned()
    } else if junction.is_cycle_entry {
        "the cycle entry".to_owned()
    } else if wires.is_empty() {
        "a structural junction".to_owned()
    } else {
        format!("the {} merge", wires.join(" and "))
    }
}

fn node_name(scene: &Scene, id: NodeId) -> String {
    let label = scene.captions.label(id).as_ref();
    match scene.topology.node(id).kind {
        NodeKind::Start => format!("Start: {label}"),
        NodeKind::StageEntry => format!("Stage: {label}"),
        NodeKind::Action => action::name(label),
        NodeKind::Call => call::name(label),
        NodeKind::Cycle => format!("Cycle: {label}"),
        NodeKind::Question => question::name(label),
        NodeKind::Select => choice::select_name(label),
        NodeKind::Case => choice::case_name(label),
        NodeKind::End => format!("End: {label}"),
        NodeKind::Transition => format!("Transition: {label}"),
    }
}

fn write_node(svg: &mut String, scene: &Scene, node: &Node) {
    let projected = scene.topology.node(node.id);
    let class = node_class(projected.kind);
    emit!(
        svg,
        "    <g class=\"node {class}\" transform=\"translate({} {})\">",
        node.x,
        node.y
    );
    if !matches!(projected.kind, NodeKind::Start | NodeKind::End) {
        write_title(svg, scene.captions.label(node.id).as_ref());
    }
    match projected.kind {
        // Start and end are the two ends of one flow, drawn alike.
        NodeKind::Start | NodeKind::End => {
            write_capsule(svg, node);
            write_label(svg, node, 0, 0, TextAnchor::Middle);
        }
        NodeKind::Action => action::write(svg, node),
        NodeKind::Call => call::write(svg, node),
        NodeKind::Cycle => cycle::write_node(svg, node),
        NodeKind::Question => question::write(svg, node),
        NodeKind::Select => choice::write_select(svg, node),
        NodeKind::Case | NodeKind::StageEntry => choice::write_case(svg, node),
        NodeKind::Transition => stage::write_transition(svg, node),
    }
    if scene.captions.back_marker(node.id) {
        stage::write_marker(svg, node, projected.kind);
    }
    svg.push_str("    </g>\n");
}

fn write_capsule(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    emit!(
        svg,
        "      <rect class=\"node-shape\" x=\"-{}\" y=\"-{}\" width=\"{}\" height=\"{}\" rx=\"{}\"/>",
        half_width,
        half_height,
        node.width,
        node.height,
        half_height
    );
}

fn write_title(svg: &mut String, label: &str) {
    emit!(
        svg,
        "      <title xml:space=\"preserve\">{}</title>",
        escape(label)
    );
}

/// Draws one node's caption. The anchor matches the `text-anchor` the node's
/// class carries in the stylesheet; the composed path positions runs itself and
/// cannot read it from the CSS.
fn write_label(svg: &mut String, node: &Node, center_y: i32, x: i32, anchor: TextAnchor) {
    let metrics = block_metrics(&node.lines, LABEL_FONT, LINE_HEIGHT);
    let first_y = center_y - metrics.height / 2 + metrics.baselines[0];
    if needs_composed_lines(&node.lines) {
        write_composed_lines(
            svg,
            "      ",
            "label",
            None,
            &node.lines,
            x,
            first_y,
            LABEL_FONT,
            LINE_HEIGHT,
            anchor,
        );
        return;
    }
    emit_inline!(
        svg,
        "      <text class=\"label\" y=\"{first_y}\" xml:space=\"preserve\">"
    );
    write_lines(svg, &node.lines, x, LABEL_FONT, LINE_HEIGHT);
}

const fn node_class(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Start => "start",
        NodeKind::StageEntry => "stage-entry",
        NodeKind::Action => "action",
        NodeKind::Call => "call",
        NodeKind::Cycle => "loop",
        NodeKind::Question => "question",
        NodeKind::Select => "select",
        NodeKind::Case => "case",
        NodeKind::End => "end",
        NodeKind::Transition => "transition",
    }
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\'' => escaped.push_str("&apos;"),
            '"' => escaped.push_str("&quot;"),
            '\r' => escaped.push_str("&#13;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn escapes_xml_text() {
        assert_eq!(escape("<&>'\"\r"), "&lt;&amp;&gt;&apos;&quot;&#13;");
    }
}
