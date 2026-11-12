use kaalang::kaa;

// One route forgets to repeat, while another enters an inner cycle that reads
// a wire only a third route produced. The error inside the body comes first.
#[kaa]
fn invalid(flag: bool, other: bool) -> u32 {
    #[kaa::cycle("Run the outer cycle.")]
    let read = loop {
        #[kaa::question("Take the first route?")]
        let (first, rest) = |flag| flag;

        #[kaa::question("Take the third route?")]
        let (third, _fourth) = |rest, other| other;

        #[kaa::action("Produce a value only on the third route.")]
        let value = |third| 1;

        #[kaa::cycle("Read the value on every other route.")]
        let read = |rest| loop {
            #[kaa::action("Read the value.")]
            let read = |&value| *value;
        };

        #[kaa::action("Forget to repeat.")]
        |first| {};
    };

    |read| return read;
}

fn main() {}
