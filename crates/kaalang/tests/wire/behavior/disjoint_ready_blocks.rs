use kaalang::kaalang;

#[kaalang]
fn disjoint_ready_blocks(left: u32, right: u32) -> (u32, u32) {
    #[action("Double the left value.")]
    |left| -> doubled { left * 2 };

    #[action("Triple the right value.")]
    |right| -> tripled { right * 3 };

    #[action("Pair the two values.")]
    |doubled, tripled| -> result { (doubled, tripled) };
}

#[test]
fn blocks_on_disjoint_wires_are_independent() {
    assert_eq!(disjoint_ready_blocks(1, 2), (2, 6));
}
