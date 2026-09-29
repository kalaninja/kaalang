use kaalang::kaalang;

struct Owned;

impl Owned {
    #[kaalang]
    fn receiver_entry(self) -> Self {
        #[stage("Return the receiver.")]
        |self| {
            |self| return self;
        };
    }
}

fn main() {}
