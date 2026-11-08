use kaalang::kaa;

#[kaa]
const fn count_down(count: usize) -> usize {
    #[kaa::stage("Count down to zero.")]
    let (count, finish) = |count| {
        #[kaa::choice("Is another step needed?")]
        #[kaa::case("Take the next step.")]
        #[kaa::case("The count is zero.")]
        let (count, finish) = |count| match count {
            value if value > 0 => value - 1,
            _ => 0usize,
        };
    };

    #[kaa::stage("Return zero.")]
    |finish| {
        |finish| return finish;
    };
}

const _: () = assert!(count_down(7) == 0);

#[test]
fn counts_down_in_const_and_runtime_calls() {
    assert_eq!(count_down(3), 0);
}
