use kaalang::kaalang;
#[kaalang]
fn mutable_initial_parameter(mut go: u8) -> u8 {
    #[action("Change the initial value.")]
    |&mut go| *go += 1;
    #[stage("Return the received value.")]
    |go| {
        |go| return go;
    };
}
#[test]
fn a_mutable_initial_parameter_reaches_an_immutable_entry() {
    assert_eq!(mutable_initial_parameter(4), 5);
}
