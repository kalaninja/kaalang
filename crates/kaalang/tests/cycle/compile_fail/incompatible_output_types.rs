use kaalang::kaa;

#[kaa]
fn invalid(number: usize, choose_number: bool) -> usize {
    #[kaa::cycle("Provide incompatible result types.")]
    let selected = loop {
        #[kaa::question("Provide the number?")]
        let (number_route, text_route) = |choose_number| choose_number;

        #[kaa::action("Keep the number.")]
        let selected = |number_route, number| number;

        #[kaa::action("Build text.")]
        let selected = |text_route| String::from("text");
    };

    |selected| return selected;
}

fn main() {}
