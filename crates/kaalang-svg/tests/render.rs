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
    assert!(svg.contains(".action .node-shape { fill: #f8fafc; }"));
    assert!(svg.contains(".question .node-shape { fill: #fffbeb; }"));
    assert!(svg.contains(".select .node-shape, .case .node-shape { fill: #f5f3ff; }"));
    assert!(svg.contains(".branch-label { fill: currentColor; font-weight: 500; }"));
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
    assert!(
        collapsed
            .contains(".action .label, .loop .label { font-weight: 500; text-anchor: start; }")
    );

    let invalid = CYCLE_SOURCE.replace("break count", "return count");
    for collapse_loops in [false, true] {
        assert!(matches!(
            render_source_with_options(&invalid, "count_to", RenderOptions { collapse_loops }),
            Err(RenderError::InvalidFlow { .. })
        ));
    }
}

#[test]
fn renders_nested_cycles_without_crossing_boundaries() {
    for (source, name) in [
        (
            include_str!("../../kaalang/tests/loop/behavior/nested_alternating_returns.rs"),
            "nested_alternating_returns",
        ),
        (
            include_str!("../../kaalang/tests/loop/behavior/nested_side_returns.rs"),
            "nested_side_returns",
        ),
    ] {
        let svg = render_source(source, name)
            .expect("nested cycle back edges clear every enclosed boundary");

        assert_eq!(svg.matches(r#"class="cycle-boundary""#).count(), 3);
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
fn a_transit_connection_does_not_imply_a_capture() {
    let svg = render_source(
        include_str!("../../kaalang/tests/wire/behavior/shared_setup.rs"),
        "shared_setup",
    )
    .expect("the setup carries the start connection through to the question");
    assert_eq!(svg.matches(">()</tspan>").count(), 1);
    assert!(
        describe(&svg)
            .contains("Action: Prepare the shared setup. capturing nothing handing over setup")
    );
}

#[test]
fn mutable_captures_keep_their_modifiers_in_labels_and_descriptions() {
    let svg = render_source(
        include_str!("../../kaalang/tests/capture/behavior/mutate_wires.rs"),
        "mutate_wires",
    )
    .expect("mutable captures render with their authored modifiers");
    let description = describe(&svg);
    assert!(svg.contains(">mut r#type: String</tspan>"));
    assert!(description.contains("with parameters mut r#type: String; count: u32"));
    for capture in ["mut count", "&amp;mut type", "&amp;type", "mut type"] {
        assert!(svg.contains(&format!(">{capture}")), "{capture}");
        assert!(
            description.contains(&format!("capturing {capture}")),
            "{capture}"
        );
    }
    assert!(description.contains("capturing &amp;mut text, &amp;mut total"));
}

#[test]
fn wire_labels_on_a_vertical_route_share_a_start_anchor() {
    let svg = render_source(
        include_str!("../../kaalang/tests/capture/behavior/mutate_wires.rs"),
        "mutate_wires",
    )
    .unwrap();
    let style = svg
        .lines()
        .find(|line| line.contains(".connection-label {"))
        .unwrap();
    assert!(style.contains("text-anchor: start;"));

    let positions = svg
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix(r#"<text class="connection-label" x=""#)
        })
        .map(|label| label.split_once('"').unwrap().0)
        .collect::<Vec<_>>();
    assert!(positions.len() > 1);
    assert!(positions.iter().all(|x| *x == positions[0]));
}

#[test]
fn terminal_cases_render_beside_a_wide_shared_continuation() {
    let svg = render_source(
        include_str!("../../kaalang/tests/wire/behavior/blocked_terminal_crossing.rs"),
        "blocked_terminal_crossing",
    )
    .expect("terminal cases leave room for every case of the shared inner choice");
    assert_eq!(svg.matches(r#"class="node case""#).count(), 8);
    assert_eq!(svg.matches(r#"class="node end""#).count(), 1);
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
