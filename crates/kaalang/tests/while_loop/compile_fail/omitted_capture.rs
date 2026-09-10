use kaalang::kaalang;

#[kaalang]
fn invalid(mut count: usize) {
    #[question("Repeat?")]
    while (|count| count > 0) {
        #[action("Change the counter without capturing it.")]
        || { count -= 1; };
    }
    #[action("Finish.")]
    let result = || {};
}

fn main() {}
