use kaalang::kaalang;

#[kaalang]
fn conditional_entry(enabled: bool, limit: usize) -> usize {
    #[question("Run the counter?")]
    let (run, skip) = |enabled| enabled;

    #[action("Initialize the selected counter.")]
    let mut count = |run| 0;

    #[cycle("Count to the limit when selected.")]
    let end = |mut count, limit| {
        #[question("Has the counter reached the limit?")]
        let (done, again) = |&count, &limit| *count >= *limit;

        |done, count| break count;

        #[action("Increment the counter.")]
        |again, &mut count| *count += 1;
    };

    #[action("Skip the counter.")]
    let end = |skip| 99;

    |end| return end;
}

#[test]
fn a_data_capture_attaches_the_whole_cycle_to_its_branch() {
    assert_eq!(conditional_entry(false, 3), 99);
    assert_eq!(conditional_entry(true, 0), 0);
    assert_eq!(conditional_entry(true, 3), 3);
}
