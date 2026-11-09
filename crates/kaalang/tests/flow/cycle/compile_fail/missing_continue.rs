use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) {
    #[kaa::cycle("Forget to repeat.")]
    let leave = loop {
        #[kaa::question("Leave now?")]
        let (leave, again) = |flag| flag;

        #[kaa::action("Do the repeating work.")]
        |again| {};
    };

    |leave| return;
}

fn main() {}
