use kaalang::kaalang;

#[kaalang]
const fn count_to(limit: usize) -> usize {
    #[action("Initialize the counter.")]
    let mut count = || 0usize;

    #[question("Is the counter below the limit?")]
    while (|&count, &limit| *count < *limit) {
        #[action("Increment the counter.")]
        |&mut count| *count += 1;
    }

    #[action("Return the counter.")]
    let result = |count| count;
}

#[test]
fn repeats_until_the_condition_is_false() {
    const THREE: usize = count_to(3);
    assert_eq!(THREE, 3);
    for limit in [0, 1, 10] {
        assert_eq!(count_to(limit), limit);
    }
}
