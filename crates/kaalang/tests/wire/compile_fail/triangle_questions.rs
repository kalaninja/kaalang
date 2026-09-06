use kaalang::kaalang;

// Splitting each selected output through an action does not make the pairwise
// meets legal: every pair is still decided by two independent questions.
#[kaalang]
fn invalid(first: bool, second: bool, third: bool) {
    #[question("Enable the first input.")]
    |first| -> (a, _first_no) { first };

    #[question("Enable the second input.")]
    |second| -> (b, _second_no) { second };

    #[question("Enable the third input.")]
    |third| -> (c, _third_no) { third };

    #[action("Split the first input for its two pairs.")]
    |a| -> (a_with_b, a_with_c) { ((), ()) };

    #[action("Split the second input for its two pairs.")]
    |b| -> (b_with_a, b_with_c) { ((), ()) };

    #[action("Split the third input for its two pairs.")]
    |c| -> (c_with_a, c_with_b) { ((), ()) };

    #[action("Use the first and second inputs.")]
    |a_with_b, b_with_a| -> () {};

    #[action("Use the second and third inputs.")]
    |b_with_c, c_with_b| -> () {};

    #[action("Use the first and third inputs.")]
    |a_with_c, c_with_a| -> () {};

    #[end]
    || {};
}

fn main() {}
