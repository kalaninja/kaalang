use kaalang::kaalang;

struct NonCopy(u8);

#[kaalang]
const fn const_noncopy_without_destructor(go: NonCopy) -> u8 {
    #[stage("Read the value.")]
    let finish = |go| {
        #[action("Read the value.")]
        let finish = |go| go.0;
    };

    #[stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
