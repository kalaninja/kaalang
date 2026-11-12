//! Traffic-light states in an endless cycle. No route completes the flow.

use kaalang::kaa;

enum Light {
    Red,
    Amber,
    Green,
}

#[kaa]
fn endless(mut showing: Light) -> ! {
    #[kaa::cycle("🚦 Keep changing the traffic light.")]
    loop {
        #[kaa::choice("Which light is on now?")]
        #[kaa::case("🔴 Red.")]
        #[kaa::case("🟡 Amber.")]
        #[kaa::case("🟢 Green.")]
        let (red, amber, green) = |&showing| match *showing {
            Light::Red => (),
            Light::Amber => (),
            Light::Green => (),
        };

        #[kaa::action("🚗 Switch to green so traffic can go.")]
        let changed = |red, &mut showing| *showing = Light::Green;

        #[kaa::action("🛑 Switch to red so traffic stops.")]
        let changed = |amber, &mut showing| *showing = Light::Red;

        #[kaa::action("⚠️ Switch to amber to warn traffic to stop.")]
        let changed = |green, &mut showing| *showing = Light::Amber;

        |changed| continue;
    }
}
