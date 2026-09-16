use kaalang::kaalang;

struct Rectangle {
    width: u32,
}

impl Rectangle {
    #[kaalang]
    fn invalid(&self, scale: u32) -> u32 {
        #[action("Scale a constant instead of the receiver.")]
        let end = |scale| scale * 2;

        |end| return end;
    }
}

fn main() {}
