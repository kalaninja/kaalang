use kaalang::kaa;
#[kaa]
fn owned_entry_self_transition(go: String) -> String {
    #[kaa::stage("Grow the string.")]
    let (go, finish) = |go| {
        #[kaa::action("Read the current string.")]
        let current = |go| go;

        #[kaa::question("Grow again?")]
        let (again, done) = |&current| current.len() < 3;
        #[kaa::action("Append a letter.")]
        let go = |again, mut current| {
            current.push('x');
            current
        };
        #[kaa::action("Keep the completed string.")]
        let finish = |done, current| current;
    };
    #[kaa::stage("Return the string.")]
    |finish| {
        |finish| return finish;
    };
}
#[test]
fn owned_entry_moves_through_each_self_transition() {
    assert_eq!(owned_entry_self_transition(String::from("a")), "axx");
}
