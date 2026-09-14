use kaalang::kaalang;

#[kaalang]
fn invalid(text: String, done: bool) -> String {
    #[cycle("Move persistent state on a repeating route.")]
    let result = |text, done| {
        #[question("Finish?")]
        let (finish, again) = |done| done;

        |finish, text| break text;

        #[action("Consume the persistent value.")]
        |again, text| drop(text);
    };

    |result| return result;
}

fn main() {}
