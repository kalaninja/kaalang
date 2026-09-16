use kaalang::kaalang;

struct Square {
    side: u32,
    area: u32,
}

impl Square {
    #[kaalang]
    fn build_a_self_value(side: u32) -> Self {
        #[action("Square the side.")]
        let area = |&side| side * side;

        #[action("Assemble the square.")]
        let end = |side, area| Self { side, area };

        |end| return end;
    }
}

#[test]
fn an_associated_function_names_its_own_type() {
    let square = Square::build_a_self_value(3);

    assert_eq!((square.side, square.area), (3, 9));
}
