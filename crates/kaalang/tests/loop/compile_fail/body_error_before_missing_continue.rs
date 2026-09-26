use kaalang::kaalang;

// One route forgets to repeat, while another enters an inner cycle that reads
// a wire only a third route produced. The error inside the body comes first.
#[kaalang]
fn invalid(flag: bool, other: bool) -> u32 {
    #[cycle("Run the outer cycle.")]
    let result = {
        #[question("Take the first route?")]
        let (first, rest) = |flag| flag;

        #[question("Take the third route?")]
        let (third, _fourth) = |rest, other| other;

        #[action("Produce a value only on the third route.")]
        let value = |third| 1;

        #[cycle("Read the value on every other route.")]
        let read = |rest| {
            #[action("Read the value.")]
            let copy = |&value| *value;

            |copy| break copy;
        };

        #[action("Forget to repeat.")]
        |first| {};

        |read| break read;
    };

    |result| return result;
}

fn main() {}
