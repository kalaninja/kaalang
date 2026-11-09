use kaalang::kaa;

#[kaa]
fn measure_chosen_text(go: ()) -> usize {
    #[kaa::stage("Measure the chosen text.")]
    let finish = |text| {
        #[kaa::action("Count the bytes of the text.")]
        let finish = |&text| text.len();
    };

    #[kaa::stage("Choose the text.")]
    let text = |go| {
        #[kaa::action("Choose a greeting.")]
        let text = || String::from("hello");
    };

    #[kaa::stage("Return the length.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
