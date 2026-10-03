use kaalang::kaalang;

#[kaalang]
fn forward<T>(go: T) -> T {
    #[stage("Forward the incoming value.")]
    let finish = |go| {
        #[cycle("Complete on the first iteration.")]
        let finish = |go| loop {
            #[action("Provide the incoming value.")]
            let finish = |go| go;
        };
    };

    #[stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn forwards_an_owned_value_through_a_cycle() {
    let input = String::from("owned");
    let pointer = input.as_ptr();
    let result = forward(input);
    assert_eq!(result, "owned");
    assert_eq!(result.as_ptr(), pointer);
}
