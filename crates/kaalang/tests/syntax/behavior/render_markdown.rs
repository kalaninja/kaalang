use kaalang::kaalang;

#[kaalang]
fn render_markdown(section: u8, condition: bool) {
    #[choice("Markdown examples.")]
    #[case("Text effects.")]
    #[case("Formulas and graphemes.")]
    #[case("HTML and literal fallbacks.")]
    let (emphasis, display, html) = |section, condition| match section {
        0 => condition,
        1 => condition,
        _ => condition,
    };

    #[action("Plain, **bold**, *italic*, and ***bold italic*** text.")]
    let code = |emphasis| emphasis;

    #[action("Use ~~obsolete~~ and `inline code` text.")]
    let tags = |code| code;

    #[action(r#"Styles: <u>underlined</u>, <mark>highlighted</mark>, and <color name="blue">blue</color>."#)]
    let indices = |tags| tags;

    #[action("Indices: x^2^ and H~2~O.")]
    let quoted = |indices| indices;

    #[action("> Quoted **guidance** remains distinct.")]
    let highlighted = |quoted| quoted;

    // A highlighted run is drawn at its measured width. Narrow glyphs are where
    // that estimate stands furthest from the font, so the same phrase plain and
    // bold shows how much a composed run is adjusted to fill its slot.
    #[action("<mark>Little titles fit in a strict, tidy list.</mark>")]
    let bold_highlight = |highlighted| highlighted;

    #[action("<mark>**Little titles fit in a strict, tidy list.**</mark>")]
    let mixed_highlight = |bold_highlight| bold_highlight;

    #[action("Plain text, <mark>one highlighted phrase</mark>, plain again.")]
    let tested = |mixed_highlight| mixed_highlight;

    #[action(
        r"$$\int_{0}^{\infty}\frac{x^{s-1}}{e^x-1}\,dx=\Gamma(s)\sum_{n=1}^{\infty}\frac{1}{n^s}$$"
    )]
    let tinted = |display| display;

    #[action(r#"Formulae: <color name="purple"><u>$x^2 + y^2$</u></color> and <mark>$$\frac{a+b}{2}$$</mark>."#)]
    let styled = |tinted| tinted;

    #[action("~~$x$~~, **$x$**, *$x$*, ^$x$^, and ~$x$~.")]
    let quoted_math = |styled| styled;

    #[action("> Quoted formula: $x$.")]
    let graphemes = |quoted_math| quoted_math;

    #[action("**👩**&zwj;💻 and e<color name=\"red\">\u{301}</color>.")]
    let styled_graphemes = |graphemes| graphemes;

    #[action("<mark>👩</mark>&zwj;💻 and <mark>e</mark>\u{301}.")]
    let measured = |styled_graphemes| styled_graphemes;

    #[action("<mark>W</mark>$x$")]
    let colored = |measured| measured;

    #[action(
        r"$\definecolor{foo}{rgb}{1,0,0}x$ and $\definecolor{foo}{rgb}{1,0,0}\textcolor{foo}{x}$"
    )]
    let framed = |colored| colored;

    #[action(r"$\overline{x}$, $\colorbox{yellow}{x}$ and $\fcolorbox{blue}{yellow}{x}$")]
    let clipped = |framed| framed;

    #[action("<u>$x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x$</u>")]
    let huge = |clipped| clipped;

    #[action(r"$x\rule{1em}{1000000000em}$")]
    let tested = |huge| huge;

    #[action(r#"<b>**warning**</b> <b><u>x</u></b> **outside**"#)]
    let crossing = |html| html;

    #[action("**<b>foo**</b> after\n<b>**foo</b> after**")]
    let overlapping = |crossing| crossing;

    #[action("<b><i>x</b><mark>**y**</mark></i>")]
    let noncanonical = |overlapping| overlapping;

    #[action("<B>**warning**</b>\n<u>**foo**</u >")]
    let enclosing = |noncanonical| noncanonical;

    #[action("**<b>foo*</b>*")]
    let indented = |enclosing| enclosing;

    #[action("   > **quote**  ")]
    let colors = |indented| indented;

    #[action(r"$\color{not-a-color}x$")]
    let empty_tags = |colors| colors;

    #[action("<u></u>\n<mark></mark>\n<color name=\"red\"></color>")]
    let tested = |empty_tags| empty_tags;

    #[question("Highlighted formulas on branch labels.")]
    #[yes("<mark>$x$</mark>")]
    #[no("<mark>**$x$**</mark>")]
    let (yes, no) = |tested| tested;

    #[action("Take the first formula.")]
    let done = |yes| {};

    #[action("Take the second formula.")]
    let done = |no| {};

    #[cycle("<mark>$x$</mark>")]
    |done| {
        #[action("Complete one pass.")]
        let ready = |done| {};

        |ready| break;
    };

    return;
}

#[test]
fn markdown_preserves_execution() {
    for section in 0..3 {
        render_markdown(section, true);
        render_markdown(section, false);
    }
}
