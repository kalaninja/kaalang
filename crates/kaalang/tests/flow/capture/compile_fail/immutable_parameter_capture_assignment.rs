use kaalang::kaa;

#[kaa]
fn immutable_parameter_capture_assignment(value: u32) -> u32 {
    #[kaa::action("Assign through an immutable capture.")]
    let result = |value| {
        value += 8;
        value
    };

    |result| return result;
}

fn main() {}
