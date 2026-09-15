//! A `for` loop written without a `for`. kaalang has no iteration construct,
//! so the cursor is an ordinary wire and the choice that asks it for the next
//! value is an ordinary block. Everything a `for` would hide is drawn.

use kaalang::kaalang;

#[kaalang]
fn for_each(values: &[i32]) -> i32 {
    #[action("Start the total at zero.")]
    let mut running = || 0;

    #[action("Start at the first value.")]
    let mut cursor = |values| values.iter();

    #[cycle("Add every value to the total.")]
    let total = |mut cursor, mut running| {
        #[choice("Is there another value?")]
        #[case("There is one.")]
        #[case("The values ran out.")]
        let (value, done) = |&mut cursor| match cursor.next() {
            Some(next) => next,
            None => (),
        };

        |done, running| break running;

        #[action("Add it to the total.")]
        |value, &mut running| *running += value;
    };

    |total| return total;
}

#[test]
fn for_each_adds_up_every_value() {
    assert_eq!(for_each(&[]), 0);
    assert_eq!(for_each(&[7]), 7);
    assert_eq!(for_each(&[1, 2, 3, 4]), 10);
    assert_eq!(for_each(&[5, -5, 5]), 5);
}
