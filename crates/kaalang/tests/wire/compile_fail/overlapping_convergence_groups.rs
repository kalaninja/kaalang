use kaalang::kaalang;

#[kaalang]
fn invalid(value: u8) {
    #[choice("Which cases share what?")]
    #[case("Shares the left step.")]
    #[case("Shares both steps.")]
    #[case("Shares the right step.")]
    |value| -> (a, b, c) {
        match value {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the left value from a.")]
    |a| -> left { 1 };

    #[action("Build both values from b.")]
    |b| -> (left, right) { (2, 3) };

    #[action("Build the right value from c.")]
    |c| -> right { 4 };

    #[action("Take the left step.")]
    |left| -> () { drop(left) };

    #[action("Take the right step.")]
    |right| -> () { drop(right) };

    #[end]
    || {};
}

fn main() {}
