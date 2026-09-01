use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[action("Use a future wire.")]
    |future| -> output { future };

    #[action("Declare the wire too late.")]
    |input| -> future { input };

    #[end]
    |output| {};
}

fn main() {}
