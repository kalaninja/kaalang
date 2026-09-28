use kaalang::kaalang;

#[kaalang]
fn prepared_measure_chosen_text() -> usize {
    #[action("Begin by choosing a greeting.")]
    let choose = || {};

    #[stage("Measure the chosen text.")]
    let finish = |text| {
        #[action("Count the bytes of the text.")]
        let finish = |&text| text.len();
    };

    #[stage("Choose the text.")]
    let text = |choose| {
        #[action("Choose a greeting.")]
        let text = || String::from("hello");
    };

    #[stage("Return the length.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn preparation_can_enter_a_later_stage_that_sends_data_to_an_earlier_one() {
    assert_eq!(prepared_measure_chosen_text(), 5);
}
