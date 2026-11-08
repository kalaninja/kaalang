use kaalang::kaa;

#[kaa]
fn render_markdown(condition: bool) {
    #[kaa::action("Plain, **bold**, *italic*, and ***bold italic*** text.")]
    let code = |condition| condition;

    #[kaa::action("Use ~~obsolete~~ and `inline code` text.")]
    let tags = |code| code;

    #[kaa::action(r#"Styles: <u>underlined</u>, <mark>highlighted</mark>, and <color name="blue">blue</color>."#)]
    let indices = |tags| tags;

    #[kaa::action("Indices: x^2^ and H~2~O.")]
    let quoted = |indices| indices;

    #[kaa::action("> Quoted **guidance** remains distinct.")]
    let highlighted = |quoted| quoted;

    // A highlighted run is drawn at its measured width. Narrow glyphs are where
    // that estimate stands furthest from the font, so the same phrase plain and
    // bold shows how much a composed run is adjusted to fill its slot.
    #[kaa::action("<mark>Little titles fit in a strict, tidy list.</mark>")]
    let bold_highlight = |highlighted| highlighted;

    #[kaa::action("<mark>**Little titles fit in a strict, tidy list.**</mark>")]
    let mixed_highlight = |bold_highlight| bold_highlight;

    #[kaa::action("Plain text, <mark>one highlighted phrase</mark>, plain again.")]
    let graphemes = |mixed_highlight| mixed_highlight;

    #[kaa::action("**👩**&zwj;💻 and e<color name=\"red\">\u{301}</color>.")]
    let styled_graphemes = |graphemes| graphemes;

    #[kaa::action("<mark>👩</mark>&zwj;💻 and <mark>e</mark>\u{301}.")]
    let tested = |styled_graphemes| styled_graphemes;

    |tested| return;
}

#[test]
fn markdown_preserves_execution() {
    render_markdown(true);
    render_markdown(false);
}
