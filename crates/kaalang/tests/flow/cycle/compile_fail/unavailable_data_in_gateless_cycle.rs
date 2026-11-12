use kaalang::kaa;

#[kaa]
fn unavailable_data_in_gateless_cycle(flag: bool) -> u8 {
    #[kaa::question("Prepare a value?")]
    let (prepare, skip) = |flag| flag;

    #[kaa::action("Prepare the value.")]
    let value = |prepare| 1u8;

    #[kaa::action("Skip the value.")]
    let ready = |skip| ();

    #[kaa::action("Finish preparation.")]
    let ready = |value| ();

    #[kaa::cycle("Try to read the optional value.")]
    let result = loop {
        #[kaa::action("Copy the value.")]
        let result = |value| value;
    };

    |ready, result| return result;
}

fn main() {}
