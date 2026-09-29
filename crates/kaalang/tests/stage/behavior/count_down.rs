use kaalang::kaalang;

#[kaalang]
const fn count_down(count: usize) -> usize {
    #[stage("Count down to zero.")]
    let (count, finish) = |count| {
        #[choice("Is another step needed?")]
        #[case("Take the next step.")]
        #[case("The count is zero.")]
        let (count, finish) = |count| match count {
            value if value > 0 => value - 1,
            _ => 0usize,
        };
    };

    #[stage("Return zero.")]
    |finish| {
        |finish| return finish;
    };
}

const _: () = assert!(count_down(7) == 0);

#[test]
fn counts_down_in_const_and_runtime_calls() {
    assert_eq!(count_down(3), 0);
}
