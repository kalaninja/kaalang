use std::fmt::Write;

use kaalang_compiler::topology::{Destination, ExitId, NodeId, NodeKind, Source};

pub(crate) use self::staged::serialize_staged;
use crate::{
    flow::layout::{
        CONNECTION_LABEL_FONT, CONNECTION_LABEL_HALO, CYCLE_CAPTION_FONT, Connection, Label,
        LabelKind, MERGE_RADIUS, NODE_LABEL_PADDING_X, NODE_LABEL_PADDING_Y, Node, ParameterPanel,
        Point, Scene,
    },
    markup::{
        BADGE_STYLE, BASE_STYLE, FONT_WEIGHT_BOLD, FONT_WEIGHT_MEDIUM, FONT_WEIGHT_REGULAR,
        FONT_WEIGHT_SEMIBOLD, MONOSPACE, SECONDARY_COLOR, STROKE_WIDTH, TextAnchor, arrow_marker,
        escape, markdown_styles, needs_composed_lines, write_composed_lines, write_lines,
        write_svg_open, write_title,
    },
    text::{LABEL_FONT, LINE_HEIGHT, RichText, block_metrics},
};

mod action;
mod call;
mod choice;
mod cycle;
mod question;
mod stage;
mod staged;

const LOOP_MARKER_FONT: i32 = 20;
const CYCLE_DASH_LENGTH: i32 = 7;
const CYCLE_DASH_GAP: i32 = 5;
const CYCLE_CAPTION_HALO: i32 = 4;

#[allow(clippy::too_many_lines)] // The node and route groups write one complete local SVG.
pub(crate) fn serialize(scene: &Scene, flow_name: &str, part: Option<usize>) -> String {
    let mut svg = String::new();
    let part_id = part.map_or_else(String::new, |part| format!("part-{part}-"));
    let back_edge_arrow_id = part.map_or_else(
        || "back-edge-arrow".to_owned(),
        |_| format!("kaalang-{part_id}back-edge-arrow"),
    );
    let markdown_styles = markdown_styles(
        scene
            .nodes
            .iter()
            .flat_map(|node| markup(&node.lines))
            .chain(scene.labels.iter().flat_map(|label| markup(&label.lines)))
            .chain(
                scene
                    .cycle_regions
                    .iter()
                    .flat_map(|region| &region.caption),
            ),
    );
    // Before the Markdown colors, so that an authored color still wins.
    let cycle_math = if markdown_styles.is_empty() {
        ""
    } else {
        "      .cycle-caption .md-math { color: #166534; }\n"
    };
    let background = if part.is_none() {
        "  <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n"
    } else {
        ""
    };
    let has = |kind: NodeKind| scene.topology.nodes.iter().any(|node| node.kind == kind);
    let stage_styles = if has(NodeKind::StageEntry) || has(NodeKind::StageTransition) {
        format!(
            "      .stage-entry .node-shape, .stage-transition .node-shape {{ fill: #f5f3ff; }}\n      .stage-entry .label, .stage-transition .label {{ font-weight: {FONT_WEIGHT_SEMIBOLD}; }}\n      .stage-marker {{ fill: currentColor; }}\n"
        )
    } else {
        String::new()
    };
    let cap_styles = if has(NodeKind::ForEntry) {
        format!(
            "      .for-entry .node-shape, .for-end .node-shape {{ fill: #f0fdf4; }}\n      .for-entry .label, .for-end .label {{ font-weight: {FONT_WEIGHT_MEDIUM}; text-anchor: start; }}\n"
        )
    } else {
        String::new()
    };
    write_svg_open(
        &mut svg,
        (scene.width, scene.height),
        &format!("kaalang-{part_id}"),
        &format!("kaalang diagram for {flow_name}"),
        &describe(scene),
    );
    emit_inline!(
        svg,
        r#"  <defs>
    <style>
      {BASE_STYLE}
      .connection {{ fill: none; stroke: currentColor; stroke-width: {STROKE_WIDTH}; stroke-linecap: square; stroke-linejoin: round; }}
      .connection-label {{ fill: {SECONDARY_COLOR}; font-size: {CONNECTION_LABEL_FONT}px; font-weight: {FONT_WEIGHT_REGULAR}; paint-order: stroke; stroke: #ffffff; stroke-width: {CONNECTION_LABEL_HALO}px; stroke-linejoin: round; text-anchor: start; }}
      .branch-label {{ fill: currentColor; font-weight: {FONT_WEIGHT_MEDIUM}; }}
      .parameter-link {{ fill: none; stroke: currentColor; stroke-width: {STROKE_WIDTH}; }}
      .node-shape {{ fill: #ffffff; stroke: currentColor; stroke-width: {STROKE_WIDTH}; }}
      .start .node-shape, .end .node-shape, .parameter-panel .node-shape {{ fill: #f0f9ff; }}
      .action .node-shape, .call .node-shape {{ fill: #f8fafc; }}
      .call-bars {{ fill: none; stroke: currentColor; stroke-width: {STROKE_WIDTH}; }}
      .cycle .node-shape {{ fill: #f0fdf4; }}
      .loop-marker {{ fill: #15803d; font-size: {LOOP_MARKER_FONT}px; font-weight: {FONT_WEIGHT_BOLD}; text-anchor: middle; }}
      .question .node-shape {{ fill: #fffbeb; }}
      .select .node-shape, .case .node-shape {{ fill: #f5f3ff; }}
      .label {{ fill: currentColor; font-size: {LABEL_FONT}px; text-anchor: middle; }}
      .start .label, .question .label, .select .label, .case .label, .end .label {{ font-weight: {FONT_WEIGHT_SEMIBOLD}; }}
      .action .label, .call .label, .cycle .label {{ font-weight: {FONT_WEIGHT_MEDIUM}; text-anchor: start; }}
      .parameter-panel .label {{ font-weight: {FONT_WEIGHT_REGULAR}; text-anchor: start; }}
      .code {{ font-family: {MONOSPACE}; }}
      .cycle-boundary {{ fill: #f0fdf433; stroke: #15803d; stroke-width: {STROKE_WIDTH}; stroke-dasharray: {CYCLE_DASH_LENGTH} {CYCLE_DASH_GAP}; }}
      .cycle-caption {{ fill: #166534; font-size: {CYCLE_CAPTION_FONT}px; font-weight: {FONT_WEIGHT_SEMIBOLD}; paint-order: stroke; stroke: #ffffff; stroke-width: {CYCLE_CAPTION_HALO}px; }}
{BADGE_STYLE}{cycle_math}{markdown_styles}{stage_styles}{cap_styles}    </style>
  </defs>
{background}  <g class="cycle-regions">
"#
    );
    for region in scene.cycle_regions.iter().filter(|region| region.drawn) {
        cycle::write(&mut svg, region);
    }
    svg.push_str("  </g>\n  <g class=\"connections\">\n");
    if !scene.topology.back_edges.is_empty() {
        emit!(
            svg,
            "    <defs>{}</defs>",
            arrow_marker(&back_edge_arrow_id)
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
    let source = source(&label.lines);
    let lines = source.as_deref().unwrap_or(&label.lines);
    let class = match (label.kind, source.is_some()) {
        (LabelKind::Wire, false) => "connection-label",
        (LabelKind::Wire, true) => "connection-label code",
        (LabelKind::Branch, false) => "connection-label branch-label",
        (LabelKind::Branch, true) => "connection-label branch-label code",
    };
    if needs_composed_lines(lines) {
        let style = matches!(label.kind, LabelKind::Branch)
            .then(|| format!("font-size: {}px", label.kind.font_size()));
        write_composed_lines(
            svg,
            "    ",
            class,
            style.as_deref(),
            lines,
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
        lines,
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
    write_title(svg, &merge_name(scene, junction));
    emit!(
        svg,
        "      <circle class=\"node-shape\" r=\"{MERGE_RADIUS}\" style=\"fill: currentColor\"/>"
    );
    svg.push_str("    </g>\n");
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
    let x = NODE_LABEL_PADDING_X - parameters.width / 2;
    let source = source(&parameters.lines);
    let (class, lines) = match &source {
        Some(lines) => ("label code", lines.as_slice()),
        None => ("label", parameters.lines.as_slice()),
    };
    emit_inline!(
        svg,
        "      <text class=\"{class}\" x=\"{x}\" y=\"{first_y}\" xml:space=\"preserve\">"
    );
    write_lines(svg, lines, x, LABEL_FONT, LINE_HEIGHT);
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
    let label = scene.captions.label(id).unmarked();
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
        NodeKind::StageTransition => format!("Stage transition: {label}"),
        NodeKind::ForEntry => format!("For entry: {label}"),
        NodeKind::ForEnd => format!("For end: {label}"),
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
        NodeKind::StageTransition => stage::write_transition(svg, node),
        NodeKind::ForEntry => cycle::write_cap(svg, node, true),
        NodeKind::ForEnd => cycle::write_cap(svg, node, false),
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
        "      <rect class=\"node-shape\" x=\"-{half_width}\" y=\"-{half_height}\" width=\"{}\" height=\"{}\" rx=\"{half_height}\"/>",
        node.width,
        node.height
    );
}

/// Draws a node's caption from its left padding, as rectangular nodes do.
fn write_start_label(svg: &mut String, node: &Node) {
    let x = NODE_LABEL_PADDING_X - node.width / 2;
    write_label(svg, node, 0, x, TextAnchor::Start);
}

/// Draws one node's caption. The anchor matches the `text-anchor` the node's
/// class carries in the stylesheet; the composed path positions runs itself and
/// cannot read it from the CSS.
fn write_label(svg: &mut String, node: &Node, center_y: i32, x: i32, anchor: TextAnchor) {
    let metrics = block_metrics(&node.lines, LABEL_FONT, LINE_HEIGHT);
    let first_y = center_y - metrics.height / 2 + metrics.baselines[0];
    let source = source(&node.lines);
    let (class, lines) = match &source {
        Some(lines) => ("label code", lines.as_slice()),
        None => ("label", node.lines.as_slice()),
    };
    if needs_composed_lines(lines) {
        write_composed_lines(
            svg,
            "      ",
            class,
            None,
            lines,
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
        "      <text class=\"{class}\" y=\"{first_y}\" xml:space=\"preserve\">"
    );
    write_lines(svg, lines, x, LABEL_FONT, LINE_HEIGHT);
}

/// Rust source as plain lines, for a `code` element whose rule sets it in
/// code, or `None` for text that keeps its Markdown spans.
fn source(lines: &[RichText]) -> Option<Vec<RichText>> {
    lines.iter().all(RichText::is_code).then(|| {
        // A badge composes the block, which places each run by the width it
        // was measured at, so the lines keep the style their rule renders.
        if lines
            .iter()
            .flat_map(RichText::spans)
            .any(|span| span.style.badge)
        {
            lines.to_vec()
        } else {
            lines
                .iter()
                .map(|line| RichText::literal(line.as_ref()))
                .collect()
        }
    })
}

/// The lines whose Markdown spans are written: all but Rust source.
fn markup(lines: &[RichText]) -> &[RichText] {
    if lines.iter().all(RichText::is_code) {
        &[]
    } else {
        lines
    }
}

const fn node_class(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Start => "start",
        NodeKind::StageEntry => "stage-entry",
        NodeKind::Action => "action",
        NodeKind::Call => "call",
        NodeKind::Cycle => "cycle",
        NodeKind::Question => "question",
        NodeKind::Select => "select",
        NodeKind::Case => "case",
        NodeKind::End => "end",
        NodeKind::StageTransition => "stage-transition",
        NodeKind::ForEntry => "for-entry",
        NodeKind::ForEnd => "for-end",
    }
}
