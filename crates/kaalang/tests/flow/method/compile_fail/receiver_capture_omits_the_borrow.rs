use kaalang::kaa;

struct Rectangle {
    width: u32,
}

impl Rectangle {
    #[kaa]
    fn invalid(&self) -> u32 {
        #[kaa::action("Read the width through a value capture.")]
        let end = |self| self.width;

        |end| return end;
    }
}

fn main() {}
