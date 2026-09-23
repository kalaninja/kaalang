use kaalang_svg::{RenderError, RenderOptions, render_source, render_source_with_options};

/// One flow carrying every label case at once, with a distinct wire name for
/// each so a duplicate in the description is detectable by counting.
const SOURCE: &str = r#"
    #[kaalang]
    fn describe(seed: u8, _spare: u8) -> u8 {
        #[choice("Pick a lane.")]
        #[case("The near lane.")]
        #[case("The far lane.")]
        let (near, far) = |seed| {
            match seed {
                0 => (),
                _ => (),
            }
        };

        #[action("Split <the> near lane & measure it.")]
        let (width, depth) = |near| { (1, 2) };

        #[action("Measure the width.")]
        let measured = |&width| { *width };

        #[action("Gauge the depth.")]
        let gauged = |depth| { depth };

        #[action("Finish from the near lane.")]
        let end = |measured, gauged| { measured + gauged };

        #[action("Finish from the far lane.")]
        let end = |far| { 9 };

        |end| return end;
    }
"#;

const CYCLE_SOURCE: &str = r#"
    #[kaalang]
    fn count_to(count: usize, limit: usize) -> usize {
        #[cycle("Count to the limit.")]
        let total = |mut count, limit| {
            #[question("Has the counter reached the limit?")]
            let (done, again) = |&count, &limit| *count == *limit;

            |done, count| break count;

            #[action("Increment the counter.")]
            |again, &mut count| *count += 1;
        };

        |total| return total;
    }
"#;

#[test]
fn renders_an_accessible_standalone_svg() {
    let svg = render_source(SOURCE, "describe").expect("the flow has a conforming diagram");

    assert!(svg.contains(
        r#"role="img" aria-labelledby="kaalang-title" aria-describedby="kaalang-description""#
    ));
    assert!(svg.contains(r#"<title id="kaalang-title">kaalang diagram for describe</title>"#));
    assert!(svg.contains(r#"<desc id="kaalang-description">"#));
    // Authored text reaches both the node title and the description escaped.
    assert!(svg.contains(
        r#"<title xml:space="preserve">Split &lt;the&gt; near lane &amp; measure it.</title>"#
    ));
    assert_eq!(svg.matches(r#"class="node end""#).count(), 1);
    assert_eq!(svg.matches(r#"class="node merge""#).count(), 1);
    assert!(svg.contains(r#"<circle class="node-shape" r="4""#));
    assert_eq!(svg.matches(r#"class="parameter-panel""#).count(), 1);
    // Distributors and merge rails share one paint operation, so overlapping
    // branches cannot darken their antialiased edges by repeated strokes.
    assert_eq!(svg.matches(r#"<path class="connection""#).count(), 1);
    assert!(svg.contains(">seed: u8</tspan><tspan"));
    assert!(svg.contains(">_spare: u8</tspan></text>"));
    assert!(svg.contains(
        ".start .node-shape, .end .node-shape, .parameter-panel .node-shape { fill: #f0f9ff; }"
    ));
    assert!(svg.contains(".action .node-shape, .call .node-shape { fill: #f8fafc; }"));
    assert!(svg.contains(".question .node-shape { fill: #fffbeb; }"));
    assert!(svg.contains(".select .node-shape, .case .node-shape { fill: #f5f3ff; }"));
    assert!(svg.contains(".branch-label { fill: currentColor; font-weight: 500; }"));
}

#[test]
fn renders_markdown_as_styled_svg_text_and_math_paths() {
    let source = r#"
        #[kaalang]
        fn styled(value: u8) -> u8 {
            #[action("**Bold** *italic* ~~old~~ `code` x^2^ H~2~O")]
            let decorated = |value| value;

            #[action("> Quote: $\\frac{1}{2}$.")]
            let result = |decorated| decorated;
            |result| return result;
        }
    "#;
    let svg = render_source(source, "styled").expect("the styled description renders");

    for (class, text) in [
        ("md-bold", "Bold"),
        ("md-italic", "italic"),
        ("md-strikethrough", "old"),
        ("md-code", "code"),
        ("md-superscript", "2"),
        ("md-subscript", "2"),
    ] {
        assert!(svg.contains(&format!(r#"<tspan class="{class}">{text}</tspan>"#)));
    }
    assert!(svg.contains(
        ".md-code { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\", monospace; font-weight: 600; }"
    ));
    assert!(svg.contains(".md-quote { font-style: italic; }"));
    assert!(svg.contains("“Quote:"));
    assert!(svg.contains(r#"<svg class="md-math""#));
    assert!(svg.contains("<path d="));
    assert!(
        svg.contains(r#"<title xml:space="preserve">Bold italic old code x^(2) H_(2)O</title>"#)
    );
    assert!(describe(&svg).contains("Action: Bold italic old code x^(2) H_(2)O"));
}

#[test]
fn renders_tex_colors_without_losing_inherited_or_explicit_colors() {
    let source = r##"
        #[kaalang]
        fn colors() {
            #[action(r#"<color name="purple">$\frac{1}{2}$ **$\textcolor{black}{x}$**</color> $\definecolor{accent}{HTML}{010203}\textcolor{accent}{x}$ $\fcolorbox{blue}{yellow}{x}$"#)]
            {};
            return;
        }
    "##;
    let svg = render_source(source, "colors").expect("TeX colors render in descriptions");

    assert!(svg.contains(r#"<svg class="md-math md-color-purple""#));
    assert!(svg.contains(r#"<svg class="md-math md-color-purple md-bold""#));
    assert!(svg.contains(".md-math rect:not([stroke]) { stroke: none;"));
    assert!(svg.contains(".md-math.md-bold g[stroke] path { stroke: inherit; }"));
    assert!(svg.contains(r#"<g fill="inherit">"#));
    assert!(svg.contains(r##"<g fill="#000000" stroke="#000000">"##));
    assert!(svg.contains(r##"<g fill="#010203" stroke="#010203">"##));
    assert!(svg.contains(r##"fill="#ffff00""##));
    assert!(svg.contains(r##"stroke="#0000ff""##));
    assert!(describe(&svg).contains(r"\definecolor{accent}{HTML}{010203}\textcolor{accent}{x}"));
}

#[test]
fn clips_a_wide_formula_without_losing_its_accessible_text() {
    let body = format!("{}x", "x+".repeat(70));
    let source = format!("#[kaalang] fn wide() {{ #[action(\"<u>${body}$</u>\")] {{}}; return; }}");
    let svg = render_source(&source, "wide").expect("the formula fits by clipping");

    assert!(svg.contains("<svg class=\"md-math\" style=\"overflow: hidden\""));
    assert!(svg.contains(&format!("<title xml:space=\"preserve\">{body}</title>")));
    assert!(describe(&svg).contains(&body));
    let node = svg
        .lines()
        .filter(|line| line.contains(r#"<rect class="node-shape""#) && line.contains("width="))
        .nth(1)
        .unwrap();
    assert!(node.contains("width=\"280\""));
    let math = svg
        .lines()
        .find(|line| line.contains(r#"<svg class="md-math""#))
        .unwrap();
    let ellipsis = svg
        .lines()
        .find(|line| line.contains("md-math-ellipsis"))
        .unwrap();
    assert!(ellipsis.contains(">…</text>"));
    assert!(
        (attribute(math, "x") + attribute(math, "width") - attribute(ellipsis, "x")).abs() < 0.001
    );
    assert!((attribute(math, "width") + attribute(ellipsis, "textLength") - 248.0).abs() < 0.001);
    assert!((formula_scale(math) - 14.0).abs() < 0.01);
    let view_box = view_box(math);
    let underline = svg
        .lines()
        .find(|line| line.contains("<line x1=\"0\""))
        .unwrap();
    assert!(attribute(underline, "y1") + 0.5 / formula_scale(math) < view_box[1] + view_box[3]);
}

#[test]
fn renders_paired_inline_styles_as_svg_without_html() {
    let source = r##"
        #[kaalang]
        fn styled(value: u8) -> u8 {
            #[action(r#"<mark>**Bright**</mark> <u>underlined</u> <color name="red">red</color>"#)]
            let decorated = |value| value;
            #[action(r#"<color name="purple"><u>$x^2$</u></color>"#)]
            let result = |decorated| decorated;
            |result| return result;
        }
    "##;
    let svg = render_source(source, "styled").expect("the styled description renders");
    assert!(svg.contains(r#"<rect class="md-highlight-box""#));
    assert!(svg.contains(r#"<tspan class="md-bold">Bright</tspan>"#));
    assert!(svg.contains(r#"<tspan class="md-underline">underlined</tspan>"#));
    assert!(svg.contains(r#"<tspan class="md-color-red">red</tspan>"#));
    assert!(svg.contains(r#"<svg class="md-math md-color-purple""#));
    assert!(svg.contains(r#"<line x1="0""#));
    assert!(svg.contains(r#"<title xml:space="preserve">Bright underlined red</title>"#));
    assert!(!svg.contains("<mark>"));
    assert!(!svg.contains("<color name="));
}

#[test]
fn highlight_offsets_follow_a_grapheme_split_across_styles() {
    let source = r##"
        #[kaalang]
        fn highlighted(value: u8) -> u8 {
            #[action(r#"<mark>e</mark>́<mark>x</mark>"#)]
            let result = |value| value;
            |result| return result;
        }
    "##;
    let svg = render_source(source, "highlighted").unwrap();
    let boxes = svg
        .lines()
        .filter(|line| line.contains("<rect class=\"md-highlight-box\""))
        .collect::<Vec<_>>();
    assert_eq!(boxes.len(), 2);
    let first_end = attribute(boxes[0], "x") + attribute(boxes[0], "width");
    assert!((first_end - attribute(boxes[1], "x")).abs() < 0.001);
    // The combining mark stays in the same shaping run as its base character.
    let run = svg
        .lines()
        .find(|line| line.contains("<text ") && line.contains("e\u{301}"))
        .unwrap();
    assert_eq!(run.matches("<text ").count(), 1);
    assert!(run.contains("<text style=\"stroke: none\"") && run.contains("e\u{301}</text>"));
    let runs = svg
        .lines()
        .filter(|line| line.contains(r#"<text style="stroke: none""#))
        .collect::<Vec<_>>();
    assert_eq!(runs.len(), 2);
    assert!((attribute(runs[1], "x") - attribute(boxes[1], "x")).abs() < 0.001);
}

#[test]
fn a_highlight_boundary_keeps_an_emoji_grapheme_together() {
    let source = r##"
        #[kaalang]
        fn emoji(value: u8) -> u8 {
            #[action(r#"<mark>👩</mark>&zwj;💻"#)]
            let result = |value| value;
            |result| return result;
        }
    "##;
    let svg = render_source(source, "emoji").unwrap();
    let run = svg
        .lines()
        .find(|line| line.contains("<text ") && line.contains("👩‍💻"))
        .unwrap();
    assert_eq!(run.matches("<text ").count(), 1);
    assert!(run.contains(r#"<text style="stroke: none""#) && run.contains("👩‍💻</text>"));
}

#[test]
fn ordinary_styled_text_keeps_graphemes_together() {
    let source = r#"
        #[kaalang]
        fn graphemes() {
            #[action("**👩**&zwj;💻 and e<color name=\"red\">\u{301}</color>")]
            {};
            return;
        }
    "#;
    let svg = render_source(source, "graphemes").unwrap();

    assert!(svg.contains("<tspan class=\"md-bold\">👩‍💻</tspan>"));
    assert!(svg.contains("and e\u{301}</tspan>"));
}

#[test]
fn a_single_highlighted_grapheme_uses_its_measured_advance() {
    let source = r#"
        #[kaalang]
        fn measured() {
            #[action("<mark>W</mark>$x$")]
            {};
            return;
        }
    "#;
    let svg = render_source(source, "measured").unwrap();
    let run = svg.lines().find(|line| line.contains(">W</text>")).unwrap();
    assert!(run.contains(r#"lengthAdjust="spacingAndGlyphs""#));
    let box_line = svg
        .lines()
        .find(|line| line.contains(r#"<rect class="md-highlight-box""#))
        .unwrap();
    let formula = svg
        .lines()
        .find(|line| line.contains(r#"<svg class="md-math""#))
        .unwrap();
    assert!((attribute(run, "textLength") - attribute(box_line, "width")).abs() < 0.001);
    assert!(
        (attribute(formula, "x") - attribute(box_line, "x") - attribute(box_line, "width")).abs()
            < 0.001
    );
}

#[test]
fn width_only_formula_advances_text_without_an_empty_svg() {
    let source = r#"
        #[kaalang]
        fn spaced() {
            #[action(r"a<u>$\,$</u>b")]
            {};
            return;
        }
    "#;
    let svg = render_source(source, "spaced").unwrap();
    assert!(svg.contains(r#"<title xml:space="preserve">a\,b</title>"#));
    assert!(!svg.contains(r#"<svg class="md-math""#));
    assert!(svg.contains(r#"<line class="md-math""#));
    let a = svg.lines().find(|line| line.contains(">a</text>")).unwrap();
    let b = svg.lines().find(|line| line.contains(">b</text>")).unwrap();
    let gap = attribute(b, "x") - attribute(a, "x") - attribute(a, "textLength");
    assert!((gap - 3.0).abs() < 0.001);
}

#[test]
fn a_formula_too_tall_for_scene_coordinates_stays_literal() {
    let formula = r"$x\rule{1em}{1000000000em}$";
    let source = format!("#[kaalang] fn huge() {{ #[action(r\"{formula}\")] {{}}; return; }}");
    let svg = render_source(&source, "huge").expect("oversized math uses literal fallback");

    assert!(svg.contains(&format!("<title xml:space=\"preserve\">{formula}</title>")));
    assert!(describe(&svg).contains(formula));
    assert!(!svg.contains("<svg class=\"md-math"));
}

fn formula_scale(math: &str) -> f64 {
    attribute(math, "width") / view_box(math)[2]
}

/// The first `name="…"` value on one line of the SVG.
fn quoted<'a>(line: &'a str, name: &str) -> &'a str {
    line.split_once(&format!("{name}=\""))
        .and_then(|(_, value)| value.split_once('"'))
        .expect("the line carries the attribute")
        .0
}

fn attribute(line: &str, name: &str) -> f64 {
    quoted(line, name)
        .parse()
        .expect("the attribute is numeric")
}

fn view_box(line: &str) -> Vec<f64> {
    quoted(line, "viewBox")
        .split_whitespace()
        .map(|value| value.parse().expect("the viewBox is numeric"))
        .collect()
}

#[test]
fn an_oversized_cycle_formula_is_clipped_at_its_base_size() {
    let math = format!("{}x", "x+".repeat(70));
    let source = CYCLE_SOURCE.replace("Count to the limit.", &format!("${math}$"));
    let expanded = render_source(&source, "count_to").unwrap();
    let caption = expanded
        .split_once(r#"<g class="cycle-caption""#)
        .map(|(_, tail)| tail.split_once("</g>").unwrap().0);
    if let Some(caption) = caption
        && let Some(math) = caption
            .lines()
            .find(|line| line.contains(r#"<svg class="md-math""#))
    {
        assert!(math.contains("style=\"overflow: hidden\""));
        assert!((formula_scale(math) - 12.0).abs() < 0.01);
    }

    let collapsed = render_source_with_options(
        &source,
        "count_to",
        RenderOptions {
            collapse_loops: true,
        },
    )
    .unwrap();
    let math = collapsed
        .lines()
        .find(|line| line.contains(r#"<svg class="md-math""#))
        .unwrap();
    assert!(math.contains("style=\"overflow: hidden\""));
    assert!((formula_scale(math) - 14.0).abs() < 0.01);
}

#[test]
fn unsupported_html_tags_are_escaped_around_styled_contents() {
    let source = r#"
        #[kaalang]
        fn literal(value: u8) -> u8 {
            #[action("<b>**warning**</b> <b><u>x</u></b>\n<b><i>x</b><mark>**y**</mark></i>")]
            let result = |value| value;
            |result| return result;
        }
    "#;
    let svg = render_source(source, "literal").unwrap();
    assert!(svg.contains(r#"&lt;b&gt;<tspan class="md-bold">warning</tspan>&lt;/b&gt;"#));
    assert!(svg.contains(r#"&lt;b&gt;<tspan class="md-underline">x</tspan>&lt;/b&gt;"#));
    assert!(svg.contains(r#"<rect class="md-highlight-box""#));
    assert!(svg.contains(r#"<tspan class="md-bold">y</tspan>"#));
    assert!(describe(&svg).contains("&lt;b&gt;warning&lt;/b&gt; &lt;b&gt;x&lt;/b&gt;"));
    assert!(describe(&svg).contains("&lt;b&gt;&lt;i&gt;x&lt;/b&gt;y&lt;/i&gt;"));
    assert!(!svg.contains("<b>") && !svg.contains("<i>") && !svg.contains("<mark>"));
}

#[test]
fn surrounding_styles_reach_formula_paths_and_decorations() {
    let source = r##"
        #[kaalang]
        fn formula(value: u8) -> u8 {
            #[action(r#"**~~*<u><mark>$x$</mark></u>*~~**"#)]
            let first = |value| value;
            #[action(r#"^$x$^ and ~$x$~"#)]
            let second = |first| first;
            #[action("> $x$")]
            let result = |second| second;
            |result| return result;
        }
    "##;
    let svg = render_source(source, "formula").unwrap();
    assert!(svg.contains(r#"<svg class="md-math md-bold""#));
    assert!(svg.contains(".md-math.md-bold path { stroke: currentColor;"));
    assert_eq!(svg.matches("skewX(-8.5308)").count(), 2);
    assert!(svg.contains(r#"<rect class="md-highlight-box""#));
    assert_eq!(svg.matches("<line x1=\"0\"").count(), 2);
    assert!(svg.matches(r#"<svg class="md-math""#).count() >= 3);
}

/// A connection label strokes a white halo around its text, which would paint
/// over the highlight box beneath it. A highlighted run drops that halo.
#[test]
fn a_highlighted_branch_description_drops_the_halo_over_its_box() {
    let source = r##"
        #[kaalang]
        fn highlighted(condition: bool) -> u8 {
            #[question("Choose.")]
            #[yes(r#"<mark>take it</mark>"#)]
            #[no]
            let (yes, no) = |condition| condition;
            #[action("Yes.")]
            let end = |yes| 1;
            #[action("No.")]
            let end = |no| 0;
            |end| return end;
        }
    "##;
    let svg = render_source(source, "highlighted").unwrap();
    let group = svg
        .split_once(r#"<g class="connection-label branch-label""#)
        .expect("the branch description is a composed group")
        .1
        .split_once("</g>")
        .expect("the group closes")
        .0;

    assert!(group.contains(r#"<rect class="md-highlight-box""#));
    assert!(group.contains(r#"<text style="stroke: none""#));
    assert!(group.contains(">take it</text>"));
    // The unhighlighted labels of the same diagram keep their halo.
    assert!(svg.contains("stroke: #ffffff; stroke-width: 5px"));

    for formula in [
        "$x$".to_owned(),
        "**$x$**".to_owned(),
        format!("${}x$", "x+".repeat(70)),
    ] {
        for (source, name) in [
            (source.replace("take it", &formula), "highlighted"),
            (
                CYCLE_SOURCE.replace("Count to the limit.", &format!("<mark>{formula}</mark>")),
                "count_to",
            ),
        ] {
            let svg = render_source(&source, name).unwrap();
            let math = svg
                .lines()
                .find(|line| line.contains("<svg class=\"md-math"))
                .unwrap();
            assert!(math.contains("stroke=\"none\""), "{name}: {math}");
            if let Some(ellipsis) = svg.lines().find(|line| line.contains("md-math-ellipsis")) {
                assert!(ellipsis.contains("stroke=\"none\""), "{name}: {ellipsis}");
            }
        }
    }
}

#[test]
fn renders_cycles_as_expanded_boundaries_or_collapsed_nodes() {
    let expanded = render_source(CYCLE_SOURCE, "count_to").expect("the cycle expands");
    let collapsed = render_source_with_options(
        CYCLE_SOURCE,
        "count_to",
        RenderOptions {
            collapse_loops: true,
        },
    )
    .expect("the cycle collapses");

    assert!(expanded.contains(r#"class="cycle-boundary""#));
    assert!(!expanded.contains(r#"class="node loop""#));
    assert!(expanded.contains("Increment the counter."));
    assert!(!expanded.contains("cycle-interface"));
    assert!(expanded.contains(r#"<title xml:space="preserve">Count to the limit.</title>"#));
    assert!(expanded.contains(">Count to the limit.</tspan>"));

    assert!(collapsed.contains(r#"class="node loop""#));
    assert!(!collapsed.contains(r#"class="cycle-boundary""#));
    assert!(!collapsed.contains("Increment the counter."));
    assert!(collapsed.contains("Count to the limit."));
    assert!(collapsed.contains("Cycle: Count to the limit."));
    assert!(collapsed.contains(">mut count, limit</tspan>"));
    assert!(collapsed.contains(">total</tspan>"));
    assert!(collapsed.contains(
        ".action .label, .call .label, .loop .label { font-weight: 500; text-anchor: start; }"
    ));

    let invalid = CYCLE_SOURCE.replace("break count", "return count");
    for collapse_loops in [false, true] {
        assert!(matches!(
            render_source_with_options(&invalid, "count_to", RenderOptions { collapse_loops }),
            Err(RenderError::InvalidFlow { .. })
        ));
    }
}

#[test]
fn expanded_cycles_keep_shortened_descriptions_in_their_tooltips() {
    let description = "Collect <the results> & keep processing until there are enough items to complete the current request. ".repeat(3);
    let source = CYCLE_SOURCE.replace("Count to the limit.", &description);
    let svg = render_source(&source, "count_to").expect("the long cycle caption renders");
    let escaped = description
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    assert!(svg.contains(&format!("<title xml:space=\"preserve\">{escaped}</title>")));
    let caption = svg
        .lines()
        .find(|line| line.contains("<text class=\"cycle-caption\""))
        .unwrap();
    assert!(caption.contains("xml:space=\"preserve\""));
    assert!(caption.contains("…</tspan>"));
    assert_eq!(caption.matches("<tspan").count(), 2);
}

#[test]
fn a_routing_error_preserves_the_flow_name_and_failed_rule() {
    let error = RenderError::UnroutableTopology {
        name: "example".to_owned(),
        reason: "two connections cross".to_owned(),
    };
    assert_eq!(
        error.to_string(),
        "could not route kaalang flow `example`: two connections cross"
    );
    assert_eq!(error.clone(), error);
}

#[test]
fn the_description_names_every_role_and_label_once() {
    let svg = render_source(SOURCE, "describe").expect("the flow has a conforming diagram");
    let description = describe(&svg);
    assert!(
        !description.contains("capturing nothing"),
        "start and cases have no capture marker"
    );

    // A choice is a select, and end carries both the return type and value.
    assert!(
        description.contains("Select: Pick a lane."),
        "the select is not named as one: {description}"
    );
    assert!(
        description.contains("End: u8 capturing end"),
        "the end node does not carry its return type and input: {description}"
    );
    assert!(
        description.contains("Merge: end"),
        "the merge is not named as a node"
    );

    for (label, what) in [
        // A flow input nothing captures is still handed over, once. `_spare`
        // itself also appears inside the start node's signature, so the
        // hand-over is what gets counted.
        ("handing over seed, _spare", "an unused hand-over"),
        // A borrowed capture keeps its `&`, escaped, and is drawn once however
        // many connections arrive.
        ("capturing &amp;width", "a borrowed capture"),
        // Outputs captured by separate steps are still handed over once.
        ("handing over width, depth", "a multi-wire hand-over"),
    ] {
        assert_eq!(
            description.matches(label).count(),
            1,
            "{what} `{label}` is not named exactly once: {description}"
        );
    }
}

#[test]
fn the_end_label_omits_the_arrow_regardless_of_spacing() {
    for output in ["->u8", "-> u8", "->\n\tu8"] {
        let source = format!("#[kaalang] fn example(end: u8){output} {{ |end| return end; }}");
        let svg = render_source(&source, "example").unwrap();
        assert!(!svg.contains("-&gt;"), "{output}");
        assert!(describe(&svg).contains("End: u8 capturing end"), "{output}");
    }
}

fn describe(svg: &str) -> String {
    let opened = svg
        .split_once(r#"<desc id="kaalang-description">"#)
        .expect("a rendered diagram carries a description")
        .1;

    opened
        .split_once("</desc>")
        .expect("the description is closed")
        .0
        .to_owned()
}
