use kaalang::kaa;

#[kaa]
fn render_markup_fallbacks(condition: bool) {
    #[kaa::action(r#"<b>**warning**</b> <b><u>x</u></b> **outside**"#)]
    let crossing = |condition| condition;

    #[kaa::action("**<b>foo**</b> after\n<b>**foo</b> after**")]
    let overlapping = |crossing| crossing;

    #[kaa::action("<b><i>x</b><mark>**y**</mark></i>")]
    let noncanonical = |overlapping| overlapping;

    #[kaa::action("<B>**warning**</b>\n<u>**foo**</u >")]
    let enclosing = |noncanonical| noncanonical;

    #[kaa::action("**<b>foo*</b>*")]
    let indented = |enclosing| enclosing;

    #[kaa::action("   > **quote**  ")]
    let colors = |indented| indented;

    #[kaa::action(r"$\color{not-a-color}x$")]
    let carriage_returns = |colors| colors;

    #[kaa::action("**Two\r\rparagraphs** stay literal.\n**One\rparagraph** keeps its style.")]
    let empty_tags = |carriage_returns| carriage_returns;

    #[kaa::action("<u></u>\n<mark></mark>\n<color name=\"red\"></color>")]
    let tested = |empty_tags| empty_tags;

    |tested| return;
}

#[test]
fn markup_fallbacks_preserve_execution() {
    render_markup_fallbacks(true);
    render_markup_fallbacks(false);
}
