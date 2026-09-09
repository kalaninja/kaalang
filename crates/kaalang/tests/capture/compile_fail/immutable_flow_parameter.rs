use kaalang::kaalang;

#[kaalang]
fn immutable_flow_parameter(value: u8) -> u8 {
    #[action("Try to mutate an immutable flow parameter.")]
    let result = |&mut value| {
        *value += 1;
        *value
    };
}

fn main() {}
