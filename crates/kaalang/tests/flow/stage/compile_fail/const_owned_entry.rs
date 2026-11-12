use kaalang::kaa;

#[kaa]
const fn const_owned_entry(go: String) -> String {
    #[kaa::stage("Forward the string.")]
    let finish = |go| {
        #[kaa::action("Forward the value.")]
        let finish = |go| go;
    };

    #[kaa::stage("Return the string.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
