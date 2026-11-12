//! Two sorting algorithms applied to the same input.

mod bubble_sort;
mod quick_sort;

pub(super) fn check() {
    for values in [
        [].as_slice(),
        &[7],
        &[1, 2, 3, 4],
        &[4, 3, 2, 1],
        &[2, 1, 3, 4],
        &[1, 3, 2, 4],
        &[2, 3, 1, 4],
        &[3, -1, 3, 0, -1],
        &[5, 5, 5],
        &[-8, 12, -8, 0, 7, -3],
        &[5, 1, 4, 2, 8],
    ] {
        let mut expected = values.to_vec();
        expected.sort_unstable();

        let mut sorted = values.to_vec();
        bubble_sort::bubble_sort(&mut sorted);
        assert_eq!(sorted, expected);
    }

    for mut input in [
        vec![],
        vec![4],
        vec![3, 1, 3, 2, 1],
        vec![1, 2, 3],
        vec![3, 2, 1],
        vec![7; 256],
        (0..1024).collect(),
        (0..1024).rev().collect(),
        vec![5, 1, 4, 2, 8],
    ] {
        let mut expected = input.clone();
        expected.sort_unstable();
        quick_sort::quick_sort(&mut input);
        assert_eq!(input, expected);
    }

    let mut owned = [String::from("z"), String::from("a"), String::from("m")];
    let mut expected = owned.clone();
    expected.sort();
    quick_sort::quick_sort(&mut owned);
    assert_eq!(owned, expected);

    let mut partial = [99, 4, 1, 3, 2, -99];
    quick_sort::quick_sort(&mut partial[1..5]);
    assert_eq!(partial, [99, 1, 2, 3, 4, -99]);

    for length in 0..=7 {
        for mut encoded in 0..3usize.pow(length) {
            let mut input = (0..length)
                .map(|_| {
                    let value = encoded % 3;
                    encoded /= 3;
                    value
                })
                .collect::<Vec<_>>();
            let mut expected = input.clone();
            expected.sort_unstable();
            quick_sort::quick_sort(&mut input);
            assert_eq!(input, expected);
        }
    }
}
