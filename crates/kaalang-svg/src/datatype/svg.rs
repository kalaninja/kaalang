use std::fmt::Write;

use super::layout::{CardShape, NAME_FONT, Route, Text, TextBlock, TypeScene};
use crate::{
    markup::{
        BADGE_STYLE, BASE_STYLE, DOT_RADIUS, FONT_WEIGHT_BOLD, FONT_WEIGHT_REGULAR,
        FONT_WEIGHT_SEMIBOLD, MONOSPACE, SECONDARY_COLOR, STROKE_WIDTH, TextAnchor, arrow_marker,
        escape, markdown_styles, needs_composed_lines, write_composed_lines, write_lines,
        write_title,
    },
    text::{LABEL_FONT, SECONDARY_FONT, block_metrics},
};

/// The side of the square where a bus leaves its card.
const PORT: i32 = 8;
/// The focus is the one outline drawn heavier, as the diagram's subject.
const FOCUS_STROKE_WIDTH: f64 = 2.5;

pub(super) fn serialize(scene: &TypeScene) -> String {
    let mut svg = String::new();
    let focus = scene.cards.last().expect("a type diagram has its focus");
    let markdown_styles = markdown_styles(
        scene
            .cards
            .iter()
            .flat_map(|card| &card.texts)
            .filter(|block| block.text == Text::Description)
            .flat_map(|block| &block.lines),
    );
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
        "  <title id=\"kaalang-title\">kaalang type diagram for {}</title>",
        escape(&focus.name)
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
      {BASE_STYLE}
      .card-shape {{ fill: #ffffff; }}
      .card-groove {{ fill: #f8fafc; }}
      .card-outline {{ fill: none; stroke: currentColor; stroke-width: {STROKE_WIDTH}; }}
      .focus .card-outline {{ stroke-width: {FOCUS_STROKE_WIDTH}; }}
      .card-rules {{ fill: none; stroke: #cbd5e1; stroke-width: 1; }}
      .type-name, .description, .field-type, .variant-name {{ fill: currentColor; }}
      .type-kind, .field-name, .target {{ fill: {SECONDARY_COLOR}; font-size: {SECONDARY_FONT}px; }}
      .type-name {{ font-size: {NAME_FONT}px; font-weight: {FONT_WEIGHT_BOLD}; }}
      .description, .variant-name {{ font-size: {LABEL_FONT}px; }}
      .field-type {{ font-size: {SECONDARY_FONT}px; }}
      .field-name, .field-type, .variant-name {{ font-family: {MONOSPACE}; font-weight: {FONT_WEIGHT_SEMIBOLD}; }}
      .target .md-code {{ font-family: {MONOSPACE}; font-weight: {FONT_WEIGHT_REGULAR}; }}
      .relationship {{ fill: none; stroke: currentColor; stroke-width: {STROKE_WIDTH}; stroke-linejoin: round; }}
      .relationship-source {{ fill: currentColor; }}
      .port {{ fill: #ffffff; stroke: currentColor; stroke-width: {STROKE_WIDTH}; }}
{BADGE_STYLE}{markdown_styles}    </style>
    {marker}
  </defs>
  <rect width="100%" height="100%" fill="#ffffff"/>
  <g class="cards">
"##,
        marker = arrow_marker("relationship-arrow")
    );
    for card in &scene.cards {
        write_card(&mut svg, card);
    }
    svg.push_str("  </g>\n  <g class=\"relationships\">\n");
    for route in &scene.routes {
        write_route(&mut svg, scene, route);
    }
    // Over the routes, so that each bus passes through its port.
    for port in &scene.ports {
        emit!(
            svg,
            "    <rect class=\"port\" x=\"{}\" y=\"{}\" width=\"{PORT}\" height=\"{PORT}\"/>",
            port.x - PORT / 2,
            port.y - PORT / 2
        );
    }
    svg.push_str("  </g>\n</svg>\n");
    svg
}

fn write_card(svg: &mut String, card: &CardShape) {
    let class = if card.focus { "card focus" } else { "card" };
    emit!(svg, "    <g class=\"{class}\">");
    write_title(svg, &card.name);
    let bounds = format!(
        "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
        card.x, card.y, card.width, card.height
    );
    emit!(svg, "      <rect class=\"card-shape\" {bounds}/>");
    for &(left, top, right, bottom) in &card.grooves {
        emit!(
            svg,
            "      <rect class=\"card-groove\" x=\"{left}\" y=\"{top}\" width=\"{}\" height=\"{}\"/>",
            right - left,
            bottom - top
        );
    }
    if !card.rules.is_empty() {
        let rules: Vec<String> = card
            .rules
            .iter()
            .map(|[from, to]| format!("M {} {} L {} {}", from.x, from.y, to.x, to.y))
            .collect();
        emit!(
            svg,
            "      <path class=\"card-rules\" d=\"{}\"/>",
            rules.join(" ")
        );
    }
    // Over the rules, so that none lightens the outline where it meets it.
    emit!(svg, "      <rect class=\"card-outline\" {bounds}/>");
    for block in &card.texts {
        write_text(svg, block);
    }
    svg.push_str("    </g>\n");
}

fn write_text(svg: &mut String, block: &TextBlock) {
    let (font, line_height) = (block.text.font(), block.text.line_height());
    let first_y = block.top + block_metrics(&block.lines, font, line_height).baselines[0];
    let class = match block.text {
        Text::Name => "type-name",
        Text::Kind => "type-kind",
        Text::Description => "description",
        Text::Field => "field-name",
        Text::Type => "field-type",
        Text::Variant => "variant-name",
        Text::Target => "target",
    };
    let end = block.text.right_aligned();
    if needs_composed_lines(&block.lines) {
        write_composed_lines(
            svg,
            "      ",
            class,
            None,
            &block.lines,
            block.x,
            first_y,
            font,
            line_height,
            if end {
                TextAnchor::End
            } else {
                TextAnchor::Start
            },
        );
        return;
    }
    let anchor = if end { " text-anchor=\"end\"" } else { "" };
    emit_inline!(
        svg,
        "      <text class=\"{class}\"{anchor} y=\"{first_y}\" xml:space=\"preserve\">"
    );
    write_lines(svg, &block.lines, block.x, font, line_height);
}

fn write_route(svg: &mut String, scene: &TypeScene, route: &Route) {
    let source = &scene.cards[route.card];
    let focus = scene.cards.last().expect("a type diagram has its focus");
    svg.push_str("    <g class=\"relationship-route\">\n");
    write_title(svg, &format!("{} → {}", source.name, focus.name));
    let path: Vec<String> = route
        .points
        .iter()
        .map(|point| format!("{} {}", point.x, point.y))
        .collect();
    emit!(
        svg,
        "      <path class=\"relationship\" d=\"M {}\" marker-end=\"url(#relationship-arrow)\"/>",
        path.join(" L ")
    );
    let start = route.points[0];
    emit!(
        svg,
        "      <circle class=\"relationship-source\" cx=\"{}\" cy=\"{}\" r=\"{DOT_RADIUS}\"/>",
        start.x,
        start.y
    );
    svg.push_str("    </g>\n");
}

/// Names the focus, then each incoming type with the number of its lines.
fn describe(scene: &TypeScene) -> String {
    let focus = scene.cards.len() - 1;
    let lines = |card: usize| {
        scene
            .routes
            .iter()
            .filter(|route| route.card == card)
            .count()
    };
    let mut described = format!("Type {}", scene.cards[focus].name);
    let incoming: Vec<String> = scene.cards[..focus]
        .iter()
        .enumerate()
        .map(|(index, card)| match lines(index) {
            0 | 1 => card.name.clone(),
            count => format!("{} ({count} relationships)", card.name),
        })
        .collect();
    if !incoming.is_empty() {
        described.push_str(" with incoming types ");
        described.push_str(&incoming.join(", "));
    }
    match lines(focus) {
        0 => {}
        1 => described.push_str("; it refers to itself once"),
        count => write!(described, "; it refers to itself {count} times")
            .expect("writing to a String cannot fail"),
    }
    described.push('.');
    described
}
