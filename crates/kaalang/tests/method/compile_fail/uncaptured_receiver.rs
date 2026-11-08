use kaalang::kaa;

struct Rectangle {
    width: u32,
}

impl Rectangle {
    #[kaa]
    fn invalid(&self, scale: u32) -> u32 {
        #[kaa::action("Scale a constant instead of the receiver.")]
        let end = |scale| scale * 2;

        |end| return end;
    }
}

fn main() {}
