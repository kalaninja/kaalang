use contour_svg::{RenderError, render_source};

const SOURCE: &str = include_str!("fixtures/all_blocks.rs");
const END_COLLECTOR: &str = include_str!("fixtures/end_collector.rs");

#[test]
fn renders_the_golden_diagram() {
    let svg = render_source(SOURCE, "route").unwrap();

    assert_eq!(svg, include_str!("fixtures/all_blocks.svg"));
}

/// Three branches reach End from clear columns. Each descends in its own
/// column onto the one collector above End, and the collector makes the single
/// descent into the node.
#[test]
fn renders_the_golden_end_collector_diagram() {
    let svg = render_source(END_COLLECTOR, "route").unwrap();

    assert_eq!(svg, include_str!("fixtures/end_collector.svg"));
}

#[test]
fn renders_the_authored_end_with_accessible_terminology() {
    let source = r#"
        #[contour]
        fn finish(input: u8) -> u8 {
            #[action("Build the result")]
            |input| -> result { input };

            #[end]
            |result| {};
        }
    "#;

    let svg = render_source(source, "finish").unwrap();

    assert_eq!(svg.matches("class=\"node end\"").count(), 1);
    assert!(svg.contains("; End. Connections:"));
    assert!(svg.contains("Action: Build the result to End via result"));
    assert!(svg.contains(">End</text>"));
    assert!(!svg.to_ascii_lowercase().contains("return"));
}

#[test]
fn escapes_authored_text() {
    let source = r#"
        #[contour]
        fn escaping(input: u8) -> u8 {
            #[action("<tag> & value")]
            |input| -> output { input };

            #[end]
            |output| {};
        }
    "#;

    let svg = render_source(source, "escaping").unwrap();

    assert!(svg.contains("<title xml:space=\"preserve\">&lt;tag&gt; &amp; value</title>"));
}

#[test]
fn uses_contour_icons_without_type_captions_or_arrows() {
    let svg = render_source(SOURCE, "route").unwrap();

    assert!(svg.contains("class=\"node question\""));
    assert!(svg.contains("class=\"node choice select\""));
    assert!(svg.contains("class=\"node case\""));
    assert!(svg.contains(">accepted</tspan>"));
    assert!(svg.contains(">rejected</tspan>"));
    assert!(!svg.contains(" / yes"));
    assert!(!svg.contains(" / no"));
    assert!(!svg.contains("class=\"kind\""));
    assert!(!svg.contains("marker-end"));
}

/// Under `xml:space="preserve"` the whitespace between a `<text>` element and
/// its `<tspan>` children is rendered content: it joins the text chunk and
/// shifts a centred label off its node. Each label stays on one line.
#[test]
fn labels_carry_no_pretty_printed_whitespace() {
    let svg = render_source(SOURCE, "route").unwrap();

    let wrapped = svg.lines().filter(|line| line.contains("<tspan"));
    let mut counted = 0;
    for line in wrapped {
        counted += 1;
        assert!(
            line.trim_start().starts_with("<text ") && line.ends_with("</text>"),
            "a tspan must share its text element's line: {line}"
        );
    }
    assert!(counted > 0, "the fixture must contain wrapped labels");
}

#[test]
fn reports_an_unknown_flow_without_falling_back() {
    assert_eq!(
        render_source(SOURCE, "missing"),
        Err(RenderError::FlowNotFound("missing".into()))
    );
}

#[test]
fn preserves_whitespace_in_block_and_case_labels() {
    let source = r#"
        #[contour]
        fn spacing(input: bool) -> u8 {
            #[choice("  exact\tchoice  ")]
            #[case("  first  case  ")]
            #[case("second")]
            |input| -> (first, second) {
                match input {
                    true => (),
                    false => (),
                }
            };

            #[action("first")]
            |first| -> result { 1 };

            #[action("second")]
            |second| -> result { 2 };

            #[end]
            |result| {};
        }
    "#;

    let svg = render_source(source, "spacing").unwrap();

    assert!(svg.contains("<title xml:space=\"preserve\">  exact\tchoice  </title>"));
    assert!(svg.contains("<title xml:space=\"preserve\">  first  case  </title>"));
    assert!(svg.contains(">first</tspan>"));
    assert!(svg.contains("aria-describedby=\"contour-description\""));
    assert!(svg.contains("<desc id=\"contour-description\">"));
}

#[test]
fn preserves_carriage_returns_through_xml_parsing() {
    let source = r#"
        #[contour]
        fn carriage_return(input: u8) -> u8 {
            #[action("left\rright")]
            |input| -> result { input };

            #[end]
            |result| {};
        }
    "#;

    let svg = render_source(source, "carriage_return").unwrap();

    assert!(svg.contains("left&#13;right"));
    assert!(!svg.contains('\r'));
}
