use kaalang::kaalang;

#[kaalang]
fn classify(seed: u32, limit: u32) -> u32 {
    #[action("Normalize the seed.")]
    let normalized = |&seed| *seed;

    #[action("Split the seed.")]
    let (left, right) = |normalized| (normalized, normalized + 1);

    #[action("Add the values.")]
    let total = |left, right, &limit| {
        let subtotal = left + right;
        subtotal + *limit
    };

    #[question("Is the total above the limit?")]
    let (accepted, rejected) = |&total, &limit| *total > *limit;

    #[action("Keep the accepted total.")]
    let end = |accepted, total| total;

    #[action("Keep the rejected total.")]
    let end = |rejected, total| total;
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
