use kaalang::kaalang;

#[kaalang]
fn blocks_without_shared_wires(left: u32, right: u32) -> (u32, u32) {
    #[action("Double the left value.")]
    let doubled = |left| left * 2;

    #[action("Triple the right value.")]
    let tripled = |right| right * 3;

    #[action("Pair the two values.")]
    let end = |doubled, tripled| (doubled, tripled);
}

#[test]
fn blocks_sharing_no_wire_run_in_source_order() {
    assert_eq!(blocks_without_shared_wires(1, 2), (2, 6));
}
