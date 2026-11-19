#![deny(unexpected_cfgs)]

use kaalang::kaa as derive;

#[derive]
struct Counter(u32);

impl Counter {
    #[allow(unexpected_cfgs)]
    #[kaalang::kaa]
    fn conditional_parameters(
        &self,
        #[r#cfg_attr(any(), cfg(inactive_backend(ignored)))]
        #[cfg(unix)]
        value: u8,
        #[r#cfg(not(unix))] value: u16,
        #[cfg_attr(all(), r#cfg_attr(all(), r#cfg(any())))] (_x, _y): (u8, u8),
        #[cfg_attr(all(), allow(unused_variables))]
        #[cfg(optional_backend)]
        _optional: u8,
    ) -> u32 {
        #[kaa::action("Read the active input.")]
        let result = |&self, value| self.0 + u32::from(value);
        |result| return result;
    }
}

#[test]
#[allow(unexpected_cfgs)]
fn only_active_parameters_provide_wires() {
    let result = Counter(2).conditional_parameters(
        21,
        #[cfg(optional_backend)]
        0,
    );
    assert_eq!(result, 23);
}
