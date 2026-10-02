use kaalang::kaalang;
use std::cell::Cell;

use super::{ROUTE, expected, selected};

thread_local! { static NEXT_ROUTE: Cell<usize> = const { Cell::new(0) }; }
#[kaalang]
fn staged_branching_with_work(
    seed: usize,
    config: usize,
    events: &Cell<usize>,
    remaining: &Cell<usize>,
) -> usize {
    #[action("Prepare the seed.")]
    let pre_prepared = |seed| seed;
    #[question("Choose work 0.")]
    let (pre_case_0_0, pre_case_0_1) = |&config| selected(0) == 0;
    #[action("Prepare branch 0/0.")]
    let pre_data_0_0 = |pre_case_0_0, &pre_prepared, &config| *pre_prepared + *config;
    #[action("Record branch 0/0.")]
    let pre_effect_0_0 = |&pre_data_0_0, &events| {
        assert_eq!(events.get(), 0);
        events.set(events.get() | (1usize << 0));
    };
    #[action("Finish branch 0/0.")]
    let pre_step_0 = |pre_data_0_0, pre_effect_0_0| pre_data_0_0;
    #[action("Prepare branch 0/1.")]
    let pre_data_0_1 = |pre_case_0_1, &pre_prepared, &config| *pre_prepared + *config + 1;
    #[call("Transform branch 0/1/0.")]
    let pre_data_0_1_0 = |pre_data_0_1| std::convert::identity(pre_data_0_1);
    #[action("Record branch 0/1.")]
    let pre_effect_0_1 = |&pre_data_0_1_0, &events| {
        assert_eq!(events.get(), 0);
        events.set(events.get() | (1usize << 0));
    };
    #[action("Finish branch 0/1.")]
    let pre_step_0 = |pre_data_0_1_0, pre_effect_0_1| pre_data_0_1_0;
    #[choice("Choose work 1.")]
    #[case("First.")]
    #[case("Second.")]
    #[case("Third.")]
    let (pre_case_1_0, pre_case_1_1, pre_case_1_2) = |&config| match selected(1) {
        0 => (),
        1 => (),
        _ => (),
    };
    #[action("Prepare branch 1/0.")]
    let pre_data_1_0 =
        |pre_case_1_0, &pre_step_0, &config, &pre_prepared| *pre_step_0 + *config + *pre_prepared;
    #[action("Record branch 1/0.")]
    let pre_effect_1_0 = |&pre_data_1_0, &events| {
        assert_eq!(events.get(), (1usize << 1) - 1);
        events.set(events.get() | (1usize << 1));
    };
    #[action("Finish branch 1/0.")]
    let pre_step_1 = |pre_data_1_0, pre_effect_1_0| pre_data_1_0;
    #[action("Prepare branch 1/1.")]
    let pre_data_1_1 = |pre_case_1_1, &pre_step_0, &config, &pre_prepared| {
        *pre_step_0 + *config + *pre_prepared + 1
    };
    #[call("Transform branch 1/1/0.")]
    let pre_data_1_1_0 = |pre_data_1_1| std::convert::identity(pre_data_1_1);
    #[action("Record branch 1/1.")]
    let pre_effect_1_1 = |&pre_data_1_1_0, &events| {
        assert_eq!(events.get(), (1usize << 1) - 1);
        events.set(events.get() | (1usize << 1));
    };
    #[action("Finish branch 1/1.")]
    let pre_step_1 = |pre_data_1_1_0, pre_effect_1_1| pre_data_1_1_0;
    #[action("Prepare branch 1/2.")]
    let pre_data_1_2 = |pre_case_1_2, &pre_step_0, &config, &pre_prepared| {
        *pre_step_0 + *config + *pre_prepared + 2
    };
    #[call("Transform branch 1/2/0.")]
    let pre_data_1_2_0 = |pre_data_1_2| std::convert::identity(pre_data_1_2);
    #[call("Transform branch 1/2/1.")]
    let pre_data_1_2_1 = |pre_data_1_2_0| std::convert::identity(pre_data_1_2_0);
    #[action("Record branch 1/2.")]
    let pre_effect_1_2 = |&pre_data_1_2_1, &events| {
        assert_eq!(events.get(), (1usize << 1) - 1);
        events.set(events.get() | (1usize << 1));
    };
    #[action("Finish branch 1/2.")]
    let pre_step_1 = |pre_data_1_2_1, pre_effect_1_2| pre_data_1_2_1;
    #[question("Choose work 2.")]
    let (pre_case_2_0, pre_case_2_1) = |&config| selected(2) == 0;
    #[action("Prepare branch 2/0.")]
    let pre_data_2_0 =
        |pre_case_2_0, &pre_step_1, &config, &pre_step_0| *pre_step_1 + *config + *pre_step_0;
    #[action("Record branch 2/0.")]
    let pre_effect_2_0 = |&pre_data_2_0, &events| {
        assert_eq!(events.get(), (1usize << 2) - 1);
        events.set(events.get() | (1usize << 2));
    };
    #[action("Finish branch 2/0.")]
    let pre_step_2 = |pre_data_2_0, pre_effect_2_0| pre_data_2_0;
    #[action("Prepare branch 2/1.")]
    let pre_data_2_1 =
        |pre_case_2_1, &pre_step_1, &config, &pre_step_0| *pre_step_1 + *config + *pre_step_0 + 1;
    #[call("Transform branch 2/1/0.")]
    let pre_data_2_1_0 = |pre_data_2_1| std::convert::identity(pre_data_2_1);
    #[action("Record branch 2/1.")]
    let pre_effect_2_1 = |&pre_data_2_1_0, &events| {
        assert_eq!(events.get(), (1usize << 2) - 1);
        events.set(events.get() | (1usize << 2));
    };
    #[action("Finish branch 2/1.")]
    let pre_step_2 = |pre_data_2_1_0, pre_effect_2_1| pre_data_2_1_0;
    #[choice("Choose work 3.")]
    #[case("First.")]
    #[case("Second.")]
    #[case("Third.")]
    let (pre_case_3_0, pre_case_3_1, pre_case_3_2) = |&config| match selected(3) {
        0 => (),
        1 => (),
        _ => (),
    };
    #[action("Prepare branch 3/0.")]
    let pre_data_3_0 =
        |pre_case_3_0, &pre_step_2, &config, &pre_step_0| *pre_step_2 + *config + *pre_step_0;
    #[action("Record branch 3/0.")]
    let pre_effect_3_0 = |&pre_data_3_0, &events| {
        assert_eq!(events.get(), (1usize << 3) - 1);
        events.set(events.get() | (1usize << 3));
    };
    #[action("Finish branch 3/0.")]
    let pre_step_3 = |pre_data_3_0, pre_effect_3_0| pre_data_3_0;
    #[action("Prepare branch 3/1.")]
    let pre_data_3_1 =
        |pre_case_3_1, &pre_step_2, &config, &pre_step_0| *pre_step_2 + *config + *pre_step_0 + 1;
    #[call("Transform branch 3/1/0.")]
    let pre_data_3_1_0 = |pre_data_3_1| std::convert::identity(pre_data_3_1);
    #[action("Record branch 3/1.")]
    let pre_effect_3_1 = |&pre_data_3_1_0, &events| {
        assert_eq!(events.get(), (1usize << 3) - 1);
        events.set(events.get() | (1usize << 3));
    };
    #[action("Finish branch 3/1.")]
    let pre_step_3 = |pre_data_3_1_0, pre_effect_3_1| pre_data_3_1_0;
    #[action("Prepare branch 3/2.")]
    let pre_data_3_2 =
        |pre_case_3_2, &pre_step_2, &config, &pre_step_0| *pre_step_2 + *config + *pre_step_0 + 2;
    #[call("Transform branch 3/2/0.")]
    let pre_data_3_2_0 = |pre_data_3_2| std::convert::identity(pre_data_3_2);
    #[call("Transform branch 3/2/1.")]
    let pre_data_3_2_1 = |pre_data_3_2_0| std::convert::identity(pre_data_3_2_0);
    #[action("Record branch 3/2.")]
    let pre_effect_3_2 = |&pre_data_3_2_1, &events| {
        assert_eq!(events.get(), (1usize << 3) - 1);
        events.set(events.get() | (1usize << 3));
    };
    #[action("Finish branch 3/2.")]
    let pre_step_3 = |pre_data_3_2_1, pre_effect_3_2| pre_data_3_2_1;
    #[question("Choose work 4.")]
    let (pre_case_4_0, pre_case_4_1) = |&config| selected(4) == 0;
    #[action("Prepare branch 4/0.")]
    let pre_data_4_0 =
        |pre_case_4_0, &pre_step_3, &config, &pre_step_1| *pre_step_3 + *config + *pre_step_1;
    #[action("Record branch 4/0.")]
    let pre_effect_4_0 = |&pre_data_4_0, &events| {
        assert_eq!(events.get(), (1usize << 4) - 1);
        events.set(events.get() | (1usize << 4));
    };
    #[action("Finish branch 4/0.")]
    let pre_step_4 = |pre_data_4_0, pre_effect_4_0| pre_data_4_0;
    #[action("Prepare branch 4/1.")]
    let pre_data_4_1 =
        |pre_case_4_1, &pre_step_3, &config, &pre_step_1| *pre_step_3 + *config + *pre_step_1 + 1;
    #[call("Transform branch 4/1/0.")]
    let pre_data_4_1_0 = |pre_data_4_1| std::convert::identity(pre_data_4_1);
    #[action("Record branch 4/1.")]
    let pre_effect_4_1 = |&pre_data_4_1_0, &events| {
        assert_eq!(events.get(), (1usize << 4) - 1);
        events.set(events.get() | (1usize << 4));
    };
    #[action("Finish branch 4/1.")]
    let pre_step_4 = |pre_data_4_1_0, pre_effect_4_1| pre_data_4_1_0;
    #[action("Enter the work stage.")]
    let entry = |pre_step_4| pre_step_4;

    #[stage("Do more work.")]
    let (finish_left, finish_right) = |entry| {
        #[action("Reset the stage effects.")]
        let reset = |entry, &events| {
            events.set(0);
            ROUTE.set(NEXT_ROUTE.get());
        };

        #[cycle("Repeat before working.")]
        let (left, right) = |reset| {
            #[question("Repeat?")]
            let (again, work) = |&remaining| remaining.get() > 0;
            #[action("Count the repeat.")]
            let repeated = |again, &remaining| remaining.set(remaining.get() - 1);
            |repeated| continue;

            #[action("Prepare the seed.")]
            let prepared = |work, entry| entry;
            #[question("Choose work 0.")]
            let (case_0_0, case_0_1) = |&prepared, &config| selected(0) == 0;
            #[action("Prepare branch 0/0.")]
            let data_0_0 = |case_0_0, &prepared, &config| *prepared + *config;
            #[action("Record branch 0/0.")]
            let effect_0_0 = |&data_0_0, &events| {
                assert_eq!(events.get(), 0);
                events.set(events.get() | (1usize << 0));
            };
            #[action("Finish branch 0/0.")]
            let step_0 = |data_0_0, effect_0_0| data_0_0;
            #[action("Prepare branch 0/1.")]
            let data_0_1 = |case_0_1, &prepared, &config| *prepared + *config + 1;
            #[call("Transform branch 0/1/0.")]
            let data_0_1_0 = |data_0_1| std::convert::identity(data_0_1);
            #[action("Record branch 0/1.")]
            let effect_0_1 = |&data_0_1_0, &events| {
                assert_eq!(events.get(), 0);
                events.set(events.get() | (1usize << 0));
            };
            #[action("Finish branch 0/1.")]
            let step_0 = |data_0_1_0, effect_0_1| data_0_1_0;
            #[choice("Choose work 1.")]
            #[case("First.")]
            #[case("Second.")]
            #[case("Third.")]
            let (case_1_0, case_1_1, case_1_2) = |&step_0, &config| match selected(1) {
                0 => (),
                1 => (),
                _ => (),
            };
            #[action("Prepare branch 1/0.")]
            let data_1_0 = |case_1_0, &step_0, &config, &prepared| *step_0 + *config + *prepared;
            #[action("Record branch 1/0.")]
            let effect_1_0 = |&data_1_0, &events| {
                assert_eq!(events.get(), (1usize << 1) - 1);
                events.set(events.get() | (1usize << 1));
            };
            #[action("Finish branch 1/0.")]
            let step_1 = |data_1_0, effect_1_0| data_1_0;
            #[action("Prepare branch 1/1.")]
            let data_1_1 =
                |case_1_1, &step_0, &config, &prepared| *step_0 + *config + *prepared + 1;
            #[call("Transform branch 1/1/0.")]
            let data_1_1_0 = |data_1_1| std::convert::identity(data_1_1);
            #[action("Record branch 1/1.")]
            let effect_1_1 = |&data_1_1_0, &events| {
                assert_eq!(events.get(), (1usize << 1) - 1);
                events.set(events.get() | (1usize << 1));
            };
            #[action("Finish branch 1/1.")]
            let step_1 = |data_1_1_0, effect_1_1| data_1_1_0;
            #[action("Prepare branch 1/2.")]
            let data_1_2 =
                |case_1_2, &step_0, &config, &prepared| *step_0 + *config + *prepared + 2;
            #[call("Transform branch 1/2/0.")]
            let data_1_2_0 = |data_1_2| std::convert::identity(data_1_2);
            #[call("Transform branch 1/2/1.")]
            let data_1_2_1 = |data_1_2_0| std::convert::identity(data_1_2_0);
            #[action("Record branch 1/2.")]
            let effect_1_2 = |&data_1_2_1, &events| {
                assert_eq!(events.get(), (1usize << 1) - 1);
                events.set(events.get() | (1usize << 1));
            };
            #[action("Finish branch 1/2.")]
            let step_1 = |data_1_2_1, effect_1_2| data_1_2_1;
            #[question("Choose work 2.")]
            let (case_2_0, case_2_1) = |&step_1, &config| selected(2) == 0;
            #[action("Prepare branch 2/0.")]
            let data_2_0 = |case_2_0, &step_1, &config, &step_0| *step_1 + *config + *step_0;
            #[action("Record branch 2/0.")]
            let effect_2_0 = |&data_2_0, &events| {
                assert_eq!(events.get(), (1usize << 2) - 1);
                events.set(events.get() | (1usize << 2));
            };
            #[action("Finish branch 2/0.")]
            let step_2 = |data_2_0, effect_2_0| data_2_0;
            #[action("Prepare branch 2/1.")]
            let data_2_1 = |case_2_1, &step_1, &config, &step_0| *step_1 + *config + *step_0 + 1;
            #[call("Transform branch 2/1/0.")]
            let data_2_1_0 = |data_2_1| std::convert::identity(data_2_1);
            #[action("Record branch 2/1.")]
            let effect_2_1 = |&data_2_1_0, &events| {
                assert_eq!(events.get(), (1usize << 2) - 1);
                events.set(events.get() | (1usize << 2));
            };
            #[action("Finish branch 2/1.")]
            let step_2 = |data_2_1_0, effect_2_1| data_2_1_0;
            #[choice("Choose work 3.")]
            #[case("First.")]
            #[case("Second.")]
            #[case("Third.")]
            let (case_3_0, case_3_1, case_3_2) = |&step_2, &config| match selected(3) {
                0 => (),
                1 => (),
                _ => (),
            };
            #[action("Prepare branch 3/0.")]
            let data_3_0 = |case_3_0, &step_2, &config, &step_0| *step_2 + *config + *step_0;
            #[action("Record branch 3/0.")]
            let effect_3_0 = |&data_3_0, &events| {
                assert_eq!(events.get(), (1usize << 3) - 1);
                events.set(events.get() | (1usize << 3));
            };
            #[action("Finish branch 3/0.")]
            let step_3 = |data_3_0, effect_3_0| data_3_0;
            #[action("Prepare branch 3/1.")]
            let data_3_1 = |case_3_1, &step_2, &config, &step_0| *step_2 + *config + *step_0 + 1;
            #[call("Transform branch 3/1/0.")]
            let data_3_1_0 = |data_3_1| std::convert::identity(data_3_1);
            #[action("Record branch 3/1.")]
            let effect_3_1 = |&data_3_1_0, &events| {
                assert_eq!(events.get(), (1usize << 3) - 1);
                events.set(events.get() | (1usize << 3));
            };
            #[action("Finish branch 3/1.")]
            let step_3 = |data_3_1_0, effect_3_1| data_3_1_0;
            #[action("Prepare branch 3/2.")]
            let data_3_2 = |case_3_2, &step_2, &config, &step_0| *step_2 + *config + *step_0 + 2;
            #[call("Transform branch 3/2/0.")]
            let data_3_2_0 = |data_3_2| std::convert::identity(data_3_2);
            #[call("Transform branch 3/2/1.")]
            let data_3_2_1 = |data_3_2_0| std::convert::identity(data_3_2_0);
            #[action("Record branch 3/2.")]
            let effect_3_2 = |&data_3_2_1, &events| {
                assert_eq!(events.get(), (1usize << 3) - 1);
                events.set(events.get() | (1usize << 3));
            };
            #[action("Finish branch 3/2.")]
            let step_3 = |data_3_2_1, effect_3_2| data_3_2_1;
            #[question("Choose work 4.")]
            let (case_4_0, case_4_1) = |&step_3, &config| selected(4) == 0;
            #[action("Prepare branch 4/0.")]
            let data_4_0 = |case_4_0, &step_3, &config, &step_1| *step_3 + *config + *step_1;
            #[action("Record branch 4/0.")]
            let effect_4_0 = |&data_4_0, &events| {
                assert_eq!(events.get(), (1usize << 4) - 1);
                events.set(events.get() | (1usize << 4));
            };
            #[action("Finish branch 4/0.")]
            let step_4 = |data_4_0, effect_4_0| data_4_0;
            #[action("Prepare branch 4/1.")]
            let data_4_1 = |case_4_1, &step_3, &config, &step_1| *step_3 + *config + *step_1 + 1;
            #[call("Transform branch 4/1/0.")]
            let data_4_1_0 = |data_4_1| std::convert::identity(data_4_1);
            #[action("Record branch 4/1.")]
            let effect_4_1 = |&data_4_1_0, &events| {
                assert_eq!(events.get(), (1usize << 4) - 1);
                events.set(events.get() | (1usize << 4));
            };
            #[action("Finish branch 4/1.")]
            let step_4 = |data_4_1_0, effect_4_1| data_4_1_0;
            #[question("Which result?")]
            let (pick_left, pick_right) = |&step_4, &config| *config & 1 == 0;
            #[action("Left result.")]
            let left = |pick_left, step_4| step_4;
            #[action("Right result.")]
            let right = |pick_right, step_4| (step_4,);
        };
        #[action("Use the left result.")]
        let finish_left = |left| left;
        #[action("Use the right result.")]
        let finish_right = |right| right.0;
    };
    #[stage("Finish on the left.")]
    let finish_right = |finish_left| {
        #[action("Forward the left result.")]
        let finish_right = |finish_left| finish_left;
    };
    #[stage("Finish on the right.")]
    |finish_right| {
        |finish_right| return finish_right;
    };
}

#[test]
fn independent_preparation_and_stage_routes_preserve_values_and_effect_order() {
    for initial in 0..72 {
        for next in 0..72 {
            NEXT_ROUTE.set(next);
            for seed in [0, 1, 7] {
                for config in [0, 3, 11] {
                    ROUTE.set(initial);
                    let first = expected(seed, config);
                    ROUTE.set(next);
                    let result = expected(first, config);
                    for repeats in [0, 1, 3] {
                        ROUTE.set(initial);
                        let events = Cell::new(0);
                        let remaining = Cell::new(repeats);
                        assert_eq!(
                            staged_branching_with_work(seed, config, &events, &remaining),
                            result
                        );
                        assert_eq!(events.get(), 31);
                        assert_eq!(remaining.get(), 0);
                    }
                }
            }
        }
    }
}
