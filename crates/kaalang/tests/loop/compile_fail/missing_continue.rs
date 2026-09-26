use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Forget to repeat.")]
    {
        #[question("Leave now?")]
        let (leave, again) = |flag| flag;

        |leave| break;

        #[action("Do the repeating work.")]
        |again| {};
    };

    return;
}

fn main() {}
