use kaalang::kaalang;

#[kaalang]
fn kmp_search(text: &[u8], pattern: &[u8]) -> Option<usize> {
    #[call("Build the pattern's prefix table.")]
    let prefix = |pattern| prefix_table(pattern);

    #[action("Start at the first byte with no matched prefix.")]
    let (mut position, mut matched) = || (0usize, 0usize);

    #[choice("Can comparison begin?")]
    #[case("Both inputs contain bytes.")]
    #[case("An input is empty.")]
    let (compare, finish) = |text, pattern| match (text.is_empty(), pattern.is_empty()) {
        (false, false) => (),
        (_, empty_pattern) => empty_pattern.then_some(0usize),
    };

    #[stage("Compare the current bytes.")]
    let (step, retry) = |compare| {
        #[choice("Do the bytes match?")]
        #[case("Extend the matched prefix.")]
        #[case("Try a shorter prefix.")]
        let (step, retry) =
            |text, pattern, position, matched| match text[position] == pattern[matched] {
                true => matched + 1,
                false => (),
            };
    };

    #[stage("Advance through the text.")]
    let (compare, finish) = |step| {
        #[action("Consume one byte and record the matched length.")]
        |step, &mut position, &mut matched| {
            *position += 1;
            *matched = step;
        };

        #[choice("Can the search continue?")]
        #[case("Compare the next byte.")]
        #[case("Return the match or exhaustion.")]
        let (compare, finish) = |text, pattern, position, matched| match (
            matched == pattern.len(),
            position == text.len(),
        ) {
            (false, false) => (),
            (found, _) => found.then_some(position - matched),
        };
    };

    #[stage("Fall back to a shorter prefix.")]
    let (compare, step) = |retry| {
        #[choice("Is any prefix still matched?")]
        #[case("Follow the prefix table.")]
        #[case("Skip the unmatched text byte.")]
        let (shorten, step) = |matched| match matched {
            length if length > 0 => (),
            _ => 0usize,
        };

        #[action("Shorten the prefix; keep the text position.")]
        let compare = |shorten, &prefix, &mut matched| {
            *matched = prefix[*matched - 1];
        };
    };

    #[stage("Return the first match.")]
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
    #[action("Initialize prefix lengths; the first byte has no proper prefix.")]
    let (mut prefix, mut position, mut matched) =
        |pattern| (vec![0usize; pattern.len()], 1usize, 0usize);

    #[cycle("Compute the prefix table from left to right.")]
    let finished = {
        #[question("Is the table complete?")]
        let (finished, next) = |position, pattern| position >= pattern.len();

        #[cycle("Find a prefix that the next byte can extend.")]
        let settled = |next| {
            #[question("Must the prefix shrink?")]
            let (shrink, settled) =
                |pattern, position, matched| matched > 0 && pattern[position] != pattern[matched];

            #[action("Follow the previous prefix length.")]
            |shrink, &prefix, &mut matched| {
                *matched = prefix[*matched - 1];
            };

            |shrink| continue;
        };

        #[action("Extend a matching prefix, save its length, and advance.")]
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
