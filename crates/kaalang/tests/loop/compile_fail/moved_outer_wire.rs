use kaalang::kaalang;

#[kaalang]
fn invalid(text: String) {
    |&text| loop {
        #[question("Repeat?")]
        let (_iterate_1, leave_1) = |text| !text.is_empty();
        |leave_1| break;
    };
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
