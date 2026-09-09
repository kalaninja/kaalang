use kaalang::kaalang;

#[kaalang]
fn non_identifier_output(value: i32) -> Vec<i32> {
    #[action("Wrap the value.")]
    let Some(output) = |value| { Some(value) };
}

fn main() {}
