use kaalang::kaa;

#[kaa]
fn prepared_initial_selection(route: u8) -> (usize, usize) {
    #[kaa::action("Keep shared temporary data alive across either initial entry.")]
    let view = || &String::from("prepared");

    #[kaa::choice("Choose an initial stage or one of two finishing values.")]
    #[kaa::case("Visit the first stage.")]
    #[kaa::case("Finish with the first value.")]
    #[kaa::case("Finish with the second value.")]
    let (first, left, right) = |route| match route {
        0 => (),
        1 => (),
        _ => (),
    };

    #[kaa::action("Provide the first finishing value.")]
    let finish = |left| 1usize;

    #[kaa::action("Provide the second finishing value.")]
    let finish = |right| 2usize;

    #[kaa::stage("Measure shared data on the first route.")]
    let finish = |first| {
        #[kaa::action("Provide the measured length.")]
        let finish = |view| view.len();
    };

    #[kaa::stage("Return the selected value and shared length.")]
    |finish| {
        #[kaa::action("Measure the shared data after either initial entry.")]
        let result = |finish, view| (finish, view.len());

        |result| return result;
    };
}

#[test]
fn a_partial_preparation_join_keeps_one_dispatcher_after_initial_selection() {
    assert_eq!(prepared_initial_selection(0), (8, 8));
    assert_eq!(prepared_initial_selection(1), (1, 8));
    assert_eq!(prepared_initial_selection(2), (2, 8));
}
