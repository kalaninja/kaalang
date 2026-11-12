//! Loop forms: entry tests, exit tests, middle exits, and iterator traversal.

mod do_while;
// The endless flow is drawn but not called so this demonstration can finish.
#[allow(dead_code)]
mod endless;
mod for_each;
mod iterator_loop;
mod middle_exit;
mod while_loop;

pub(super) fn check() {
    assert_eq!(do_while::do_while(0), [0]);
    assert_eq!(do_while::do_while(7), [7]);
    assert_eq!(do_while::do_while(120), [1, 2, 0]);
    assert_eq!(do_while::do_while(1024), [1, 0, 2, 4]);

    assert_eq!(while_loop::while_loop(0, 0), 0);
    assert_eq!(while_loop::while_loop(12, 0), 12);
    assert_eq!(while_loop::while_loop(0, 12), 12);
    for (a, b, divisor) in [
        (12, 18, 6),
        (17, 5, 1),
        (270, 192, 6),
        (36, 36, 36),
        (48, 18, 6),
    ] {
        assert_eq!(while_loop::while_loop(a, b), divisor);
        assert_eq!(while_loop::while_loop(b, a), divisor);
    }

    for number in (0..200).chain([u64::from(u32::MAX), 1 << 40, u64::MAX]) {
        assert_eq!(middle_exit::middle_exit(number), number.isqrt());
    }

    for (values, expected) in [
        ([].as_slice(), 0),
        (&[7], 7),
        (&[1, 2, 3, 4], 10),
        (&[5, -5, 5], 5),
        (&[-2, 0, 5, 7], 10),
        (&[i32::MAX, i32::MAX], 2 * i64::from(i32::MAX)),
        (&[i32::MIN, i32::MIN], 2 * i64::from(i32::MIN)),
    ] {
        assert_eq!(for_each::for_each(values), expected);
        assert_eq!(iterator_loop::iterator_loop(values), expected);
    }
}
