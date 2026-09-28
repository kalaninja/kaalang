use kaalang::kaalang;

#[kaalang]
const fn const_outgoing_signal(go: ()) -> String {
    #[stage("Create text.")]
    let finish = |go| {
        #[action("Create the text.")]
        let finish = |go| String::new();
    };

    #[stage("Return text.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
