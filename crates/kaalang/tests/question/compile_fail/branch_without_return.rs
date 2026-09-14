use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) {
    #[question("Is the input large?")]
    let (large, small) = |&input| { *input > 10 };

    #[action("Handle only the large input.")]
    let handled = |large, input| { drop(input) };

    |handled| return handled;
}

fn main() {}
