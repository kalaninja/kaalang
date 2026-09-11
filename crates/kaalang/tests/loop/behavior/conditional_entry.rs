use kaalang::kaalang;

#[kaalang]
fn conditional_entry(enabled: bool, limit: usize) -> usize {
    #[question("Run the counter?")]
    let (run, skip) = |enabled| enabled;

    #[action("Initialize the selected counter.")]
    let mut count = |run| 0;

    |&count| loop {
        #[question("Has the counter reached the limit?")]
        let (done, again) = |&count, limit| *count >= limit;

        |done| break;

        #[action("Increment the counter.")]
        |again, &mut count| *count += 1;
    };

    #[action("Return the counter.")]
    let end = |count| count;

    #[action("Skip the counter.")]
    let end = |skip| 99;
}

#[test]
fn a_data_capture_attaches_the_whole_loop_to_its_branch() {
    assert_eq!(conditional_entry(false, 3), 99);
    assert_eq!(conditional_entry(true, 0), 0);
    assert_eq!(conditional_entry(true, 3), 3);
}
