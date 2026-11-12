use kaalang::kaa;

#[kaa]
fn invalid() {
    #[kaa::cycle("Use the result before completion.")]
    let result = loop {
        #[kaa::action("Read the unavailable result.")]
        |result| {};

        #[kaa::action("Produce the result.")]
        let result = || ();
    };

    |result| return result;
}

fn main() {}
