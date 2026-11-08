use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) -> &'static str {
    #[kaa::cycle("Try to return a borrowed local.")]
    let borrowed = |flag| loop {
        #[kaa::question("Transfer a local value?")]
        let (done, again) = |flag| flag;

        #[kaa::action("Create the local owner.")]
        let text = |done| String::from("local");

        #[kaa::action("Borrow the local owner.")]
        let borrowed = |&text| text.as_str();

        |again| continue;
    };

    |borrowed| return borrowed;
}

fn main() {}
