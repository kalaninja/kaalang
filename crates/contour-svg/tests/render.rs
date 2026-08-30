use contour_svg::{RenderError, render_source};

const SOURCE: &str = include_str!("fixtures/all_blocks.rs");

#[test]
fn renders_the_golden_diagram() {
    let svg = render_source(SOURCE, "route").unwrap();

    assert_eq!(svg, include_str!("fixtures/all_blocks.svg"));
}

/// The golden diagram pins the rest of the rendering; this covers only the two
/// claims it cannot state on its own.
#[test]
fn output_is_deterministic_and_escapes_authored_text() {
    let first = render_source(SOURCE, "route").unwrap();
    let second = render_source(SOURCE, "route").unwrap();

    assert_eq!(first, second);
    assert!(first.contains("Есть &lt;заявка&gt; &amp; она подходит?"));
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
            |first| -> first_result { 1 };

            #[action("second")]
            |second| -> second_result { 2 };
        }
    "#;

    let svg = render_source(source, "spacing").unwrap();

    assert!(svg.contains("<title xml:space=\"preserve\">  exact\tchoice  </title>"));
    assert!(svg.contains("first /   first  case  "));
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
        }
    "#;

    let svg = render_source(source, "carriage_return").unwrap();

    assert!(svg.contains("left&#13;right"));
    assert!(!svg.contains('\r'));
}
