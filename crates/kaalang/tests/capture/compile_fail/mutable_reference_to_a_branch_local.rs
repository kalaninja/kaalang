use kaalang::kaalang;

#[kaalang]
fn mutable_reference_to_a_branch_local(condition: bool, mut fallback: String) -> usize {
    #[question("Create a local owner?")]
    |condition| -> (yes, no) { condition };

    #[action("Create the branch-local owner.")]
    |yes| -> local { String::from("local") };

    #[action("Try to carry its mutable reference past the merge.")]
    |&mut local| -> view { local };

    #[action("Borrow an owner that outlives the merge.")]
    |no, &mut fallback| -> view { fallback };

    #[action("Use the merged reference.")]
    |view| -> result { view.len() };
}

fn main() {}
