use kaalang::kaalang;

#[kaalang]
fn render_markdown() {
    #[action("Plain, **bold**, *italic*, and ***bold italic*** text.")]
    {};

    #[action("Use ~~obsolete~~ and `inline code` text.")]
    {};

    #[action(r#"Styles: <u>underlined</u>, <mark>highlighted</mark>, and <color name="blue">blue</color>."#)]
    {};

    #[action("Indices: x^2^ and H~2~O.")]
    {};

    #[action("> Quoted **guidance** remains distinct.")]
    {};

    #[action(
        r"$$\int_{0}^{\infty}\frac{x^{s-1}}{e^x-1}\,dx=\Gamma(s)\sum_{n=1}^{\infty}\frac{1}{n^s}$$"
    )]
    {};

    #[action(r#"Formulae: <color name="purple"><u>$x^2 + y^2$</u></color> and <mark>$$\frac{a+b}{2}$$</mark>."#)]
    {};

    // A highlighted run is drawn at its measured width. Narrow glyphs are where
    // that estimate stands furthest from the font, so the same phrase plain and
    // bold shows how much a composed run is adjusted to fill its slot.
    #[action("<mark>Little titles fit in a strict, tidy list.</mark>")]
    {};

    #[action("<mark>**Little titles fit in a strict, tidy list.**</mark>")]
    {};

    #[action("Plain text, <mark>one highlighted phrase</mark>, plain again.")]
    {};

    #[action(r#"<b>**warning**</b> <b><u>x</u></b> **outside**"#)]
    {};

    #[action("~~$x$~~, **$x$**, *$x$*, ^$x$^, and ~$x$~.")]
    {};

    #[action("> Quoted formula: $x$.")]
    {};

    #[action("<mark>👩</mark>&zwj;💻 and <mark>e</mark>\u{301}.")]
    {};

    return;
}

#[test]
fn markdown_renders_without_wires() {
    render_markdown();
}
