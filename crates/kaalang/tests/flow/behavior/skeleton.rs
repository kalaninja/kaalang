use kaalang::kaa;

#[allow(unreachable_code)]
#[kaa]
fn skeleton(seed: u32) -> u32 {
    #[kaa::question("Accept the seed?")]
    let (accepted, rejected) = |&seed| todo!();

    #[kaa::action("Keep the accepted seed.")]
    let end = |accepted, seed| todo!();

    #[kaa::action("Keep the rejected seed.")]
    let end = |rejected, seed| todo!();

    |end| return end;
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
