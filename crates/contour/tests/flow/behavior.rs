use contour::contour;

#[contour]
fn classify(seed: u32, limit: u32) -> u32 {
    #[action("Normalize the seed.")]
    |&seed| -> (normalized,) { (*seed,) };

    #[action("Split the seed.")]
    |normalized| -> (left, right) { (normalized, normalized + 1) };

    #[action("Add the values.")]
    |left, right, &limit| -> total {
        let subtotal = left + right;
        subtotal + *limit
    };

    #[question("Is the total above the limit?")]
    |&total, &limit| -> (accepted, rejected) { *total > *limit };

    #[action("Keep the accepted total.")]
    |accepted, total| -> accepted_total { total };

    #[action("Keep the rejected total.")]
    |rejected, total| -> rejected_total { total };
}

#[contour]
fn skeleton(seed: u32) -> u32 {
    #[question("Accept the seed?")]
    |&seed| -> (accepted, rejected) { todo!() };

    #[action("Keep the accepted seed.")]
    |accepted, seed| -> accepted_seed { todo!() };

    #[action("Keep the rejected seed.")]
    |rejected, seed| -> rejected_seed { todo!() };
}

#[test]
fn flow_is_an_ordinary_rust_function() {
    fn accepts_flow(_: fn(u32, u32) -> u32) {}
    fn accepts_skeleton(_: fn(u32) -> u32) {}

    accepts_flow(classify);
    accepts_skeleton(skeleton);
}

#[test]
fn generated_flow_executes() {
    assert_eq!(classify(1, 2), 5);
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn author_skeleton_fails_only_when_executed() {
    skeleton(1);
}
