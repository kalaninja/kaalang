use kaalang::kaa;

// The block declares no outputs, so Rust checks its body against `()`.
#[kaa]
fn invalid(input: u32) {
    #[kaa::action("Declare no outputs.")]
    let () = |input| { input + 1 };

    #[kaa::action("Finish.")]
    let end = || {};

    |end| return end;
}

fn main() {}
