use kaalang::kaalang;

#[kaalang]
const fn shadowed_labels(mut count: usize) -> usize {
    'r#pass: loop {
        'pass: loop {
            break 'r#pass;
        }

        #[action("Advance the outer pass.")]
        let advanced = |&mut count| *count += 1;

        #[question("Have two passes finished?")]
        let (done, again) = |advanced, &count| *count >= 2;

        |done| break 'pass;

        #[action("Finish this pass.")]
        |again| {};
    }

    #[action("Return the count.")]
    let end = |count| count;
}

#[test]
fn a_label_names_the_nearest_enclosing_loop_with_that_name() {
    const TWO: usize = shadowed_labels(0);
    assert_eq!(TWO, 2);
}
