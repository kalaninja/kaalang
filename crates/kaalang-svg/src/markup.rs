//! The SVG markup every diagram shares: text and formulas, the common style
//! values, and the arrowhead.

use std::fmt::Write;

use latex_rust::Dim;
use unicode_segmentation::UnicodeSegmentation;

use crate::text::{
    Color, FORMULA_STRIKETHROUGH_OFFSET, FORMULA_UNDERLINE_OFFSET, FORMULA_UNDERLINE_PADDING,
    Formula, RichText, SCRIPT_FONT_SCALE, SUBSCRIPT_SHIFT, SUPERSCRIPT_SHIFT, Script, Style,
    TEXT_ADVANCE_SCALE, block_metrics, line_ink, shaping_spans, svg_dimension,
};

/// The margin around a flow or type diagram; a staged flow's composition keeps
/// its own.
pub(crate) const MARGIN: i32 = 32;
/// The dot that marks a merge, or where a relationship leaves its tile.
pub(crate) const DOT_RADIUS: i32 = 4;
/// Every outline and line: nodes, cards, cycle boundaries, and connections. A
/// type diagram draws its focus card heavier.
pub(crate) const STROKE_WIDTH: f64 = 1.75;
pub(crate) const FONT_WEIGHT_REGULAR: i32 = 400;
pub(crate) const FONT_WEIGHT_MEDIUM: i32 = 500;
pub(crate) const FONT_WEIGHT_SEMIBOLD: i32 = 600;
pub(crate) const FONT_WEIGHT_BOLD: i32 = 700;
/// Markdown bold, visibly stronger than any label's own weight, as RFC 0005 asks.
const FONT_WEIGHT_HEAVY: i32 = 800;
/// The text color and font stack of every diagram.
pub(crate) const BASE_STYLE: &str = r#"svg { color: #1f2937; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; text-rendering: optimizeLegibility; }"#;
pub(crate) const MONOSPACE: &str =
    r#"ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace"#;
/// Secondary text: wire labels, a card's kind, field names.
pub(crate) const SECONDARY_COLOR: &str = "#64748b";
const FORMULA_BOLD_STROKE: f64 = 0.7;
const FORMULA_DECORATION_STROKE: i32 = 1;
const FORMULA_CLIP_PADDING: i32 = 1;
// The skew angle matching text::FORMULA_SLANT, rounded for SVG output.
const FORMULA_SKEW_DEGREES: f64 = -8.5308;
pub(crate) const ARROW_SIZE: i32 = 10;
pub(crate) const ARROW_MIDPOINT: i32 = ARROW_SIZE / 2;
const BACK_EDGE_ARROW_TIP_INSET: i32 = 1;
const BACK_EDGE_ARROW_REF_X: i32 = ARROW_SIZE - BACK_EDGE_ARROW_TIP_INSET;

/// The Markdown rules, when any of these lines needs one.
pub(crate) fn markdown_styles<'a>(mut lines: impl Iterator<Item = &'a RichText>) -> String {
    if lines.any(|line| {
        line.spans()
            .iter()
            .any(|span| span.style != Style::default() || span.formula.is_some())
    }) {
        format!(
            "      .md-code {{ font-family: {MONOSPACE}; font-weight: {FONT_WEIGHT_SEMIBOLD}; }}
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
      .md-color-red, .md-math.md-color-red {{ fill: #b91c1c; color: #b91c1c; }}
      .md-color-green, .md-math.md-color-green {{ fill: #166534; color: #166534; }}
      .md-color-blue, .md-math.md-color-blue {{ fill: #1d4ed8; color: #1d4ed8; }}
      .md-color-purple, .md-math.md-color-purple {{ fill: #6d28d9; color: #6d28d9; }}
      .md-color-muted, .md-math.md-color-muted {{ fill: #475569; color: #475569; }}
",
            script_font_percent = SCRIPT_FONT_SCALE.0 * 100 / SCRIPT_FONT_SCALE.1,
            superscript_shift = f64::from(SUPERSCRIPT_SHIFT.0) / f64::from(SUPERSCRIPT_SHIFT.1),
            subscript_shift = f64::from(SUBSCRIPT_SHIFT.0) / f64::from(SUBSCRIPT_SHIFT.1),
        )
    } else {
        String::new()
    }
}

/// The arrowhead every diagram draws at a line's end, its tip just inside it.
pub(crate) fn arrow_marker(id: &str) -> String {
    format!(
        "<marker id=\"{id}\" markerWidth=\"{ARROW_SIZE}\" markerHeight=\"{ARROW_SIZE}\" refX=\"{BACK_EDGE_ARROW_REF_X}\" refY=\"{ARROW_MIDPOINT}\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M 0 0 L {ARROW_SIZE} {ARROW_MIDPOINT} L 0 {ARROW_SIZE} Z\" fill=\"currentColor\"/></marker>"
    )
}

/// Writes the lines of one `<text>` as `<tspan>`s and closes it. The first line
/// sits on the text's own baseline; each later one drops by `line_height`.
pub(crate) fn write_lines(
    svg: &mut String,
    lines: &[RichText],
    x: i32,
    font_size: i32,
    line_height: i32,
) {
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
pub(crate) enum TextAnchor {
    Start,
    Middle,
    End,
}

pub(crate) fn needs_composed_lines(lines: &[RichText]) -> bool {
    lines
        .iter()
        .flat_map(RichText::spans)
        .any(|span| span.formula.is_some() || span.style.highlight)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn write_composed_lines(
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
    let bottom_padding = top_padding
        + i32::from(formula.is_clipped() && style.underline) * FORMULA_UNDERLINE_PADDING;
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

pub(crate) fn write_title(svg: &mut String, label: &str) {
    emit!(
        svg,
        "      <title xml:space=\"preserve\">{}</title>",
        escape(label)
    );
}

pub(crate) fn escape(text: &str) -> String {
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
