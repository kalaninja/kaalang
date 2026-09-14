use kaalang::kaalang;

#[kaalang]
fn invalid(text: String) -> ! {
    #[cycle("Capture the outer input.")]
    |text| {
        #[cycle("Omit the nested interface.")]
        || {
            #[action("Use the unforwarded input.")]
            |text| drop(text);
        };
    };
}

fn main() {}
