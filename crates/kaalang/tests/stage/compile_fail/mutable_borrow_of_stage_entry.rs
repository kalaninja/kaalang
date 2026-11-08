use kaalang::kaa;

#[kaa]
fn mutable_borrow_of_stage_entry() {
    #[kaa::action("Go.")]
    let go = || ();
    #[kaa::stage("Go.")]
    |go| {
        |&mut go| return;
    };
}

fn main() {}
