use contour::contour;

#[contour]
fn invalid(value: u8) -> u8 {
    #[choice("Select a branch.")]
    #[case("Take branch zero.")]
    #[case("Take branch one.")]
    #[case("Take branch two.")]
    #[case("Take branch three.")]
    |value| -> (zero, one, two, three) {
        match value {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 3,
        }
    };

    #[merge]
    |zero, one| -> low {};

    #[merge]
    |two, three| -> high {};

    #[action("Return the low branch.")]
    |low| -> low_result { low };

    #[action("Return the high branch.")]
    |high| -> high_result { high };
}

fn main() {}
