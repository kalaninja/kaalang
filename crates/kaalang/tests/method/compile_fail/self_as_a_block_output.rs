use kaalang::kaalang;

struct Rectangle {
    width: u32,
}

impl Rectangle {
    #[kaalang]
    fn invalid(&self) -> u32 {
        #[action("Produce a wire named after the receiver.")]
        let self = |&self| self.width;

        |self| return self;
    }
}

fn main() {}
