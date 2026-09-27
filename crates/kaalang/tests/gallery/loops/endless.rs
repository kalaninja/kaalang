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
    #[cycle("🚦 Show the next colour, forever.")]
    {
        #[choice("Which colour is showing?")]
        #[case("🔴 Red.")]
        #[case("🟡 Amber.")]
        #[case("🟢 Green.")]
        let (red, amber, green) = |&showing| match *showing {
            Light::Red => (),
            Light::Amber => (),
            Light::Green => (),
        };

        #[action("🚗 Let traffic go.")]
        let changed = |red, &mut showing| *showing = Light::Green;

        #[action("🛑 Stop the traffic.")]
        let changed = |amber, &mut showing| *showing = Light::Red;

        #[action("⚠️ Prepare to stop.")]
        let changed = |green, &mut showing| *showing = Light::Amber;

        |changed| continue;
    };
}
