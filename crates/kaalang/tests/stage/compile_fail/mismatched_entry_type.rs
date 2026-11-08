use kaalang::kaa;

#[kaa]
fn mismatched_entry_type(go: u32) -> u32 {
    #[kaa::stage("Try again or finish.")]
    let (go, done) = |go| {
        #[kaa::question("Is more work needed?")]
        let (again, finish) = |go| go > 0;

        #[kaa::action("Send the wrong type.")]
        let go = |again| String::from("wrong");

        #[kaa::action("Finish.")]
        let done = |finish| 0u32;
    };

    #[kaa::stage("Return the result.")]
    |done| {
        |done| return done;
    };
}

fn main() {}
