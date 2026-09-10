use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> &'static str {
    #[question("Return a local value?")]
    while (|flag| flag) {
        #[action("Create the local owner.")]
        let text = || String::from("local");
        #[action("Return a reference to the local owner.")]
        let end = |&text| text.as_str();
    }
    #[action("Return a static value.")]
    let end = || "static";
}

fn main() {}
