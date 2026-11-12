use kaalang::kaa;

#[kaa]
fn invalid(text: String, done: bool) -> String {
    #[kaa::cycle("Move outer data on a repeating route.")]
    let finish = loop {
        #[kaa::question("Finish?")]
        let (finish, again) = |done| done;

        #[kaa::action("Consume the outer value.")]
        |again, text| drop(text);

        |again| continue;
    };

    |finish, text| return text;
}

fn main() {}
