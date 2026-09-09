use kaalang::kaalang;

// `let ()` declares no wires, and a choice needs one per case.
#[kaalang]
fn invalid(input: u32) -> u32 {
    #[choice("Pick without outputs.")]
    #[case("First.")]
    #[case("Second.")]
    let () = |&input| {
        match input {
            0 => (),
            _ => (),
        }
    };

    #[action("Finish.")]
    let result = |input| { input };
}

fn main() {}
