use kaalang::kaalang;

struct Rectangle {
    width: u32,
}

impl Rectangle {
    #[kaalang]
    fn invalid(self) -> u32 {
        #[action("Read the width through a borrowing capture.")]
        let end = |&self| self.width;

        |end| return end;
    }
}

fn main() {}
