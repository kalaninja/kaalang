use kaalang::kaalang;

#[kaalang]
fn invalid(left: bool, right: bool) -> u8 {
    #[question("Choose the left branch.")]
    |left| -> (a, b) { left };

    #[question("Choose the right branch.")]
    |right| -> (c, d) { right };

    #[action("Combine both yes branches.")]
    |a, c| -> result { 0 };

    #[action("Combine yes and no.")]
    |a, d| -> result { 1 };

    #[action("Combine no and yes.")]
    |b, c| -> result { 2 };

    #[action("Combine both no branches.")]
    |b, d| -> result { 3 };

    #[end]
    |result| {};
}

fn main() {}
