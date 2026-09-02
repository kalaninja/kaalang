use kaalang::kaalang;

#[kaalang]
fn question_value(condition: bool, shared_runs: &mut usize) -> u32 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes value.")]
    |yes| -> selected { 11 };

    #[action("Build the no value.")]
    |no| -> selected { 29 };

    #[action("Use the selected value once.")]
    |selected, shared_runs| -> result {
        *shared_runs += 1;
        selected
    };

    #[end]
    |result| {};
}

#[test]
fn question_converges_two_producers_into_one_consumer() {
    let mut shared_runs = 0;
    assert_eq!(question_value(true, &mut shared_runs), 11);
    assert_eq!(shared_runs, 1);
    assert_eq!(question_value(false, &mut shared_runs), 29);
    assert_eq!(shared_runs, 2);
}
