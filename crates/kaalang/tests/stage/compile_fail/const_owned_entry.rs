use kaalang::kaalang;

#[kaalang]
const fn const_owned_entry(go: String) -> String {
    #[stage("Forward the string.")]
    let finish = |go| {
        #[action("Forward the value.")]
        let finish = |go| go;
    };

    #[stage("Return the string.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
