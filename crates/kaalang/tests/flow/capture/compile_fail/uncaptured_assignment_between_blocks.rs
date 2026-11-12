use kaalang::kaa;

#[kaa]
// The underscore permits no consumer, but never permits an uncaptured use.
fn uncaptured_assignment_between_blocks(_value: u32) -> u32 {
    #[kaa::action("Assign to an uncaptured input.")]
    { _value = 9; };

    #[kaa::action("Read the uncaptured input in another body.")]
    let result = || _value;

    |result| return result;
}

fn main() {}
