use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Use the result before completion.")]
    let result = || {
        #[action("Read the unavailable result.")]
        |result| {};

        break;
    };

    |result| return result;
}

fn main() {}
