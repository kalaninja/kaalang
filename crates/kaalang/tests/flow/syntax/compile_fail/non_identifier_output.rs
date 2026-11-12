use kaalang::kaa;

#[kaa]
fn non_identifier_output(value: i32) -> Vec<i32> {
    #[kaa::action("Wrap the value.")]
    let Some(output) = |value| { Some(value) };
}

fn main() {}
