use kaalang::kaalang;

#[kaalang]
fn immutable_parameter_capture_assignment(value: u32) -> u32 {
    #[action("Assign through an immutable capture.")]
    let result = |value| {
        value += 8;
        value
    };

    |result| return result;
}

fn main() {}
