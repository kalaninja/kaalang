use kaalang::kaa;

#[kaa]
fn classify(seed: u32, limit: u32) -> u32 {
    #[kaa::action("Normalize the seed.")]
    let normalized = |&seed| *seed;

    #[kaa::action("Split the seed.")]
    let (left, right) = |normalized| (normalized, normalized + 1);

    #[kaa::action("Add the values.")]
    let total = |left, right, &limit| {
        let subtotal = left + right;
        subtotal + *limit
    };

    #[kaa::question("Is the total above the limit?")]
    let (accepted, rejected) = |&total, &limit| *total > *limit;

    #[kaa::action("Keep the accepted total.")]
    let end = |accepted, total| total;

    #[kaa::action("Keep the rejected total.")]
    let end = |rejected, total| total;

    |end| return end;
}

#[test]
fn flow_is_an_ordinary_rust_function() {
    fn accepts_flow(_: fn(u32, u32) -> u32) {}

    accepts_flow(classify);
}

#[test]
fn generated_flow_executes() {
    assert_eq!(classify(1, 2), 5);
}
