use kaalang::kaa;

#[kaa]
fn invalid(input: u32) {
    #[kaa::question("Is the input large?")]
    let (large, small) = |&input| { *input > 10 };

    #[kaa::action("Handle only the large input.")]
    let handled = |large, input| { drop(input) };

    |handled| return handled;
}

fn main() {}
