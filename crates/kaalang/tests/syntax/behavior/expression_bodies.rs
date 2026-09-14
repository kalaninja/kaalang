use std::cell::Cell;

use kaalang::kaalang;

#[kaalang]
fn expression_bodies(input: Option<u32>, visits: &Cell<u32>) -> u32 {
    #[action("Record a visit.")]
    |visits| visits.set(visits.get() + 1);

    #[choice("Was a value supplied?")]
    #[case("Use the supplied value.")]
    #[case("Use a default.")]
    let (value, absent) = |input| match input {
        Some(value) => value,
        None => (),
    };

    #[action("Capture the supplied branch value.")]
    let number = |value| value;

    #[question("Is the value positive?")]
    let (positive, zero) = |&number| *number > 0;

    #[action("Double the positive value.")]
    let end = |positive, number| number * 2;

    #[action("Return zero.")]
    let end = |zero| 0;

    #[action("Return the default.")]
    let end = |absent| 1;

    |end| return end;
}

#[test]
fn expression_bodies_run_for_every_block_kind() {
    let visits = Cell::new(0);
    assert_eq!(expression_bodies(Some(3), &visits), 6);
    assert_eq!(expression_bodies(Some(0), &visits), 0);
    assert_eq!(expression_bodies(None, &visits), 1);
    assert_eq!(visits.get(), 3);
}
