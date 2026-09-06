use kaalang::kaalang;

#[kaalang]
fn classify(seed: u32, limit: u32) -> u32 {
    #[action("Normalize the seed.")]
    |&seed| -> (normalized,) { *seed };

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
    |accepted, total| -> result { total };

    #[action("Keep the rejected total.")]
    |rejected, total| -> result { total };
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
