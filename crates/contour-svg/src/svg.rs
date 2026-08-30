use std::fmt::Write;

use crate::layout::{
    CHOICE_SKEW, EDGE_LABEL_FONT, EDGE_LABEL_WIDTH, Edge, Node, NodeId, NodeKind, Scene, wrap_text,
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
    svg.push_str(
        r##"  <defs>
    <marker id="arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto" markerUnits="strokeWidth">
      <path d="M 0 0 L 8 4 L 0 8 z" fill="#334155"/>
    </marker>
    <style>
      .edge { fill: none; stroke: #334155; stroke-width: 2; }
      .edge-label { fill: #334155; font: 12px ui-sans-serif, system-ui, sans-serif; paint-order: stroke; stroke: #f8fafc; stroke-width: 5px; stroke-linejoin: round; text-anchor: middle; }
      .node-shape { stroke: #334155; stroke-width: 2; }
      .start .node-shape, .end .node-shape { fill: #dbeafe; }
      .action .node-shape { fill: #f8fafc; }
      .question .node-shape { fill: #fef3c7; }
      .choice .node-shape { fill: #ede9fe; }
      .merge .node-shape { fill: #cbd5e1; }
      .kind { fill: #64748b; font: 600 11px ui-sans-serif, system-ui, sans-serif; letter-spacing: 1px; text-anchor: middle; }
      .label { fill: #0f172a; font: 14px ui-sans-serif, system-ui, sans-serif; text-anchor: middle; }
      .merge .label { font-weight: 700; }
    </style>
  </defs>
  <rect width="100%" height="100%" fill="#f8fafc"/>
  <g class="edges">
"##,
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
    if let Some(side_x) = edge.side_x {
        emit!(
            svg,
            "    <path class=\"edge\" d=\"M {} {} V {} H {side_x} V {} H {} V {}\" marker-end=\"url(#arrow)\"/>",
            edge.start_x,
            edge.start_y,
            edge.start_y + 30,
            edge.end_y - 30,
            edge.end_x,
            edge.end_y
        );
    } else {
        emit!(
            svg,
            "    <path class=\"edge\" d=\"M {} {} V {} H {} V {}\" marker-end=\"url(#arrow)\"/>",
            edge.start_x,
            edge.start_y,
            edge.middle_y,
            edge.end_x,
            edge.end_y
        );
    }
    let Some(label) = &edge.label else {
        return;
    };
    let lines = wrap_text(label, EDGE_LABEL_WIDTH, EDGE_LABEL_FONT);
    let (x, label_y) = edge.side_x.map_or_else(
        || ((edge.start_x + edge.end_x) / 2, edge.middle_y),
        |side_x| ((edge.start_x + side_x) / 2, edge.start_y + 30),
    );
    let first_y = label_y - (lines.len() as i32 - 1) * 7 - 5;
    emit_inline!(
        svg,
        "    <text class=\"edge-label\" x=\"{x}\" y=\"{first_y}\" xml:space=\"preserve\">"
    );
    for (index, line) in lines.iter().enumerate() {
        let dy = if index == 0 { 0 } else { 14 };
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
        description.push_str(&node_name(scene, node));
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
    node_name(scene, node)
}

fn node_name(scene: &Scene, node: &Node) -> String {
    match node.kind {
        NodeKind::Start => format!("Start: {}", node.label),
        NodeKind::Action => format!("Action: {}", node.label),
        NodeKind::Question => format!("Question: {}", node.label),
        NodeKind::Choice => format!("Choice: {}", node.label),
        NodeKind::Merge => match node.id {
            NodeId::Block(index) => format!("Merge {}", merge_ordinal(scene, index)),
            NodeId::Start | NodeId::End => unreachable!("merge nodes are authored blocks"),
        },
        NodeKind::End => "End".to_owned(),
    }
}

/// Numbers a merge by its position among the flow's merges. A block index would
/// name a count the reader cannot see: the node itself is drawn only as "M".
fn merge_ordinal(scene: &Scene, index: usize) -> usize {
    scene
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Merge)
        .filter(|node| matches!(node.id, NodeId::Block(other) if other <= index))
        .count()
}

fn write_node(svg: &mut String, node: &Node) {
    let class = kind_class(node.kind);
    emit!(
        svg,
        "    <g class=\"node {class}\" transform=\"translate({} {})\">",
        node.x,
        node.y
    );
    if matches!(
        node.kind,
        NodeKind::Action | NodeKind::Question | NodeKind::Choice
    ) {
        emit!(
            svg,
            "      <title xml:space=\"preserve\">{}</title>",
            escape(&node.label)
        );
    }
    write_shape(svg, node);
    match node.kind {
        NodeKind::Merge => {
            svg.push_str("      <text class=\"label\" y=\"5\">M</text>\n");
        }
        NodeKind::End => {
            svg.push_str("      <text class=\"label\" y=\"5\">End</text>\n");
        }
        kind => {
            let caption = match kind {
                NodeKind::Start => "START",
                NodeKind::Action => "ACTION",
                NodeKind::Question => "QUESTION",
                NodeKind::Choice => "CHOICE",
                NodeKind::Merge | NodeKind::End => unreachable!(),
            };
            let block_height = node.lines.len() as i32 * 18;
            let first_y = -block_height / 2 + 15;
            emit!(
                svg,
                "      <text class=\"kind\" y=\"{}\">{caption}</text>",
                first_y - 18
            );
            emit_inline!(
                svg,
                "      <text class=\"label\" y=\"{first_y}\" xml:space=\"preserve\">"
            );
            for (index, line) in node.lines.iter().enumerate() {
                let dy = if index == 0 { 0 } else { 18 };
                emit_inline!(svg, "<tspan x=\"0\" dy=\"{dy}\">{}</tspan>", escape(line));
            }
            emit!(svg, "</text>");
        }
    }
    svg.push_str("    </g>\n");
}

fn write_shape(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    match node.kind {
        NodeKind::Start | NodeKind::End => {
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
        NodeKind::Action => {
            emit!(
                svg,
                "      <rect class=\"node-shape\" x=\"-{}\" y=\"-{}\" width=\"{}\" height=\"{}\" rx=\"8\"/>",
                half_width,
                half_height,
                node.width,
                node.height
            );
        }
        NodeKind::Question => {
            emit!(
                svg,
                "      <polygon class=\"node-shape\" points=\"0,-{half_height} {half_width},0 0,{half_height} -{half_width},0\"/>"
            );
        }
        NodeKind::Choice => {
            let inner = half_width - CHOICE_SKEW;
            emit!(
                svg,
                "      <polygon class=\"node-shape\" points=\"-{inner},-{half_height} {inner},-{half_height} {half_width},0 {inner},{half_height} -{inner},{half_height} -{half_width},0\"/>"
            );
        }
        NodeKind::Merge => {
            emit!(
                svg,
                "      <circle class=\"node-shape\" r=\"{half_width}\"/>"
            );
        }
    }
}

fn kind_class(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Start => "start",
        NodeKind::Action => "action",
        NodeKind::Question => "question",
        NodeKind::Choice => "choice",
        NodeKind::Merge => "merge",
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
