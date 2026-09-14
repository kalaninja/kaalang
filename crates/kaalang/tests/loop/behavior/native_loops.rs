use kaalang::kaalang;

#[kaalang]
fn native_loops(limit: usize) -> usize {
    #[cycle("Run native Rust control flow once.")]
    let result = |limit| {
        #[action("Count with local Rust control flow.")]
        let count = |limit| {
            let mut count = 0;
            'counting: while count < limit {
                count += 1;
                for step in 0..2 {
                    if step == 0 {
                        continue;
                    }
                    if count == 2 {
                        break 'counting;
                    }
                    break;
                }
            }
            count
        };

        |count| break count;
    };

    |result| return result;
}

#[test]
fn native_while_and_transfers_remain_local_to_the_rust_body() {
    assert_eq!(native_loops(0), 0);
    assert_eq!(native_loops(1), 1);
    assert_eq!(native_loops(5), 2);
}
