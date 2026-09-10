use kaalang::kaalang;

#[kaalang]
fn question_value(condition: bool, shared_runs: &mut usize) -> u32 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| condition;

    #[action("Build the yes value.")]
    let selected = |yes| 11;

    #[action("Build the no value.")]
    let selected = |no| 29;

    #[action("Use the selected value once.")]
    let end = |selected, shared_runs| {
        *shared_runs += 1;
        selected
    };
}

#[test]
fn question_converges_two_producers_into_one_consumer() {
    let mut shared_runs = 0;
    assert_eq!(question_value(true, &mut shared_runs), 11);
    assert_eq!(shared_runs, 1);
    assert_eq!(question_value(false, &mut shared_runs), 29);
    assert_eq!(shared_runs, 2);
}
