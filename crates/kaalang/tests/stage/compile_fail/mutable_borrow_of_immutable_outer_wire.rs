use kaalang::kaalang;

#[kaalang]
fn mutable_borrow_of_immutable_outer_wire() {
    #[action("Outer.")]
    let shared = || 1;
    #[action("Go.")]
    let go = || ();
    #[stage("Go.")]
    |go| {
        |&mut shared| return;
    };
}

fn main() {}
