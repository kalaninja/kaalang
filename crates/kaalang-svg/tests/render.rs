use kaalang_svg::{RenderError, render_source};

/// One flow carrying every label case at once, with a distinct wire name for
/// each so a duplicate in the description is detectable by counting.
const SOURCE: &str = r#"
    #[kaalang]
    fn describe(seed: u8, _spare: u8) -> u8 {
        #[choice("Pick a lane.")]
        #[case("The near lane.")]
        #[case("The far lane.")]
        |seed| -> (near, far) {
            match seed {
                0 => (),
                _ => (),
            }
        };

        #[action("Split <the> near lane & measure it.")]
        |near| -> (width, depth) { (1u8, 2u8) };

        #[action("Measure the width.")]
        |&width| -> measured { *width };

        #[action("Gauge the depth.")]
        |depth| -> gauged { depth };

        #[action("Finish from the near lane.")]
        |measured, gauged| -> result { measured + gauged };

        #[action("Finish from the far lane.")]
        |far| -> result { 9u8 };
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
    assert_eq!(svg.matches(r#"class="parameter-panel""#).count(), 1);
    assert!(svg.contains(">seed: u8</tspan><tspan"));
    assert!(svg.contains(">_spare: u8</tspan></text>"));
    assert!(svg.contains(
        ".start .node-shape, .end .node-shape, .parameter-panel .node-shape { fill: #f0f9ff; }"
    ));
    assert!(svg.contains(".action .node-shape { fill: #f8fafc; }"));
    assert!(svg.contains(".question .node-shape { fill: #fffbeb; }"));
    assert!(svg.contains(".select .node-shape, .case .node-shape { fill: #f5f3ff; }"));
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

    // A choice is a select, and the end node carries the return type without
    // the arrow that its drawn caption keeps.
    assert!(
        description.contains("Select: Pick a lane."),
        "the select is not named as one: {description}"
    );
    assert!(
        description.contains("End: u8"),
        "the end node does not carry its return type: {description}"
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
fn the_end_description_omits_the_arrow_regardless_of_spacing() {
    for output in ["->u8", "-> u8", "->\n\tu8"] {
        let source = format!("#[kaalang] fn example(result: u8){output} {{}}");
        let svg = render_source(&source, "example").unwrap();
        assert!(
            describe(&svg).contains("End: u8 capturing result"),
            "{output}"
        );
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
