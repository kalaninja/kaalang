use kaalang::kaa;

struct Values {
    values: Vec<u32>,
}

impl Values {
    #[kaa]
    fn invalid(&self) {
        #[kaa::cycle("Visit every value.")]
        || for value in self.values.iter() {
            #[kaa::action("Use the value.")]
            |value| drop(value);
        };

        return;
    }
}

fn main() {}
