use kaalang::kaalang;

#[kaalang]
fn nested_loop_tail(mut count: usize, limit: usize) -> usize {
    #[question("Is another counting pass needed?")]
    #[no]
    #[yes]
    while (|count, limit| count < limit) {
        #[question("Is the count below the limit?")]
        while (|count, limit| count < limit) {
            #[action("Increment the count.")]
            |&mut count| *count += 1;
        }
    }

    #[action("Return the count.")]
    let end = |count| count;
}

#[test]
fn inner_exit_can_finish_the_outer_iteration() {
    assert_eq!(nested_loop_tail(0, 0), 0);
    assert_eq!(nested_loop_tail(0, 4), 4);
    assert_eq!(nested_loop_tail(5, 4), 5);
}
