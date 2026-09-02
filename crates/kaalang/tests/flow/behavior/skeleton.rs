use kaalang::kaalang;

#[allow(unreachable_code)]
#[kaalang]
fn skeleton(seed: u32) -> u32 {
    #[question("Accept the seed?")]
    |&seed| -> (accepted, rejected) { todo!() };

    #[action("Keep the accepted seed.")]
    |accepted, seed| -> result { todo!() };

    #[action("Keep the rejected seed.")]
    |rejected, seed| -> result { todo!() };

    #[end]
    |result| {};
}

#[test]
fn flow_is_an_ordinary_rust_function() {
    fn accepts_skeleton(_: fn(u32) -> u32) {}

    accepts_skeleton(skeleton);
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn author_skeleton_fails_only_when_executed() {
    skeleton(1);
}
