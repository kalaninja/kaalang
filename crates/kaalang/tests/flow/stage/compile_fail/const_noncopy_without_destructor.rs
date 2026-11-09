use kaalang::kaa;

struct NonCopy(u8);

#[kaa]
const fn const_noncopy_without_destructor(go: NonCopy) -> u8 {
    #[kaa::stage("Read the value.")]
    let finish = |go| {
        #[kaa::action("Read the value.")]
        let finish = |go| go.0;
    };

    #[kaa::stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
