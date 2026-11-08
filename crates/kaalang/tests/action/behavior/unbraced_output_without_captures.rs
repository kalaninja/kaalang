use kaalang::kaa;

fn two() -> u32 {
    2
}

#[kaa]
fn unbraced_output_without_captures() -> u32 {
    #[kaa::action("Start from forty.")]
    let start = 40;

    #[kaa::action("Build two.")]
    let step = two();

    #[kaa::action("Add them.")]
    let sum = |start, step| start + step;

    |sum| return sum;
}

#[test]
fn an_unbraced_body_without_captures_binds_its_value() {
    assert_eq!(unbraced_output_without_captures(), 42);
}
