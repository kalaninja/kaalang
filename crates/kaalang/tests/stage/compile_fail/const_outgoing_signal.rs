use kaalang::kaa;

#[kaa]
const fn const_outgoing_signal(go: ()) -> String {
    #[kaa::stage("Create text.")]
    let finish = |go| {
        #[kaa::action("Create the text.")]
        let finish = |go| String::new();
    };

    #[kaa::stage("Return text.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
