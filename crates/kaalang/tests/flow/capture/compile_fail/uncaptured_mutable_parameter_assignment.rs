use kaalang::kaa;

#[kaa]
fn uncaptured_mutable_parameter_assignment(mut value: u32) -> u32 {
    #[kaa::action("Assign without a capture to a mutable parameter.")]
    let result = || {
        value = 9;
        value
    };

    |result| return result;
}

fn main() {}
