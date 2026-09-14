use kaalang::kaalang;

#[kaalang]
fn invalid(number: usize, choose_number: bool) -> usize {
    #[cycle("Provide incompatible result types.")]
    let result = |number, choose_number| {
        #[question("Provide the number?")]
        let (number_route, text_route) = |choose_number| choose_number;

        |number_route, number| break number;

        #[action("Build text.")]
        let text = |text_route| String::from("text");

        |text| break text;
    };

    |result| return result;
}

fn main() {}
