use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) -> usize {
    #[kaa::question("Prepare a value?")]
    let (prepare, skip) = |flag| flag;

    #[kaa::action("Prepare the value.")]
    let value = |prepare| 1;

    #[kaa::action("Skip the preparation.")]
    let ready = |skip| ();

    #[kaa::action("Finish the preparation.")]
    let ready = |value| ();

    #[kaa::cycle("Read a value only one route prepared.")]
    let result = |ready| loop {
        #[kaa::action("Copy the value.")]
        let result = |value| value;
    };

    |result| return result;
}

fn main() {}
