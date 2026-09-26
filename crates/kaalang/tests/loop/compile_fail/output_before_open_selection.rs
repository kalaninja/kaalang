use kaalang::kaalang;

#[kaalang]
fn invalid(mut items: Vec<String>) -> Option<String> {
    #[cycle("Take the next item.")]
    let taken = {
        #[action("Take an item.")]
        let taken = |&mut items| items.pop();

        #[question("Is it worth logging?")]
        let (log, quiet) = |&taken| taken.is_some();

        #[action("Log it.")]
        |log, &taken| println!("{taken:?}");

        #[action("Stay quiet.")]
        |quiet| {};
    };

    |taken| return taken;
}

fn main() {}
