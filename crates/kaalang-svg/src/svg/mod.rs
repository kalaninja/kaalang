use std::fmt::Write;

use crate::layout::{
    CONNECTION_LABEL_FONT, CONNECTION_LABEL_HALO, CYCLE_CAPTION_FONT, Connection, LABEL_FONT,
    LINE_HEIGHT, Label, LabelKind, MERGE_RADIUS, Node, ParameterPanel, Point, Scene,
};
use crate::text::{
    Formula, RichText, Script, Style, block_metrics, line_ink, shaping_spans, span_advances,
    svg_dimension,
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
mod loop_block;
mod question;

pub(crate) fn serialize(scene: &Scene, flow_name: &str) -> String {
    let mut svg = String::new();
    let markdown_styles = markdown_styles(scene);
    emit!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img" aria-labelledby="kaalang-title" aria-describedby="kaalang-description">"#,
        scene.width,
        scene.height,
        scene.width,
        scene.height
    );
    emit!(
        svg,
        "  <title id=\"kaalang-title\">kaalang diagram for {}</title>",
        escape(flow_name)
    );
    emit!(
        svg,
        "  <desc id=\"kaalang-description\">{}</desc>",
        escape(&describe(scene))
    );
    emit_inline!(
        svg,
        r##"  <defs>
    <style>
      svg {{ color: #1f2937; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; text-rendering: optimizeLegibility; }}
      .connection {{ fill: none; stroke: currentColor; stroke-width: 1.75; stroke-linecap: square; stroke-linejoin: round; }}
      .connection-label {{ fill: #64748b; font-size: {CONNECTION_LABEL_FONT}px; font-weight: 400; paint-order: stroke; stroke: #ffffff; stroke-width: {CONNECTION_LABEL_HALO}px; stroke-linejoin: round; text-anchor: start; }}
      .branch-label {{ fill: currentColor; font-weight: 500; }}
      .parameter-link {{ fill: none; stroke: currentColor; stroke-width: 1.75; }}
      .node-shape {{ fill: #ffffff; stroke: currentColor; stroke-width: 1.75; }}
      .start .node-shape, .end .node-shape, .parameter-panel .node-shape {{ fill: #f0f9ff; }}
      .action .node-shape, .call .node-shape {{ fill: #f8fafc; }}
      .call-bars {{ fill: none; stroke: currentColor; stroke-width: 1.75; }}
      .loop .node-shape {{ fill: #f0fdf4; stroke-width: 2; }}
      .loop-marker {{ fill: #15803d; font-size: 20px; font-weight: 700; text-anchor: middle; }}
      .question .node-shape {{ fill: #fffbeb; }}
      .select .node-shape, .case .node-shape {{ fill: #f5f3ff; }}
      .label {{ fill: currentColor; font-size: {LABEL_FONT}px; text-anchor: middle; }}
      .start .label, .question .label, .select .label, .case .label, .end .label {{ font-weight: 600; }}
      .action .label, .call .label, .loop .label {{ font-weight: 500; text-anchor: start; }}
      .parameter-panel .label {{ font-weight: 400; text-anchor: start; }}
      .cycle-boundary {{ fill: #f0fdf433; stroke: #15803d; stroke-width: 1.5; stroke-dasharray: 7 5; }}
      .cycle-caption {{ fill: #166534; font-size: {CYCLE_CAPTION_FONT}px; font-weight: 600; paint-order: stroke; stroke: #ffffff; stroke-width: 4px; }}
{markdown_styles}    </style>
  </defs>
  <rect width="100%" height="100%" fill="#ffffff"/>
  <g class="cycle-regions">
"##
    );
    for region in &scene.loop_regions {
        loop_block::write(&mut svg, region);
    }
    svg.push_str("  </g>\n  <g class=\"connections\">\n");
    if !scene.topology.back_edges.is_empty() {
        svg.push_str("    <defs><marker id=\"loop-arrow\" markerWidth=\"10\" markerHeight=\"10\" refX=\"9\" refY=\"5\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M 0 0 L 10 5 L 0 10 Z\" fill=\"currentColor\"/></marker></defs>\n");
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
            svg.push_str("    \" marker-end=\"url(#loop-arrow)\"/>\n");
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

fn markdown_styles(scene: &Scene) -> &'static str {
    if scene
        .nodes
        .iter()
        .flat_map(|node| &node.lines)
        .chain(scene.labels.iter().flat_map(|label| &label.lines))
        .chain(scene.loop_regions.iter().flat_map(|region| &region.caption))
        .any(|line| {
            line.spans()
                .iter()
                .any(|span| span.style != Style::default() || span.formula.is_some())
        })
    {
        r#"      .md-code { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace; font-weight: 600; }
      .md-bold { font-weight: 800; }
      .md-italic { font-style: italic; }
      .md-quote { font-style: italic; }
      .md-strikethrough { text-decoration: line-through; }
      .md-underline { text-decoration: underline; }
      .md-strikethrough.md-underline { text-decoration: underline line-through; }
      .md-highlight-box { fill: #fef08a; stroke: none; }
      .md-superscript, .md-subscript { font-size: 75%; }
      .md-superscript { baseline-shift: 0.4em; }
      .md-subscript { baseline-shift: -0.2em; }
      .md-math { color: inherit; overflow: visible; }
      .md-math path, .md-math rect:not([stroke]) { stroke: none; vector-effect: non-scaling-stroke; }
      .md-math.md-bold path { stroke: currentColor; stroke-width: 0.7px; stroke-linejoin: round; }
      .md-math.md-bold g[stroke] path { stroke: inherit; }
      .cycle-caption .md-math { color: #166534; }
      .md-color-red, .md-math.md-color-red { fill: #b91c1c; color: #b91c1c; }
      .md-color-green, .md-math.md-color-green { fill: #166534; color: #166534; }
      .md-color-blue, .md-math.md-color-blue { fill: #1d4ed8; color: #1d4ed8; }
      .md-color-purple, .md-math.md-color-purple { fill: #6d28d9; color: #6d28d9; }
      .md-color-muted, .md-math.md-color-muted { fill: #475569; color: #475569; }
"#
    } else {
        ""
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
        for span in line.spans() {
            write_span(svg, &span.text, span.style);
        }
        svg.push_str("</tspan>");
    }
    emit!(svg, "</text>");
}

fn write_span(svg: &mut String, text: &str, style: Style) {
    if style == Style::default() {
        svg.push_str(&escape(text));
        return;
    }
    let mut classes = Vec::new();
    if style.bold {
        classes.push("md-bold");
    }
    if style.italic {
        classes.push("md-italic");
    }
    if style.quote {
        classes.push("md-quote");
    }
    if style.strikethrough {
        classes.push("md-strikethrough");
    }
    if style.underline {
        classes.push("md-underline");
    }
    if let Some(color) = style.color {
        classes.push(color.class());
    }
    if style.code {
        classes.push("md-code");
    }
    match style.script {
        Some(Script::Superscript) => classes.push("md-superscript"),
        Some(Script::Subscript) => classes.push("md-subscript"),
        None => {}
    }
    // A highlight is the rect behind the run, so a span carrying only that
    // effect needs no element of its own.
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
        let advances = span_advances(line, font_size);
        let width = Dim::ratio(advances.iter().copied().sum(), 10_000);
        let spans = line.spans().iter().zip(&advances).collect::<Vec<_>>();
        let x = Dim::from_i64(i64::from(x));
        let start = match anchor {
            TextAnchor::Start => x,
            TextAnchor::Middle => x - width / Dim::from_i64(2),
            TextAnchor::End => x - width,
        };
        let mut decorated_x = start.clone();
        for (span, advance) in &spans {
            let advance = Dim::ratio(**advance, 10_000);
            if span.style.highlight && !advance.is_zero() {
                let (ascent, descent) =
                    line_ink(&RichText::from_spans(std::slice::from_ref(span)), font_size);
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
        let shaped = shaping_spans(line, font_size);
        let mut cursor = start;
        for run in shaped.chunk_by(|(left, _, left_index), (right, _, right_index)| {
            left.formula.is_none()
                && right.formula.is_none()
                && left.style.highlight == right.style.highlight
                && (!left.style.highlight || left_index == right_index)
        }) {
            let width = Dim::ratio(run.iter().map(|(_, advance, _)| *advance).sum(), 10_000);
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
    let top_padding = i32::from(formula.is_clipped());
    let bottom_padding = if formula.is_clipped() {
        1 + if style.underline { 2 } else { 0 }
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
    if !visible_width.is_zero() {
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
            let slant = &formula.view_box_width(style) - formula.view_box_width(Style::default());
            emit!(
                svg,
                "{indent}    <g transform=\"translate({} 0) skewX(-8.5308)\">",
                svg_dimension(&slant)
            );
        }
        // The body is renderer-owned markup: latex-rust emits only shapes with
        // numeric attributes, and no TeX source reaches it.
        svg.push_str(formula.svg_body());
        if style.italic || style.quote {
            emit!(svg, "{indent}    </g>");
        }
        if style.underline {
            let below = svg_dimension(&(formula.view_box_height() + Dim::ratio(1, 10)));
            let x2 = formula.view_box_width(style);
            emit!(
                svg,
                "{indent}    <line x1=\"0\" y1=\"{below}\" x2=\"{}\" y2=\"{below}\" stroke=\"currentColor\" stroke-width=\"1\" vector-effect=\"non-scaling-stroke\"/>",
                svg_dimension(&x2)
            );
        }
        if style.strikethrough {
            let across = svg_dimension(&(formula.view_box_height() * Dim::ratio(1, 2)));
            emit!(
                svg,
                "{indent}    <line x1=\"0\" y1=\"{across}\" x2=\"{}\" y2=\"{across}\" stroke=\"currentColor\" stroke-width=\"1\" vector-effect=\"non-scaling-stroke\"/>",
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
        .loop_regions
        .iter()
        .map(|region| {
            format!(
                "{} with inputs {} and outputs {}",
                region.description, region.inputs, region.outputs
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    if cycles.is_empty() {
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
    let first_y = 15 - parameters.lines.len() as i32 * LINE_HEIGHT / 2;
    emit_inline!(
        svg,
        "      <text class=\"label\" x=\"{}\" y=\"{first_y}\" xml:space=\"preserve\">",
        16 - parameters.width / 2
    );
    write_lines(
        svg,
        &parameters.lines,
        16 - parameters.width / 2,
        LABEL_FONT,
        LINE_HEIGHT,
    );
    svg.push_str("    </g>\n");
}

fn source_name(scene: &Scene, source: Source) -> String {
    match source {
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
    if let Some(loop_) = scene
        .topology
        .loops
        .iter()
        .find(|loop_| loop_.tail == junction || loop_.entry == junction)
    {
        let owner = if scene
            .topology
            .nodes
            .iter()
            .any(|node| node.id == NodeId::Block(loop_.header))
        {
            node_name(scene, NodeId::Block(loop_.header))
        } else {
            "the cycle".to_owned()
        };
        let part = if loop_.tail == junction {
            "iteration tail"
        } else {
            "entry"
        };
        return format!("the {part} of {owner}");
    }
    let wires = scene.captions.junction_wires(junction);
    let junction = &scene.topology.junctions[junction];
    if junction.is_loop_result && !wires.is_empty() {
        format!("the {} merge at the cycle result", wires.join(" and "))
    } else if junction.is_loop_result {
        "the cycle result".to_owned()
    } else if junction.is_break {
        "a cycle break".to_owned()
    } else if junction.is_loop_entry {
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
        NodeKind::Action => action::name(label),
        NodeKind::Call => call::name(label),
        NodeKind::Loop => format!("Cycle: {label}"),
        NodeKind::Question => question::name(label),
        NodeKind::Select => choice::select_name(label),
        NodeKind::Case => choice::case_name(label),
        NodeKind::End => format!("End: {label}"),
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
        NodeKind::Loop => loop_block::write_node(svg, node),
        NodeKind::Question => question::write(svg, node),
        NodeKind::Select => choice::write_select(svg, node),
        NodeKind::Case => choice::write_case(svg, node),
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
        NodeKind::Action => "action",
        NodeKind::Call => "call",
        NodeKind::Loop => "loop",
        NodeKind::Question => "question",
        NodeKind::Select => "select",
        NodeKind::Case => "case",
        NodeKind::End => "end",
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
