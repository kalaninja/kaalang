use kaalang::kaa;

#[kaa("A device a session opens.")]
pub struct Device {
    #[kaa::value("The device's name.")]
    pub name: String,
}

// Rust compiles one of the two, and the diagrams draw the one the fixture
// target, 64-bit Linux, compiles: without its Windows field, in its Unix
// representation.
#[cfg(unix)]
#[kaa("An open device on Unix.")]
#[cfg_attr(unix, repr(C))]
pub struct Handle {
    #[kaa::refer(Device, "The device it opens.")]
    pub device: Device,
    #[kaa::value("The file descriptor.")]
    pub fd: i32,
    #[cfg(windows)]
    #[kaa::value("A raw handle, which Unix has none of.")]
    pub raw: isize,
}

#[cfg(not(unix))]
#[kaa("An open device on other systems.")]
pub struct Handle {
    #[kaa::refer(Device, "The device it opens.")]
    pub device: Device,
    #[kaa::value("The raw handle.")]
    pub raw: isize,
}

#[kaa("A session that holds a device open.")]
pub struct Session {
    #[kaa::refer(Handle, "The open device.")]
    pub handle: Handle,
    // Only on Unix does the log name a device.
    #[cfg_attr(unix, kaa::refer(Device, "The device it logs to."))]
    pub log: String,
}

#[test]
fn a_type_may_be_declared_once_for_each_target() {
    let device = Device {
        name: String::from("tty"),
    };
    #[cfg(unix)]
    let handle = Handle { device, fd: 3 };
    #[cfg(not(unix))]
    let handle = Handle { device, raw: 3 };
    let session = Session {
        handle,
        log: String::from("console"),
    };

    assert_eq!(session.handle.device.name, "tty");
    assert_eq!(session.log, "console");
    #[cfg(unix)]
    assert_eq!(session.handle.fd, 3);
    #[cfg(not(unix))]
    assert_eq!(session.handle.raw, 3);
}
