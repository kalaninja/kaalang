use kaalang::kaalang;

struct Holder<T> {
    value: T,
}

impl<T: Clone> Holder<T> {
    #[kaalang]
    fn read_a_generic_receiver(&self) -> (T, T) {
        #[action("Copy the held value.")]
        let copied = |&self| self.value.clone();

        #[action("Pair it with another copy.")]
        let end = |&self, copied| (copied, self.value.clone());

        |end| return end;
    }
}

#[test]
fn a_method_uses_the_generics_of_its_impl() {
    let holder = Holder {
        value: "kaalang".to_owned(),
    };

    assert_eq!(
        holder.read_a_generic_receiver(),
        ("kaalang".to_owned(), "kaalang".to_owned())
    );
}
