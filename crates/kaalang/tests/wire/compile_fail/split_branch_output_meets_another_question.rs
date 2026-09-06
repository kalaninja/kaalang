use kaalang::kaalang;

// Splitting the first selected output through an action delivers that output,
// but not the second one: `b` is still selected in executions where the block
// capturing it never runs.
#[kaalang]
fn invalid(first: bool, second: bool, third: bool) {
    #[question("Enable the shared input.")]
    |first| -> (a, _first_no) { first };

    #[question("Enable the second input.")]
    |second| -> (b, _second_no) { second };

    #[question("Enable the third input.")]
    |third| -> (c, _third_no) { third };

    #[action("Split the shared input.")]
    |a| -> (a_with_second, a_with_third) { ((), ()) };

    #[action("Require the first and second inputs.")]
    |a_with_second, b| -> () {};

    #[action("Require the first and third inputs.")]
    |a_with_third, c| -> () {};

    #[end]
    || {};
}

fn main() {}
