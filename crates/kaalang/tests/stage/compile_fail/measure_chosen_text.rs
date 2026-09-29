use kaalang::kaalang;

#[kaalang]
fn measure_chosen_text(go: ()) -> usize {
    #[stage("Measure the chosen text.")]
    let finish = |text| {
        #[action("Count the bytes of the text.")]
        let finish = |&text| text.len();
    };

    #[stage("Choose the text.")]
    let text = |go| {
        #[action("Choose a greeting.")]
        let text = || String::from("hello");
    };

    #[stage("Return the length.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
