use kaalang::kaalang;

fn two() -> u32 {
    2
}

#[kaalang]
fn unbraced_output_without_captures() -> u32 {
    #[action("Start from forty.")]
    let start = 40;

    #[action("Build two.")]
    let step = two();

    #[action("Add them.")]
    let sum = |start, step| start + step;

    |sum| return sum;
}

#[test]
fn an_unbraced_body_without_captures_binds_its_value() {
    assert_eq!(unbraced_output_without_captures(), 42);
}
