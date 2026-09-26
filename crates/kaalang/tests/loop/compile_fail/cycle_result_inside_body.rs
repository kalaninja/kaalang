use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Use the result before completion.")]
    let result = {
        #[action("Read the unavailable result.")]
        |result| {};

        #[action("Produce the result.")]
        let result = || ();
    };

    |result| return result;
}

fn main() {}
