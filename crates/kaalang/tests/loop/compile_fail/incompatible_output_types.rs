use kaalang::kaalang;

#[kaalang]
fn invalid(number: usize, choose_number: bool) -> usize {
    #[cycle("Provide incompatible result types.")]
    let selected = {
        #[question("Provide the number?")]
        let (number_route, text_route) = |choose_number| choose_number;

        #[action("Keep the number.")]
        let selected = |number_route, number| number;

        #[action("Build text.")]
        let selected = |text_route| String::from("text");
    };

    |selected| return selected;
}

fn main() {}
