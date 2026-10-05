//! Traffic-light states in an endless cycle. No route completes the flow.

use kaalang::kaalang;

#[allow(dead_code)]
enum Light {
    Red,
    Amber,
    Green,
}

#[allow(dead_code)]
#[kaalang]
fn endless(mut showing: Light) -> ! {
    #[cycle("🚦 Keep changing the traffic light.")]
    loop {
        #[choice("Which light is on now?")]
        #[case("🔴 Red.")]
        #[case("🟡 Amber.")]
        #[case("🟢 Green.")]
        let (red, amber, green) = |&showing| match *showing {
            Light::Red => (),
            Light::Amber => (),
            Light::Green => (),
        };

        #[action("🚗 Switch to green so traffic can go.")]
        let changed = |red, &mut showing| *showing = Light::Green;

        #[action("🛑 Switch to red so traffic stops.")]
        let changed = |amber, &mut showing| *showing = Light::Red;

        #[action("⚠️ Switch to amber to warn traffic to stop.")]
        let changed = |green, &mut showing| *showing = Light::Amber;

        |changed| continue;
    }
}
