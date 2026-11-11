//! Linear-time byte search using a prefix table and three repeating stages.

use kaalang::kaa;

#[kaa]
fn kmp_search(text: &[u8], pattern: &[u8]) -> Option<usize> {
    #[kaa::call("Build a table for reusing matched beginnings of the pattern.")]
    let prefix = |pattern| prefix_table(pattern);

    #[kaa::action("Start at the first text byte with no pattern bytes matched.")]
    let (mut position, mut matched) = || (0usize, 0usize);

    #[kaa::question("Is the pattern empty?")]
    #[kaa::no("NO")]
    #[kaa::yes("YES")]
    let (nonempty, empty) = |pattern| pattern.is_empty();

    #[kaa::action("An empty pattern matches at the start of the text.")]
    let finish = |empty| Some(0usize);

    #[kaa::question("Can the pattern fit inside the text?")]
    #[kaa::yes("YES")]
    #[kaa::no("NO")]
    let (compare, too_long) = |nonempty, text, pattern| pattern.len() <= text.len();

    #[kaa::action("The pattern is longer than the text; report no match.")]
    let finish = |too_long| None;

    #[kaa::stage("Compare the next pattern byte.")]
    let (step, retry) = |compare| {
        #[kaa::action("Read the current text byte and the next unmatched pattern byte.")]
        let (text_byte, pattern_byte) =
            |text, pattern, position, matched| (text[position], pattern[matched]);

        #[kaa::question("Do these two bytes match?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (extend, retry) = |text_byte, pattern_byte| text_byte == pattern_byte;

        #[kaa::action("Count this byte as one more matched pattern byte.")]
        let step = |extend, matched| matched + 1;
    };

    #[kaa::stage("Move to the next text byte.")]
    let (compare, finish) = |step| {
        #[kaa::action("Move past this text byte and remember how many pattern bytes matched.")]
        |step, &mut position, &mut matched| {
            *position += 1;
            *matched = step;
        };

        #[kaa::question("Has the whole pattern matched?")]
        #[kaa::no("NO")]
        #[kaa::yes("YES")]
        let (remaining, found) = |pattern, matched| matched == pattern.len();

        #[kaa::action("Report where the matching part of the text begins.")]
        let finish = |found, position, matched| Some(position - matched);

        #[kaa::question("Are there more text bytes to read?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (compare, exhausted) = |remaining, text, position| position < text.len();

        #[kaa::action("The text has ended without a full match; report no match.")]
        let finish = |exhausted| None;
    };

    #[kaa::stage("Reuse a shorter match.")]
    let (compare, step) = |retry| {
        #[kaa::question("Have any bytes at the start of the pattern already matched?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (shorten, skip) = |matched| matched > 0;

        #[kaa::action("Use the table to keep a shorter matched beginning; retry this text byte.")]
        let compare = |shorten, &prefix, &mut matched| {
            *matched = prefix[*matched - 1];
        };

        #[kaa::action("Start a new match after this text byte.")]
        let step = |skip| 0usize;
    };

    #[kaa::stage("Return the search result.")]
    |finish| {
        |finish| return finish;
    };
}

#[kaa]
fn prefix_table(pattern: &[u8]) -> Vec<usize> {
    #[kaa::action("Set all prefix lengths to zero; no pattern beginning is matched yet.")]
    let (mut prefix, mut matched) = |pattern| (vec![0usize; pattern.len()], 0usize);

    #[kaa::cycle("Find how much of the pattern's beginning repeats at each ending.")]
    let finished = |pattern| {
        for position in 1..pattern.len() {
            #[kaa::action("Read the pattern byte at this position.")]
            let byte = |position, pattern| pattern[position];

            #[kaa::cycle("Extend the matched beginning or try a shorter one.")]
            let settled = |byte| loop {
                #[kaa::question("Does this byte match the next byte of the pattern's beginning?")]
                #[kaa::yes("YES")]
                #[kaa::no("NO")]
                let (extend, mismatch) = |byte, pattern, matched| byte == pattern[matched];

                #[kaa::action("Include this byte in the matched beginning.")]
                let settled = |extend, &mut matched| *matched += 1;

                #[kaa::question("Is there a matched beginning to shorten?")]
                #[kaa::no("NO")]
                #[kaa::yes("YES")]
                let (settled, shrink) = |mismatch, matched| matched > 0;

                #[kaa::action("Use the table to keep the next shorter matching beginning.")]
                |shrink, &prefix, &mut matched| {
                    *matched = prefix[*matched - 1];
                };

                |shrink| continue;
            };

            #[kaa::action("Save the matched length for this byte.")]
            |settled, &mut prefix, position, matched| prefix[position] = matched;
        }
    };

    |finished, prefix| return prefix;
}

pub(super) fn check() {
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

    for (pattern, expected) in [
        ("", vec![]),
        ("a", vec![0]),
        ("aaaaa", vec![0, 1, 2, 3, 4]),
        ("ababaca", vec![0, 0, 1, 2, 3, 0, 1]),
        ("aabaaab", vec![0, 1, 0, 1, 2, 2, 3]),
        ("abcabcd", vec![0, 0, 0, 1, 2, 3, 0]),
        ("ababd", vec![0, 0, 1, 2, 0]),
    ] {
        assert_eq!(prefix_table(pattern.as_bytes()), expected, "{pattern:?}");
    }
}
