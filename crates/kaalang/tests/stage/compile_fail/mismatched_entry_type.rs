use kaalang::kaalang;

#[kaalang]
fn mismatched_entry_type(go: u32) -> u32 {
    #[stage("Try again or finish.")]
    let (go, done) = |go| {
        #[question("Is more work needed?")]
        let (again, finish) = |go| go > 0;

        #[action("Send the wrong type.")]
        let go = |again| String::from("wrong");

        #[action("Finish.")]
        let done = |finish| 0u32;
    };

    #[stage("Return the result.")]
    |done| {
        |done| return done;
    };
}

fn main() {}
