use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> bool {
    #[question("Declare one name for both branches.")]
    |condition| -> (result, result) { condition };
}

fn main() {}
