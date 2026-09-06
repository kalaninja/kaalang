use kaalang::kaalang;

#[kaalang]
fn fizzbuzz(number: u32) -> String {
    #[choice("Which of three and five divide the number?")]
    #[case("Both three and five divide it.")]
    #[case("Only three divides it.")]
    #[case("Only five divides it.")]
    #[case("Neither divides it.")]
    |number| -> (fizz_buzz, fizz, buzz, plain) {
        match (number % 3, number % 5) {
            (0, 0) => (),
            (0, _) => (),
            (_, 0) => (),
            _ => number,
        }
    };

    #[action("Say FizzBuzz.")]
    |fizz_buzz| -> result { String::from("FizzBuzz") };

    #[action("Say Fizz.")]
    |fizz| -> result { String::from("Fizz") };

    #[action("Say Buzz.")]
    |buzz| -> result { String::from("Buzz") };

    #[action("Say the number itself.")]
    |plain| -> result { plain.to_string() };
}

#[test]
fn fizzbuzz_names_the_first_fifteen_numbers() {
    let said: Vec<String> = (1..=15).map(fizzbuzz).collect();

    assert_eq!(
        said,
        [
            "1", "2", "Fizz", "4", "Buzz", "Fizz", "7", "8", "Fizz", "Buzz", "11", "Fizz", "13",
            "14", "FizzBuzz"
        ]
    );
}
