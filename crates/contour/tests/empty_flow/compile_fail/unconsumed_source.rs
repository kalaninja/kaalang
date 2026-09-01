use contour::contour;

#[contour]
fn invalid(value: u8) {
    #[end]
    || {};
}

fn main() {}
