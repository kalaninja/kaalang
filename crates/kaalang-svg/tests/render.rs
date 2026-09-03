use kaalang_svg::render_source;

#[test]
fn renders_an_accessible_standalone_svg() {
    let source = r#"
        #[kaalang]
        fn finish(input: u8) -> u8 {
            #[action("<tag> & value")]
            |input| -> result { input };

            #[end]
            |result| {};
        }
    "#;

    let svg = render_source(source, "finish").unwrap();

    assert!(svg.contains(
        r#"role="img" aria-labelledby="kaalang-title" aria-describedby="kaalang-description""#
    ));
    assert!(svg.contains(r#"<title id="kaalang-title">kaalang diagram for finish</title>"#));
    assert!(svg.contains(r#"<desc id="kaalang-description">"#));
    assert!(svg.contains(r#"<title xml:space="preserve">&lt;tag&gt; &amp; value</title>"#));
    assert!(svg.contains("Action: &lt;tag&gt; &amp; value to End via result"));
    assert_eq!(svg.matches(r#"class="node end""#).count(), 1);
}
