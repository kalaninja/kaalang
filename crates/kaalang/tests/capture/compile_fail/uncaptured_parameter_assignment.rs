use kaalang::kaalang;

#[kaalang]
fn uncaptured_parameter_assignment(value: u32) -> u32 {
    #[action("Assign without a capture.")]
    let result = || {
        value = 9;
        value
    };

    |result| return result;
}

fn main() {}
