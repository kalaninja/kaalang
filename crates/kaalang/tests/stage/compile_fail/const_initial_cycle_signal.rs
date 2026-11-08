use kaalang::kaa;

#[kaa]
const fn const_initial_cycle_signal() -> String {
    #[kaa::cycle("Prepare text.")]
    let go = loop {
        #[kaa::action("Create the text.")]
        let go = || String::new();
    };

    #[kaa::stage("Return text.")]
    |go| {
        |go| return go;
    };
}

fn main() {}
