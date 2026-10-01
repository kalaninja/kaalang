use kaalang::kaalang;

#[kaalang]
fn render_large_formulas(value: u32) -> u32 {
    #[action(
        r"$\begin{matrix}1 & 2 \\ 2 & 4 \\ 3 & 6 \\ 4 & 8 \\ 5 & 10 \\ 6 & 12 \\ 7 & 14 \\ 8 & 16 \\ 9 & 18 \\ 10 & 20 \\ 11 & 22 \\ 12 & 24 \\ 13 & 26 \\ 14 & 28\end{matrix}$"
    )]
    let rendered = |value| value;

    #[action(
        r"$\begin{matrix}1 & 2 \\ 2 & 4 \\ 3 & 6 \\ 4 & 8 \\ 5 & 10 \\ 6 & 12 \\ 7 & 14 \\ 8 & 16 \\ 9 & 18 \\ 10 & 20 \\ 11 & 22 \\ 12 & 24 \\ 13 & 26 \\ 14 & 28 \\ 15 & 30\end{matrix}$"
    )]
    let literal = |rendered| rendered;

    |literal| return literal;
}

#[test]
fn large_formulas_preserve_execution() {
    assert_eq!(render_large_formulas(7), 7);
}
