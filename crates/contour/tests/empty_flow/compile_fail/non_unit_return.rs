use contour::contour;

#[contour]
fn invalid() -> u8 {
    #[end]
    || {};
}

fn main() {}
