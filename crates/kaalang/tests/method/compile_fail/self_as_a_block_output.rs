use kaalang::kaa;

struct Rectangle {
    width: u32,
}

impl Rectangle {
    #[kaa]
    fn invalid(&self) -> u32 {
        #[kaa::action("Produce a wire named after the receiver.")]
        let self = |&self| self.width;

        |self| return self;
    }
}

fn main() {}
