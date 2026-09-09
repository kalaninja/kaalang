use kaalang::kaalang;

// `-> ()` declares no wires, and a choice needs one per case.
#[kaalang]
fn invalid(input: u32) -> u32 {
    #[choice("Pick without outputs.")]
    #[case("First.")]
    #[case("Second.")]
    |&input| -> () {
        match input {
            0 => (),
            _ => (),
        }
    };

    #[action("Finish.")]
    |input| -> result { input };
}

fn main() {}
