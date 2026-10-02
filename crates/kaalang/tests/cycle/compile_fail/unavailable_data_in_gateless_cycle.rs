use kaalang::kaalang;

#[kaalang]
fn unavailable_data_in_gateless_cycle(flag: bool) -> u8 {
    #[question("Prepare a value?")]
    let (prepare, skip) = |flag| flag;

    #[action("Prepare the value.")]
    let value = |prepare| 1u8;

    #[action("Skip the value.")]
    let ready = |skip| ();

    #[action("Finish preparation.")]
    let ready = |value| ();

    #[cycle("Try to read the optional value.")]
    let result = {
        #[action("Copy the value.")]
        let result = |value| value;
    };

    |ready, result| return result;
}

fn main() {}
