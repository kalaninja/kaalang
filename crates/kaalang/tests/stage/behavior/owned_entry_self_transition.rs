use kaalang::kaalang;
#[kaalang]
fn owned_entry_self_transition(go: String) -> String {
    #[stage("Grow the string.")]
    let (go, finish) = |go| {
        #[action("Read the current string.")]
        let current = |go| go;

        #[question("Grow again?")]
        let (again, done) = |&current| current.len() < 3;
        #[action("Append a letter.")]
        let go = |again, mut current| {
            current.push('x');
            current
        };
        #[action("Keep the completed string.")]
        let finish = |done, current| current;
    };
    #[stage("Return the string.")]
    |finish| {
        |finish| return finish;
    };
}
#[test]
fn owned_entry_moves_through_each_self_transition() {
    assert_eq!(owned_entry_self_transition(String::from("a")), "axx");
}
