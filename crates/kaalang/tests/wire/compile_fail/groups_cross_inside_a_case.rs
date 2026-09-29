use kaalang::kaalang;

// The left marker takes the first two sources and the third source's left
// route; the right marker takes the second source and both third-source
// routes. By case the groups nest, but they cross inside the third source.
#[kaalang]
fn invalid(value: u8, split: bool) -> u8 {
    #[choice("Which source?")]
    #[case("The first source.")]
    #[case("The second source.")]
    #[case("The third source.")]
    let (first, second, third) = |value| match value {
        0 => (),
        1 => (),
        _ => (),
    };

    #[question("Does the third source take both markers?")]
    let (left, right) = |third, split| split;

    #[action("Mark the first source.")]
    let (_left, end) = |first| ((), 1);

    #[action("Mark the second source with both markers.")]
    let (_left, _right, end) = |second| ((), (), 2);

    #[action("Mark the third source on the left with both markers.")]
    let (_left, _right, end) = |left| ((), (), 3);

    #[action("Mark the third source on the right.")]
    let (_right, end) = |right| ((), 4);

    |end| return end;
}

fn main() {}
