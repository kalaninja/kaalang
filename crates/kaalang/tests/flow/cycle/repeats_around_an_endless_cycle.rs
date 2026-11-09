use kaalang::kaa;

#[kaa]
fn repeats_around_an_endless_cycle(mut mode: u8) -> u8 {
    #[kaa::cycle("Settle the mode.")]
    let leave = loop {
        #[kaa::choice("Which route?")]
        #[kaa::case("Advance on the left.")]
        #[kaa::case("Serve forever.")]
        #[kaa::case("Advance on the right.")]
        #[kaa::case("Leave.")]
        let (left, serve, right, leave) = |&mode| match *mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        };

        #[kaa::action("Settle from the left.")]
        let advanced = |left, &mut mode| *mode = 3;

        #[kaa::cycle("Serve requests forever.")]
        |serve| loop {
            #[kaa::choice("Which request?")]
            #[kaa::case("A read.")]
            #[kaa::case("A write.")]
            let (read, write) = |&mode| match *mode {
                0 => (),
                _ => (),
            };

            #[kaa::action("Serve a read.")]
            let served = |read| {};

            #[kaa::action("Serve a write.")]
            let served = |write| {};

            |served| continue;
        };

        #[kaa::action("Settle from the right.")]
        let advanced = |right, &mut mode| *mode = 3;

        |advanced| continue;
    };

    |leave, mode| return mode;
}

/// The serving route never returns, so the settling routes on either side of it
/// meet at the iteration tail below its whole boundary.
#[test]
fn the_settling_routes_repeat_around_the_endless_cycle() {
    assert_eq!(repeats_around_an_endless_cycle(0), 3);
    assert_eq!(repeats_around_an_endless_cycle(2), 3);
    assert_eq!(repeats_around_an_endless_cycle(3), 3);
}
