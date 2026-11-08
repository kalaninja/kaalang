use kaalang::kaa;

#[kaa]
fn invalid() {
    #[kaa::cycle("Repeat without a loop body.")]
    || continue;

    return;
}

fn main() {}
