use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Forget to repeat.")]
    let leave = {
        #[question("Leave now?")]
        let (leave, again) = |flag| flag;

        #[action("Do the repeating work.")]
        |again| {};
    };

    |leave| return;
}

fn main() {}
