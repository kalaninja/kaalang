use std::fmt::Write;

use crate::layout::{
    EDGE_LABEL_FONT, EDGE_LABEL_HALO, EDGE_LINE_HEIGHT, Edge, LABEL_FONT, LINE_HEIGHT, Node,
    NodeId, NodeKind, Scene,
};

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
mod choice;
mod end;
mod question;

pub(crate) fn serialize(scene: &Scene, flow_name: &str) -> String {
    let mut svg = String::new();
    emit!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img" aria-labelledby="contour-title" aria-describedby="contour-description">"#,
        scene.width,
        scene.height,
        scene.width,
        scene.height
    );
    emit!(
        svg,
        "  <title id=\"contour-title\">Contour diagram for {}</title>",
        escape(flow_name)
    );
    emit!(
        svg,
        "  <desc id=\"contour-description\">{}</desc>",
        escape(&describe(scene))
    );
    emit_inline!(
        svg,
        r##"  <defs>
    <style>
      svg {{ color: #1f2937; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; text-rendering: optimizeLegibility; }}
      .edge {{ fill: none; stroke: currentColor; stroke-width: 1.75; stroke-linecap: square; stroke-linejoin: round; }}
      .edge-label {{ fill: currentColor; font-size: {EDGE_LABEL_FONT}px; font-weight: 500; paint-order: stroke; stroke: #ffffff; stroke-width: {EDGE_LABEL_HALO}px; stroke-linejoin: round; text-anchor: middle; }}
      .node-shape {{ fill: #ffffff; stroke: currentColor; stroke-width: 1.75; }}
      .label {{ fill: currentColor; font-size: {LABEL_FONT}px; text-anchor: middle; }}
      .start .label, .question .label, .select .label, .case .label, .end .label {{ font-weight: 600; }}
      .action .label {{ font-weight: 400; text-anchor: start; }}
    </style>
  </defs>
  <rect width="100%" height="100%" fill="#ffffff"/>
  <g class="edges">
"##
    );
    for edge in &scene.edges {
        write_edge(&mut svg, edge);
    }
    svg.push_str("  </g>\n  <g class=\"nodes\">\n");
    for node in &scene.nodes {
        write_node(&mut svg, node);
    }
    svg.push_str("  </g>\n</svg>\n");

    svg
}

fn write_edge(svg: &mut String, edge: &Edge) {
    let first = edge
        .points
        .first()
        .expect("a positioned edge has at least two points");
    emit_inline!(
        svg,
        "    <path class=\"edge\" d=\"M {} {}",
        first.x,
        first.y
    );
    for point in &edge.points[1..] {
        emit_inline!(svg, " L {} {}", point.x, point.y);
    }
    emit!(svg, "\"/>");
    // Empty exactly when the connection carries no label: layout wraps every
    // label it sets, and a wrapped label always has at least one line.
    if edge.lines.is_empty() {
        return;
    }
    let lines = &edge.lines;
    let label_at = edge
        .label_at
        .expect("a labelled edge has a positioned label");
    let x = label_at.x;
    let first_y = label_at.y - (lines.len() as i32 - 1) * EDGE_LINE_HEIGHT / 2;
    emit_inline!(
        svg,
        "    <text class=\"edge-label\" x=\"{x}\" y=\"{first_y}\" xml:space=\"preserve\">"
    );
    for (index, line) in lines.iter().enumerate() {
        let dy = if index == 0 { 0 } else { EDGE_LINE_HEIGHT };
        emit_inline!(svg, "<tspan x=\"{x}\" dy=\"{dy}\">{}</tspan>", escape(line));
    }
    emit!(svg, "</text>");
}

fn describe(scene: &Scene) -> String {
    let mut description = String::from("Nodes: ");
    for (index, node) in scene.nodes.iter().enumerate() {
        if index != 0 {
            description.push_str("; ");
        }
        description.push_str(&node_name(node));
    }
    description.push_str(". Connections: ");
    for (index, edge) in scene.edges.iter().enumerate() {
        if index != 0 {
            description.push_str("; ");
        }
        description.push_str(&node_name_by_id(scene, edge.from));
        description.push_str(" to ");
        description.push_str(&node_name_by_id(scene, edge.to));
        if let Some(label) = &edge.label {
            description.push_str(" via ");
            description.push_str(label);
        }
    }
    description.push('.');
    description
}

fn node_name_by_id(scene: &Scene, id: NodeId) -> String {
    let node = scene
        .nodes
        .iter()
        .find(|node| node.id == id)
        .expect("positioned edges reference positioned nodes");
    node_name(node)
}

fn node_name(node: &Node) -> String {
    match node.kind {
        NodeKind::Start => format!("Start: {}", node.label),
        NodeKind::Action => action::name(node),
        NodeKind::Question => question::name(node),
        NodeKind::Choice => choice::select_name(node),
        NodeKind::Case => choice::case_name(node),
        NodeKind::End => end::name(),
    }
}

fn write_node(svg: &mut String, node: &Node) {
    let class = node_class(node.kind);
    emit!(
        svg,
        "    <g class=\"node {class}\" transform=\"translate({} {})\">",
        node.x,
        node.y
    );
    if matches!(
        node.kind,
        NodeKind::Action | NodeKind::Question | NodeKind::Choice | NodeKind::Case
    ) {
        write_title(svg, &node.label);
    }
    match node.kind {
        NodeKind::Start => write_start(svg, node),
        NodeKind::Action => action::write(svg, node),
        NodeKind::Question => question::write(svg, node),
        NodeKind::Choice => choice::write_select(svg, node),
        NodeKind::Case => choice::write_case(svg, node),
        NodeKind::End => end::write(svg, node),
    }
    svg.push_str("    </g>\n");
}

fn write_start(svg: &mut String, node: &Node) {
    write_boundary_shape(svg, node);
    write_label(svg, node, 0, 0);
}

fn write_boundary_shape(svg: &mut String, node: &Node) {
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

fn write_label(svg: &mut String, node: &Node, center_y: i32, x: i32) {
    let block_height = node.lines.len() as i32 * LINE_HEIGHT;
    let first_y = center_y - block_height / 2 + 15;
    emit_inline!(
        svg,
        "      <text class=\"label\" y=\"{first_y}\" xml:space=\"preserve\">"
    );
    for (index, line) in node.lines.iter().enumerate() {
        let dy = if index == 0 { 0 } else { LINE_HEIGHT };
        emit_inline!(svg, "<tspan x=\"{x}\" dy=\"{dy}\">{}</tspan>", escape(line));
    }
    emit!(svg, "</text>");
}

fn node_class(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Start => "start",
        NodeKind::Action => "action",
        NodeKind::Question => "question",
        NodeKind::Choice => "choice select",
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
