use contour::contour;

#[contour]
fn invalid(condition: bool) -> bool {
    #[question("Declare one name for both branches.")]
    |condition| -> (branch, branch) { condition };
}

fn main() {}
