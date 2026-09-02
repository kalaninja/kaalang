use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> bool {
    #[question("Declare one name for both branches.")]
    |condition| -> (branch, branch) { condition };

    #[end]
    |branch| {};
}

fn main() {}
