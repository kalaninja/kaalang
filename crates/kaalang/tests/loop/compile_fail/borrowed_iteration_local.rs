use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> &'static str {
    |&flag| loop {
        #[question("Return a local value?")]
        let (iterate_1, leave_1) = |flag| flag;
        |leave_1| break;
        #[action("Create the local owner.")]
        let text = |iterate_1| String::from("local");
        #[action("Return a reference to the local owner.")]
        let end = |&text| text.as_str();
    };
    #[action("Return a static value.")]
    let end = || "static";
}

fn main() {}
