use kaalang::kaa;

#[kaa]
fn mutable_borrow_of_immutable_outer_wire() {
    #[kaa::action("Outer.")]
    let shared = || 1;
    #[kaa::action("Go.")]
    let go = || ();
    #[kaa::stage("Go.")]
    |go| {
        |&mut shared| return;
    };
}

fn main() {}
