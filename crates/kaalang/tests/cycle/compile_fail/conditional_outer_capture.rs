use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    #[question("Prepare a value?")]
    let (prepare, skip) = |flag| flag;

    #[action("Prepare the value.")]
    let value = |prepare| 1;

    #[action("Skip the preparation.")]
    let ready = |skip| ();

    #[action("Finish the preparation.")]
    let ready = |value| ();

    #[cycle("Read a value only one route prepared.")]
    let result = |ready| loop {
        #[action("Copy the value.")]
        let result = |value| value;
    };

    |result| return result;
}

fn main() {}
