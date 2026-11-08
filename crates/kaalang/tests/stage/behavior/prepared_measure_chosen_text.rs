use kaalang::kaa;

#[kaa]
fn prepared_measure_chosen_text() -> usize {
    #[kaa::action("Begin by choosing a greeting.")]
    let choose = || {};

    #[kaa::stage("Measure the chosen text.")]
    let finish = |text| {
        #[kaa::action("Count the bytes of the text.")]
        let finish = |&text| text.len();
    };

    #[kaa::stage("Choose the text.")]
    let text = |choose| {
        #[kaa::action("Choose a greeting.")]
        let text = || String::from("hello");
    };

    #[kaa::stage("Return the length.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn preparation_can_enter_a_later_stage_that_sends_data_to_an_earlier_one() {
    assert_eq!(prepared_measure_chosen_text(), 5);
}
