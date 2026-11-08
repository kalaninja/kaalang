use kaalang::kaa;

#[kaa]
fn loop_inside_for(values: &[u32]) -> u32 {
    #[kaa::action("Start with a total of zero.")]
    let mut total = || 0;

    #[kaa::cycle("Count down every value.")]
    |values| {
        for value in values {
            #[kaa::action("Start counting at the value.")]
            let mut left = |value| *value;

            #[kaa::cycle("Count the value down to zero.")]
            let _zero = loop {
                #[kaa::question("Has the value been counted down?")]
                let (_zero, more) = |&left| *left == 0;

                #[kaa::action("Count one down.")]
                let counted = |more, &mut left, &mut total| {
                    *left -= 1;
                    *total += 1;
                };

                |counted| continue;
            };
        }
    };

    |total| return total;
}

#[test]
fn a_loop_cycle_completes_inside_every_item() {
    assert_eq!(loop_inside_for(&[]), 0);
    assert_eq!(loop_inside_for(&[2, 0, 3]), 5);
}
