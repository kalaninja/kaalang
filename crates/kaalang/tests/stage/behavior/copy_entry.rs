use kaalang::kaalang;

#[kaalang]
const fn copy_entry<T: Copy>(go: T) -> T {
    #[stage("Forward the copied entry.")]
    let finish = |go| {
        #[action("Forward the value.")]
        let finish = |go| go;
    };

    #[stage("Return the copied entry.")]
    |finish| {
        |finish| return finish;
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Token(u8);

const _: () = assert!(copy_entry((1u8, 2u8)).0 == 1);

#[test]
fn const_dispatch_carries_generic_copy_values() {
    assert_eq!(copy_entry(Token(3)), Token(3));
    assert_eq!(copy_entry([4u8, 5u8]), [4, 5]);
    assert_eq!(copy_entry(&7u8), &7);
}
