use kaalang::kaalang;

#[kaalang]
fn mutable_reference_to_a_branch_local(condition: bool, mut fallback: String) -> usize {
    #[question("Create a local owner?")]
    let (yes, no) = |condition| { condition };

    #[action("Create the branch-local owner.")]
    let mut local = |yes| { String::from("local") };

    #[action("Try to carry its mutable reference past the merge.")]
    let view = |&mut local| { local };

    #[action("Borrow an owner that outlives the merge.")]
    let view = |no, &mut fallback| { fallback };

    #[action("Use the merged reference.")]
    let end = |view| { view.len() };
}

fn main() {}
