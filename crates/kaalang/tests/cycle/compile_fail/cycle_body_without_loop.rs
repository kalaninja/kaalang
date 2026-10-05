use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Repeat without a loop body.")]
    || continue;

    return;
}

fn main() {}
