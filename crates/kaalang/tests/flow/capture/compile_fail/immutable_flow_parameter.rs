use kaalang::kaa;

#[kaa]
fn immutable_flow_parameter(value: u8) -> u8 {
    #[kaa::action("Try to mutate an immutable flow parameter.")]
    let end = |&mut value| {
        *value += 1;
        *value
    };

    |end| return end;
}

fn main() {}
