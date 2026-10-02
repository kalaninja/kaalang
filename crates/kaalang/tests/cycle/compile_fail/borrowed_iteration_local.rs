use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> &'static str {
    #[cycle("Try to return a borrowed local.")]
    let borrowed = |flag| {
        #[question("Transfer a local value?")]
        let (done, again) = |flag| flag;

        #[action("Create the local owner.")]
        let text = |done| String::from("local");

        #[action("Borrow the local owner.")]
        let borrowed = |&text| text.as_str();

        |again| continue;
    };

    |borrowed| return borrowed;
}

fn main() {}
