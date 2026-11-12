use kaalang::kaa;

#[kaa]
fn preparation_wire_missing_on_a_route(choice: bool) {
    #[kaa::question("Choose.")]
    let (yes, no) = |choice| choice;
    #[kaa::action("Left data.")]
    let shared = |yes| 1;
    #[kaa::action("Left signal.")]
    let go = |yes| ();
    #[kaa::action("Right signal.")]
    let go = |no| ();
    #[kaa::stage("Go.")]
    |go| {
        |shared| return;
    };
}

fn main() {}
