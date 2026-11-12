use kaalang::kaa;

struct Owned;

impl Owned {
    #[kaa]
    fn receiver_entry(self) -> Self {
        #[kaa::stage("Return the receiver.")]
        |self| {
            |self| return self;
        };
    }
}

fn main() {}
