use kaalang::kaalang;

#[kaalang]
fn invalid(text: String, done: bool) -> String {
    #[cycle("Move outer data on a repeating route.")]
    let result = || {
        #[question("Finish?")]
        let (finish, again) = |done| done;

        |finish, text| break text;

        #[action("Consume the outer value.")]
        |again, text| drop(text);
    };

    |result| return result;
}

fn main() {}
