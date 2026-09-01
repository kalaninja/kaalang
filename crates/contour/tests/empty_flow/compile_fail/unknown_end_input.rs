use contour::contour;

#[contour]
fn invalid() -> u8 {
    #[end]
    |unknown| {};
}

fn main() {}
