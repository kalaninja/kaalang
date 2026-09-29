use kaalang::kaalang;

#[kaalang]
fn sum_inputs(
    first_input_value: u32,
    second_input_value: u32,
    third_input_value: u32,
    fourth_input_value: u32,
) -> u32 {
    #[action("Begin.")]
    let go = || {};

    #[stage("Return the sum.")]
    |go| {
        #[action("Add the inputs.")]
        let sum = |first_input_value, second_input_value, third_input_value, fourth_input_value| {
            first_input_value + second_input_value + third_input_value + fourth_input_value
        };

        |sum| return sum;
    };
}

#[test]
fn a_stage_reads_several_prepared_inputs() {
    assert_eq!(sum_inputs(1, 2, 3, 4), 10);
}
