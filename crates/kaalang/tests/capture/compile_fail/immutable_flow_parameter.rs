use kaalang::kaalang;

#[kaalang]
fn immutable_flow_parameter(value: u8) -> u8 {
    #[action("Try to mutate an immutable flow parameter.")]
    |&mut value| -> result {
        *value += 1;
        *value
    };
}

fn main() {}
