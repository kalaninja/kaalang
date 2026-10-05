use kaalang::kaalang;

#[kaalang]
fn render_markup_fallbacks(condition: bool) {
    #[action(r#"<b>**warning**</b> <b><u>x</u></b> **outside**"#)]
    let crossing = |condition| condition;

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
    let carriage_returns = |colors| colors;

    #[action("**Two\r\rparagraphs** stay literal.\n**One\rparagraph** keeps its style.")]
    let empty_tags = |carriage_returns| carriage_returns;

    #[action("<u></u>\n<mark></mark>\n<color name=\"red\"></color>")]
    let tested = |empty_tags| empty_tags;

    |tested| return;
}

#[test]
fn markup_fallbacks_preserve_execution() {
    render_markup_fallbacks(true);
    render_markup_fallbacks(false);
}
