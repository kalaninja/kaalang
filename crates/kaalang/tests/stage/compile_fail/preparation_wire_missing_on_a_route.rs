use kaalang::kaalang;

#[kaalang]
fn preparation_wire_missing_on_a_route(choice: bool) {
    #[question("Choose.")]
    let (yes, no) = |choice| choice;
    #[action("Left data.")]
    let shared = |yes| 1;
    #[action("Left signal.")]
    let go = |yes| ();
    #[action("Right signal.")]
    let go = |no| ();
    #[stage("Go.")]
    |go| {
        |shared| return;
    };
}

fn main() {}
