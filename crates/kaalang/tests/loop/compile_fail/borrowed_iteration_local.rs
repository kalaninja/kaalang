use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> &'static str {
    #[cycle("Try to return a borrowed local.")]
    let result = |flag| {
        #[question("Transfer a local value?")]
        let (done, _again) = |flag| flag;

        #[action("Create the local owner.")]
        let text = |done| String::from("local");

        #[action("Borrow the local owner.")]
        let borrowed = |&text| text.as_str();

        |borrowed| break borrowed;
    };

    |result| return result;
}

fn main() {}
