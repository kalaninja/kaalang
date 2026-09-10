use std::fmt::Write;

use crate::layout::{
    CONNECTION_LABEL_FONT, CONNECTION_LABEL_HALO, Connection, LABEL_FONT, LINE_HEIGHT, Label,
    LabelKind, Node, ParameterPanel, Point, Scene,
};
use crate::topology::{Destination, ExitId, NodeId, NodeKind, Source};

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
mod question;

pub(crate) fn serialize(scene: &Scene, flow_name: &str) -> String {
    let mut svg = String::new();
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
      .parameter-link {{ fill: none; stroke: currentColor; stroke-width: 1.75; }}
      .node-shape {{ fill: #ffffff; stroke: currentColor; stroke-width: 1.75; }}
      .start .node-shape, .end .node-shape, .parameter-panel .node-shape {{ fill: #f0f9ff; }}
      .action .node-shape {{ fill: #f8fafc; }}
      .question .node-shape {{ fill: #fffbeb; }}
      .select .node-shape, .case .node-shape {{ fill: #f5f3ff; }}
      .label {{ fill: currentColor; font-size: {LABEL_FONT}px; text-anchor: middle; }}
      .start .label, .question .label, .select .label, .case .label, .end .label {{ font-weight: 600; }}
      .action .label {{ font-weight: 500; text-anchor: start; }}
      .parameter-panel .label {{ font-weight: 400; text-anchor: start; }}
    </style>
  </defs>
  <rect width="100%" height="100%" fill="#ffffff"/>
  <g class="connections">
"##
    );
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
    for node in &scene.nodes {
        write_node(&mut svg, scene, node);
    }
    svg.push_str("  </g>\n</svg>\n");

    svg
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
    emit_inline!(svg, "    <text class=\"{class}\"");
    if matches!(label.kind, LabelKind::Branch) {
        emit_inline!(svg, " style=\"font-size: {}px\"", label.kind.font_size());
    }
    emit_inline!(svg, " x=\"{x}\" y=\"{y}\" xml:space=\"preserve\">");
    write_lines(svg, &label.lines, x, label.kind.line_height());
}

/// Writes the lines of one `<text>` as `<tspan>`s and closes it. The first line
/// sits on the text's own baseline; each later one drops by `line_height`.
fn write_lines(svg: &mut String, lines: &[String], x: i32, line_height: i32) {
    for (index, line) in lines.iter().enumerate() {
        let dy = if index == 0 { 0 } else { line_height };
        emit_inline!(svg, "<tspan x=\"{x}\" dy=\"{dy}\">{}</tspan>", escape(line));
    }
    emit!(svg, "</text>");
}

/// Describes the diagram once: each node with the labels it and its exits own,
/// then each connection as the pair of ends it joins. A junction is not a node,
/// so it appears only as a connection end.
fn describe(scene: &Scene) -> String {
    let topology = &scene.topology;
    let nodes = topology
        .nodes
        .iter()
        .map(|node| {
            let mut described = node_name(scene, node.id);
            if node.id == NodeId::Start
                && let Some(parameters) = &scene.parameters
            {
                described.push_str(" with parameters ");
                described.push_str(&parameters.parameters.join("; "));
            }
            let capture = topology.capture_label(node.id);
            if capture == ["()"] {
                described.push_str(" capturing nothing");
            } else if !capture.is_empty() {
                described.push_str(" capturing ");
                described.push_str(&capture.join(", "));
            }
            let handovers = topology
                .exits
                .iter()
                .filter(|exit| exit.id.node == node.id && !exit.handover.is_empty())
                .map(|exit| exit.handover.join(", "))
                .collect::<Vec<_>>();
            if !handovers.is_empty() {
                described.push_str(" handing over ");
                described.push_str(&handovers.join(" / "));
            }
            let branch_descriptions = topology
                .exits
                .iter()
                .filter(|exit| exit.id.node == node.id)
                .filter_map(|exit| {
                    exit.branch_description.as_ref().map(|description| {
                        format!(
                            "branch {}: {description}",
                            exit.id
                                .branch
                                .expect("only question branches have descriptions")
                                + 1
                        )
                    })
                })
                .collect::<Vec<_>>();
            if !branch_descriptions.is_empty() {
                described.push_str(" described as ");
                described.push_str(&branch_descriptions.join(" / "));
            }
            described
        })
        .collect::<Vec<_>>()
        .join("; ");
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

    format!("Nodes: {nodes}. Connections: {connections}.")
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
        return format!(
            "the {} of {}",
            if loop_.tail == junction {
                "iteration tail"
            } else {
                "entry"
            },
            node_name(scene, NodeId::Block(loop_.header))
        );
    }
    format!(
        "the {} merge",
        scene.topology.junctions[junction].wires.join(" and ")
    )
}

fn node_name(scene: &Scene, id: NodeId) -> String {
    let node = scene.topology.node(id);
    match node.kind {
        NodeKind::Start => format!("Start: {}", node.label),
        NodeKind::Action => action::name(&node.label),
        NodeKind::Question => question::name(&node.label),
        NodeKind::Select => choice::select_name(&node.label),
        NodeKind::Case => choice::case_name(&node.label),
        // The drawn caption carries the arrow; the role does not.
        NodeKind::End => format!(
            "End: {}",
            node.label
                .strip_prefix("->")
                .unwrap_or(&node.label)
                .trim_start()
        ),
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
    if matches!(
        projected.kind,
        NodeKind::Action | NodeKind::Question | NodeKind::Select | NodeKind::Case
    ) {
        write_title(svg, &projected.label);
    }
    match projected.kind {
        // Start and end are the two ends of one flow, drawn alike.
        NodeKind::Start | NodeKind::End => {
            write_capsule(svg, node);
            write_label(svg, node, 0, 0);
        }
        NodeKind::Action => action::write(svg, node),
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

fn write_label(svg: &mut String, node: &Node, center_y: i32, x: i32) {
    let block_height = node.lines.len() as i32 * LINE_HEIGHT;
    let first_y = center_y - block_height / 2 + 15;
    emit_inline!(
        svg,
        "      <text class=\"label\" y=\"{first_y}\" xml:space=\"preserve\">"
    );
    write_lines(svg, &node.lines, x, LINE_HEIGHT);
}

const fn node_class(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Start => "start",
        NodeKind::Action => "action",
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
