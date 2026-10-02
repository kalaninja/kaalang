use kaalang::kaalang;

#[kaalang]
fn invalid(mut items: Vec<String>) -> Option<String> {
    #[cycle("Find a nonempty item.")]
    let (exhausted, found) = {
        #[choice("Is another item available?")]
        #[case("Inspect the item.")]
        #[case("The collection is exhausted.")]
        let (item, exhausted) = |&mut items| match items.pop() {
            Some(item) => item,
            None => (),
        };

        #[question("Is the item empty?")]
        let (skip, keep) = |&item| item.is_empty();

        #[action("Provide the nonempty item.")]
        let found = |keep, item| item;

        |skip| continue;
    };

    #[action("Return the found item.")]
    let result = |found| Some(found);

    #[action("Report exhaustion.")]
    let result = |exhausted| None;

    |result| return result;
}

fn main() {}
