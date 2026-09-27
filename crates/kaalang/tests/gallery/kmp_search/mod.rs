//! Linear-time byte search using a prefix table and three repeating stages.

use kaalang::kaalang;

#[kaalang]
fn kmp_search(text: &[u8], pattern: &[u8]) -> Option<usize> {
    #[call("Build the prefix table.")]
    let prefix = |pattern| prefix_table(pattern);

    #[action("Start with no matched bytes.")]
    let (mut position, mut matched) = || (0usize, 0usize);

    #[question("Does a nonempty pattern fit?")]
    #[yes("YES")]
    #[no("NO")]
    let (compare, trivial) = |text, pattern| !pattern.is_empty() && pattern.len() <= text.len();

    #[action("Resolve the empty or oversized pattern.")]
    let finish = |trivial, pattern| pattern.is_empty().then_some(0usize);

    #[stage("Compare bytes.")]
    let (step, retry) = |compare| {
        #[question("Do the bytes match?")]
        #[yes("YES")]
        #[no("NO")]
        let (extend, retry) = |text, pattern, position, matched| text[position] == pattern[matched];

        #[action("Extend the matched prefix.")]
        let step = |extend, matched| matched + 1;
    };

    #[stage("Advance one byte.")]
    let (compare, finish) = |step| {
        #[action("Advance and save the matched length.")]
        |step, &mut position, &mut matched| {
            *position += 1;
            *matched = step;
        };

        #[question("Can the search continue?")]
        #[yes("YES")]
        #[no("NO")]
        let (compare, stopped) =
            |text, pattern, position, matched| matched < pattern.len() && position < text.len();

        #[action("Report the match or exhaustion.")]
        let finish = |stopped, pattern, position, matched| {
            (matched == pattern.len()).then_some(position - matched)
        };
    };

    #[stage("Try a shorter prefix.")]
    let (compare, step) = |retry| {
        #[question("Is any prefix still matched?")]
        #[yes("YES")]
        #[no("NO")]
        let (shorten, skip) = |matched| matched > 0;

        #[action("Follow the previous prefix length.")]
        let compare = |shorten, &prefix, &mut matched| {
            *matched = prefix[*matched - 1];
        };

        #[action("Skip the unmatched byte.")]
        let step = |skip| 0usize;
    };

    #[stage("Return the result.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn finds_the_first_match_with_empty_repeated_and_utf8_inputs() {
    for (text, pattern) in [
        ("", ""),
        ("", "a"),
        ("abc", ""),
        ("abc", "abcd"),
        ("abc", "abc"),
        ("abc", "bc"),
        ("abc", "d"),
        ("ababcabcabababd", "ababd"),
        ("aaaaab", "aaab"),
        ("ababababac", "ababac"),
        ("ababababab", "ababac"),
        ("one one one", "one"),
        ("\0a\0b", "\0b"),
        ("café déjà vu", "déjà"),
        ("🦀🦀🔎🦀", "🦀🔎"),
    ] {
        assert_eq!(
            kmp_search(text.as_bytes(), pattern.as_bytes()),
            text.find(pattern),
            "text={text:?}, pattern={pattern:?}",
        );
    }

    let text = format!("{}b", "a".repeat(4096));
    let pattern = format!("{}b", "a".repeat(128));
    assert_eq!(kmp_search(text.as_bytes(), pattern.as_bytes()), Some(3968));
}

#[test]
fn agrees_with_direct_search_for_all_short_binary_inputs() {
    let inputs = |maximum| {
        (0..=maximum)
            .flat_map(|length| {
                (0..1usize << length).map(move |bits| {
                    (0..length)
                        .map(|bit| u8::from(bits & (1 << bit) != 0))
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>()
    };
    let texts = inputs(7);
    let patterns = inputs(5);
    for text in &texts {
        for pattern in &patterns {
            let expected = if pattern.is_empty() {
                Some(0)
            } else {
                text.windows(pattern.len())
                    .position(|window| window == pattern)
            };
            assert_eq!(kmp_search(text, pattern), expected, "{text:?}, {pattern:?}");
        }
    }
}

#[kaalang]
fn prefix_table(pattern: &[u8]) -> Vec<usize> {
    #[action("Start with zero prefix lengths.")]
    let (mut prefix, mut position, mut matched) =
        |pattern| (vec![0usize; pattern.len()], 1usize, 0usize);

    #[cycle("Build prefixes from left to right.")]
    let finished = {
        #[question("Is the table complete?")]
        #[yes("YES")]
        #[no("NO")]
        let (finished, next) = |position, pattern| position >= pattern.len();

        #[cycle("Find an extendable prefix.")]
        let settled = |next| {
            #[question("Must the prefix shrink?")]
            #[yes("YES")]
            #[no("NO")]
            let (shrink, settled) =
                |pattern, position, matched| matched > 0 && pattern[position] != pattern[matched];

            #[action("Follow the previous prefix length.")]
            |shrink, &prefix, &mut matched| {
                *matched = prefix[*matched - 1];
            };

            |shrink| continue;
        };

        #[action("Extend if matched; save and advance.")]
        |settled, pattern, &mut prefix, &mut position, &mut matched| {
            if pattern[*position] == pattern[*matched] {
                *matched += 1;
            }
            prefix[*position] = *matched;
            *position += 1;
        };

        |settled| continue;
    };

    |finished, prefix| return prefix;
}

#[test]
fn builds_longest_proper_prefix_lengths() {
    for (pattern, expected) in [
        ("", vec![]),
        ("a", vec![0]),
        ("aaaaa", vec![0, 1, 2, 3, 4]),
        ("ababaca", vec![0, 0, 1, 2, 3, 0, 1]),
        ("aabaaab", vec![0, 1, 0, 1, 2, 2, 3]),
        ("abcabcd", vec![0, 0, 0, 1, 2, 3, 0]),
    ] {
        assert_eq!(prefix_table(pattern.as_bytes()), expected, "{pattern:?}");
    }
}
