use kaalang::kaa;

#[kaa]
fn mutable_reference_to_a_branch_local(condition: bool, mut fallback: String) -> usize {
    #[kaa::question("Create a local owner?")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Create the branch-local owner.")]
    let mut local = |yes| { String::from("local") };

    #[kaa::action("Try to carry its mutable reference past the merge.")]
    let view = |&mut local| { local };

    #[kaa::action("Borrow an owner that outlives the merge.")]
    let view = |no, &mut fallback| { fallback };

    #[kaa::action("Use the merged reference.")]
    let end = |view| { view.len() };

    |end| return end;
}

fn main() {}
