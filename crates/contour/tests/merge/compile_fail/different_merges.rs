use contour::contour;

#[contour]
fn invalid(value: u8) -> u8 {
    #[choice("Select a route.")]
    #[case("Take route zero.")]
    #[case("Take route one.")]
    #[case("Take route two.")]
    #[case("Take route three.")]
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

    #[action("Return the low route.")]
    |low| -> low_result { low };

    #[action("Return the high route.")]
    |high| -> high_result { high };
}

fn main() {}
