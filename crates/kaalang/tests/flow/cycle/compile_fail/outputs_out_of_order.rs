use kaalang::kaa;

#[kaa]
fn invalid(mut items: Vec<String>) -> Option<String> {
    #[kaa::cycle("Find a nonempty item.")]
    let (exhausted, found) = loop {
        #[kaa::choice("Is another item available?")]
        #[kaa::case("Inspect the item.")]
        #[kaa::case("The collection is exhausted.")]
        let (item, exhausted) = |&mut items| match items.pop() {
            Some(item) => item,
            None => (),
        };

        #[kaa::question("Is the item empty?")]
        let (skip, keep) = |&item| item.is_empty();

        #[kaa::action("Provide the nonempty item.")]
        let found = |keep, item| item;

        |skip| continue;
    };

    #[kaa::action("Return the found item.")]
    let result = |found| Some(found);

    #[kaa::action("Report exhaustion.")]
    let result = |exhausted| None;

    |result| return result;
}

fn main() {}
