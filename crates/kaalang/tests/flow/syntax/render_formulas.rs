use kaalang::kaa;

#[kaa]
fn render_formulas(condition: bool) {
    #[kaa::action(
        r"$$\int_{0}^{\infty}\frac{x^{s-1}}{e^x-1}\,dx=\Gamma(s)\sum_{n=1}^{\infty}\frac{1}{n^s}$$"
    )]
    let tinted = |condition| condition;

    #[kaa::action(r#"Formulae: <color name="purple"><u>$x^2 + y^2$</u></color> and <mark>$$\frac{a+b}{2}$$</mark>."#)]
    let styled = |tinted| tinted;

    #[kaa::action(r"~~$x$~~, **$x$**, *$x$*, ^$x$^, ~$x$~, and a<u>$\,$</u>b.")]
    let quoted_math = |styled| styled;

    #[kaa::action("> Quoted formula: $x$.")]
    let measured = |quoted_math| quoted_math;

    #[kaa::action("<mark>W</mark>$x$")]
    let colored = |measured| measured;

    #[kaa::action(
        r"$\definecolor{foo}{rgb}{1,0,0}x$ and $\definecolor{foo}{rgb}{1,0,0}\textcolor{foo}{x}$"
    )]
    let framed = |colored| colored;

    #[kaa::action(r"$\overline{x}$, $\colorbox{yellow}{x}$ and $\fcolorbox{blue}{yellow}{x}$")]
    let clipped = |framed| framed;

    #[kaa::action("<u>$x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x+x$</u>")]
    let huge = |clipped| clipped;

    #[kaa::action(r"$x\rule{1em}{13em}$")]
    let fraction = |huge| huge;

    #[kaa::action(r"$\dfrac{1}{2}$")]
    let tested = |fraction| fraction;

    #[kaa::question("Highlighted formulas on branch labels.")]
    #[kaa::yes("<mark>$x$</mark>")]
    #[kaa::no("<mark>**$x$**</mark>")]
    let (yes, no) = |tested| tested;

    #[kaa::action("Take the first formula.")]
    let done = |yes| {};

    #[kaa::action("Take the second formula.")]
    let done = |no| {};

    #[kaa::cycle("<mark>$x$</mark>")]
    let ready = |done| loop {
        #[kaa::action("Complete one pass.")]
        let ready = |done| {};
    };

    |ready| return;
}

#[test]
fn formulas_preserve_execution() {
    render_formulas(true);
    render_formulas(false);
}
