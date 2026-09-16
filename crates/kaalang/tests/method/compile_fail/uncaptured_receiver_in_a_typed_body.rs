use kaalang::kaalang;

struct Rectangle {
    width: u32,
}

impl Rectangle {
    #[kaalang]
    fn invalid(self: &Self, scale: u32) -> u32 {
        #[action("Read the width properly.")]
        let read = |&self| self.width;

        #[action("Read the receiver without capturing it.")]
        let end = |read, scale| read * scale + self.width;

        |end| return end;
    }
}

fn main() {}
