use kaalang::kaalang;

#[kaalang]
const fn const_initial_cycle_signal() -> String {
    #[cycle("Prepare text.")]
    let go = loop {
        #[action("Create the text.")]
        let go = || String::new();
    };

    #[stage("Return text.")]
    |go| {
        |go| return go;
    };
}

fn main() {}
