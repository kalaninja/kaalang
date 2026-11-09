#[allow(unused_must_use)]
fn main() {
    #[kaa::action("Copy the input.")]
    let _output = |input: u32| { input };
}
