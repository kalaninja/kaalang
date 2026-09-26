use kaalang::kaalang;

#[kaalang]
fn repeats_around_an_endless_cycle(mut mode: u8) -> u8 {
    #[cycle("Settle the mode.")]
    let leave = {
        #[choice("Which route?")]
        #[case("Advance on the left.")]
        #[case("Serve forever.")]
        #[case("Advance on the right.")]
        #[case("Leave.")]
        let (left, serve, right, leave) = |&mode| match *mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        };

        #[action("Settle from the left.")]
        let advanced = |left, &mut mode| *mode = 3;

        #[cycle("Serve requests forever.")]
        |serve| {
            #[choice("Which request?")]
            #[case("A read.")]
            #[case("A write.")]
            let (read, write) = |&mode| match *mode {
                0 => (),
                _ => (),
            };

            #[action("Serve a read.")]
            let served = |read| {};

            #[action("Serve a write.")]
            let served = |write| {};

            |served| continue;
        };

        #[action("Settle from the right.")]
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
