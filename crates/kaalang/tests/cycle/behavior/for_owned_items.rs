use kaalang::kaalang;

#[kaalang]
fn for_owned_items(names: Vec<String>) -> Vec<String> {
    #[action("Start with no greetings.")]
    let mut greetings = || Vec::new();

    #[cycle("Greet every name.")]
    |names| {
        for mut name in names {
            #[action("Turn the name into a greeting.")]
            let greeting = |mut name| {
                name.insert_str(0, "Hello, ");
                name
            };

            #[action("Keep the greeting.")]
            |greeting, &mut greetings| greetings.push(greeting);
        }
    };

    |greetings| return greetings;
}

#[test]
fn owned_items_move_into_the_body() {
    assert_eq!(
        for_owned_items(vec!["Ann".to_owned(), "Bo".to_owned()]),
        ["Hello, Ann", "Hello, Bo"]
    );
}
