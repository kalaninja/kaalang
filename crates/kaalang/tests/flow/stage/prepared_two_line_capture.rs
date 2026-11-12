use kaalang::kaa;

#[kaa]
fn prepared_two_line_capture(
    first_input_value: u32,
    second_input_value: u32,
    third_input_value: u32,
    fourth_input_value: u32,
) -> u32 {
    #[kaa::action("Add the values before entering the stage.")]
    let finish = |first_input_value, second_input_value, third_input_value, fourth_input_value| {
        first_input_value + second_input_value + third_input_value + fourth_input_value
    };

    #[kaa::stage("Return the sum.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn a_wrapped_preparation_capture_clears_the_stage_rail() {
    assert_eq!(prepared_two_line_capture(1, 2, 3, 4), 10);
}
