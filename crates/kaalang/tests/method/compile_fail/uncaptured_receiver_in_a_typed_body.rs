use kaalang::kaa;

struct Rectangle {
    width: u32,
}

impl Rectangle {
    #[kaa]
    fn invalid(self: &Self, scale: u32) -> u32 {
        #[kaa::action("Read the width properly.")]
        let read = |&self| self.width;

        #[kaa::action("Read the receiver without capturing it.")]
        let end = |read, scale| read * scale + self.width;

        |end| return end;
    }
}

fn main() {}
