use kaalang::kaalang;

#[kaalang]
fn singleton_output(start: (u8, u8)) -> (u8, u8) {
    #[stage("Increment the first field.")]
    let (done,) = |start| {
        #[action("Keep the tuple intact.")]
        let done = |start| (start.0 + 1, start.1);
    };

    #[stage("Return the tuple.")]
    |done| {
        |done| return done;
    };
}

#[test]
fn a_singleton_stage_output_transfers_its_whole_tuple() {
    assert_eq!(singleton_output((2, 7)), (3, 7));
}
