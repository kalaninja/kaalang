use kaalang::kaa;

#[kaa]
fn invalid(value: u8) {
    #[kaa::action("Finish without the flow input.")]
    let end = || {};

    |end| return end;
}

fn main() {}
