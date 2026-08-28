use contour::contour;

#[contour]
fn invalid(secret: u32, trigger: ()) -> u32 {
    #[action("Read an internal wire name.")]
    |trigger| -> output { __contour_wire_0 };
}

fn main() {}
