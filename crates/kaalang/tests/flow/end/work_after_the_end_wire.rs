use kaalang::kaa;

#[kaa]
fn work_after_the_end_wire(input: u32, other: u32) -> u32 {
    #[kaa::action("Produce an ordinary `end` wire.")]
    let end = |input| input;

    #[kaa::action("Do later work.")]
    let later = |other| other;

    #[kaa::action("Combine both values.")]
    let result = |end, later| end + later;

    |result| return result;
}

#[test]
fn producing_end_does_not_finish_the_flow() {
    assert_eq!(work_after_the_end_wire(3, 4), 7);
}
