use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[question("Is the input large?")]
    |&input| -> (large, small) { *input > 10 };

    #[action("Handle the large input.")]
    |large, &input| -> from_large { *input };

    #[action("Handle the small input.")]
    |small, &input| -> from_small { *input };

    #[action("Merge both branches.")]
    |from_large, from_small| -> merged { from_large + from_small };
}

fn main() {}
