use kaalang::kaa;

#[kaa]
fn internal_stage_parameter_names(
    first: u32,
    __kaalang_wire_0: u32,
    r#__kaalang_wire_0_: u32,
) -> (u32, u32, u32) {
    #[kaa::stage("Forward the first input.")]
    let next = |first| {
        #[kaa::action("Forward.")]
        let next = |first| first;
    };

    #[kaa::stage("Keep the inputs.")]
    |next| {
        #[kaa::action("Collect the values.")]
        let result = |next, __kaalang_wire_0, r#__kaalang_wire_0_| {
            (next, __kaalang_wire_0, r#__kaalang_wire_0_)
        };

        |result| return result;
    };
}

#[test]
fn staged_parameters_preserve_distinct_values_with_internal_spellings() {
    assert_eq!(internal_stage_parameter_names(1, 2, 3), (1, 2, 3));
}
