//! An endless cycle: nothing in the body breaks, so the flow never finishes
//! and the function returns `!`. A traffic light has no reason to stop.

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

        #[action("🚗 Let the waiting traffic go.")]
        |red, &mut showing| *showing = Light::Green;

        #[action("🛑 Stop the traffic.")]
        |amber, &mut showing| *showing = Light::Red;

        #[action("⚠️ Warn that the light is about to change.")]
        |green, &mut showing| *showing = Light::Amber;
    };
}
