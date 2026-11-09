use kaalang::kaa;

// `let ()` declares no wires, and a choice needs one per case.
#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::choice("Pick without outputs.")]
    #[kaa::case("First.")]
    #[kaa::case("Second.")]
    let () = |&input| {
        match input {
            0 => (),
            _ => (),
        }
    };

    |input| return input;
}

fn main() {}
