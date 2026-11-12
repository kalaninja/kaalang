use kaalang::kaa;

#[kaa]
fn invalid(mut items: Vec<String>) -> Option<String> {
    #[kaa::cycle("Take the next item.")]
    let taken = loop {
        #[kaa::action("Take an item.")]
        let taken = |&mut items| items.pop();

        #[kaa::question("Is it worth logging?")]
        let (log, quiet) = |&taken| taken.is_some();

        #[kaa::action("Log it.")]
        |log, &taken| println!("{taken:?}");

        #[kaa::action("Stay quiet.")]
        |quiet| {};
    };

    |taken| return taken;
}

fn main() {}
