use contour::contour;

#[contour]
fn cross_level_merge(outer: bool, inner: bool) -> u32 {
    #[question("Take the outer-left branch?")]
    |outer| -> (left, right) { outer };

    #[question("Take the inner-left branch?")]
    |left, inner| -> (inner_left, inner_terminal) { inner };

    #[action("Build the first merge input.")]
    |inner_left| -> first { 1 };

    #[action("End the other inner branch.")]
    |inner_terminal| -> inner_result { 2 };

    #[action("Build the second merge input.")]
    |right| -> second { 3 };

    #[merge]
    |first, second| -> selected {};

    #[action("Return the merged value.")]
    |selected| -> result { selected }
}

fn main() {}
