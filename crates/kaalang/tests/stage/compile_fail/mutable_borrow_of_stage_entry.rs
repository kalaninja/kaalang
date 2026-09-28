use kaalang::kaalang;

#[kaalang]
fn mutable_borrow_of_stage_entry() {
    #[action("Go.")]
    let go = || ();
    #[stage("Go.")]
    |go| {
        |&mut go| return;
    };
}

fn main() {}
