use kaalang::kaalang;

#[kaalang]
fn invalid(text: String, done: bool) -> String {
    #[cycle("Move outer data on a repeating route.")]
    let finish = loop {
        #[question("Finish?")]
        let (finish, again) = |done| done;

        #[action("Consume the outer value.")]
        |again, text| drop(text);

        |again| continue;
    };

    |finish, text| return text;
}

fn main() {}
