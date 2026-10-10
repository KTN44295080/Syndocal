//! macOS serial-device generation and opened-descriptor verification.
//! IOKit is queried only during discovery/open, never in the DMX frame loop.

pub(super) fn valid_instance(port: &str, instance: &str) -> bool {
    (port.starts_with("/dev/cu.")
        || port
            .strip_prefix("usb-ftdi://")
            .is_some_and(|id| instance.strip_prefix("ioreg:") == Some(id)))
        && instance.strip_prefix("ioreg:").is_some_and(|value| {
            value.len() == 16
                && value.bytes().all(|byte| byte.is_ascii_hexdigit())
                && u64::from_str_radix(value, 16).is_ok_and(|id| id != 0)
        })
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Binding {
    pub port_name: String,
    pub instance_id: String,
    pub device_number: u64,
    pub usb_registry_id: u64,
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn verify_open(
    expected: &str,
    before: &Binding,
    opened_device_number: u64,
    after: &Binding,
) -> Result<(), String> {
    if !valid_instance(&before.port_name, expected)
        || before.instance_id != expected
        || before != after
        || before.device_number != opened_device_number
    {
        return Err("USB-DMX device changed while opening or the descriptor belongs to another device; refresh and reselect".into());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
mod native {
    use super::Binding;
    use core_foundation::{
        base::{CFType, TCFType},
        number::CFNumber,
        string::CFString,
    };
    use core_foundation_sys::base::kCFAllocatorDefault;
    use io_kit_sys::{
        IOIteratorNext, IOObjectConformsTo, IOObjectRelease, IORegistryEntryCreateCFProperty,
        IORegistryEntryGetParentEntry, IORegistryEntryGetRegistryEntryID,
        IORegistryEntryIDMatching, IOServiceGetMatchingService, IOServiceGetMatchingServices,
        IOServiceMatching,
    };
    use std::{
        mem::MaybeUninit,
        os::unix::{
            fs::{FileTypeExt, MetadataExt},
            io::RawFd,
        },
    };

    struct IoObject(u32);
    impl Drop for IoObject {
        fn drop(&mut self) {
            unsafe {
                IOObjectRelease(self.0);
            }
        }
    }

    fn string_property(entry: u32, key: &str) -> Option<String> {
        let key = CFString::new(key);
        let raw = unsafe {
            IORegistryEntryCreateCFProperty(
                entry,
                key.as_concrete_TypeRef(),
                kCFAllocatorDefault,
                0,
            )
        };
        if raw.is_null() {
            return None;
        }
        let value = unsafe { CFType::wrap_under_create_rule(raw) };
        value.downcast::<CFString>().map(|value| value.to_string())
    }
    fn number_property(entry: u32, key: &str) -> Option<i64> {
        let key = CFString::new(key);
        let raw = unsafe {
            IORegistryEntryCreateCFProperty(
                entry,
                key.as_concrete_TypeRef(),
                kCFAllocatorDefault,
                0,
            )
        };
        if raw.is_null() {
            return None;
        }
        let value = unsafe { CFType::wrap_under_create_rule(raw) };
        value
            .downcast::<CFNumber>()
            .and_then(|value| value.to_i64())
    }
    fn registry_id(entry: u32) -> Result<u64, String> {
        let mut id = 0;
        if unsafe { IORegistryEntryGetRegistryEntryID(entry, &mut id) } != 0 || id == 0 {
            return Err("USB service has no registry generation".into());
        }
        Ok(id)
    }
    fn usb_parent_id(entry: u32) -> Result<u64, String> {
        let mut parent = 0;
        if unsafe { IORegistryEntryGetParentEntry(entry, c"IOService".as_ptr(), &mut parent) } != 0
        {
            return Err("Serial device has no USB ancestor".into());
        }
        let mut parent = IoObject(parent);
        // io-kit-sys declares className mutable; supply writable, NUL-terminated bytes.
        let mut usb_class = *b"IOUSBHostDevice\0";
        for _ in 0..32 {
            if unsafe { IOObjectConformsTo(parent.0, usb_class.as_mut_ptr().cast()) } != 0 {
                return registry_id(parent.0);
            }
            let mut next = 0;
            if unsafe { IORegistryEntryGetParentEntry(parent.0, c"IOService".as_ptr(), &mut next) }
                != 0
            {
                break;
            }
            parent = IoObject(next);
        }
        Err("Serial device has no verifiable USB ancestor".into())
    }
    pub(crate) fn enumerate_ftdi() -> Result<Vec<protocol::SerialPortSummary>, String> {
        let mut iterator = 0;
        if unsafe {
            IOServiceGetMatchingServices(
                0,
                IOServiceMatching(c"IOUSBHostDevice".as_ptr()),
                &mut iterator,
            )
        } != 0
            || iterator == 0
        {
            return Err("IOKit USB enumeration failed".into());
        }
        let iterator = IoObject(iterator);
        let mut devices = Vec::new();
        loop {
            let entry = unsafe { IOIteratorNext(iterator.0) };
            if entry == 0 {
                break;
            }
            let entry = IoObject(entry);
            if number_property(entry.0, "idVendor") != Some(0x0403)
                || number_property(entry.0, "idProduct") != Some(0x6001)
            {
                continue;
            }
            let id = registry_id(entry.0)?;
            devices.push(protocol::SerialPortSummary {
                name: format!("usb-ftdi://{id:016x}"),
                port_type: "USB FT232R (macOS native)".into(),
                usb_vid: Some(0x0403),
                usb_pid: Some(0x6001),
                serial_number: string_property(entry.0, "USB Serial Number"),
                manufacturer: string_property(entry.0, "USB Vendor Name"),
                product: string_property(entry.0, "USB Product Name"),
                windows_device_instance_id: None,
                macos_device_instance_id: Some(format!("ioreg:{id:016x}")),
                recommended_protocol: None, // FTDI alone never proves that the adapter is DMX.
            });
        }
        Ok(devices)
    }
    pub(crate) fn resolve(port_name: &str) -> Result<Binding, String> {
        if let Some(id) = port_name.strip_prefix("usb-ftdi://") {
            let id =
                u64::from_str_radix(id, 16).map_err(|_| "Invalid native FTDI registry identity")?;
            let service = unsafe { IOServiceGetMatchingService(0, IORegistryEntryIDMatching(id)) };
            if service == 0 {
                return Err(
                    "Selected native FTDI USB interface is absent; refresh and reselect".into(),
                );
            }
            let service = IoObject(service);
            if registry_id(service.0)? != id
                || number_property(service.0, "idVendor") != Some(0x0403)
                || number_property(service.0, "idProduct") != Some(0x6001)
            {
                return Err("Selected USB interface is not the approved FT232R generation".into());
            }
            return Ok(Binding {
                port_name: port_name.into(),
                instance_id: format!("ioreg:{id:016x}"),
                device_number: 0,
                usb_registry_id: id,
            });
        }
        if !port_name.starts_with("/dev/cu.") {
            return Err("macOS USB-DMX requires a /dev/cu.* callout port".into());
        }
        let mut iterator = 0;
        // IOServiceGetMatchingServices consumes its matching dictionary.
        let status = unsafe {
            IOServiceGetMatchingServices(
                0,
                IOServiceMatching(c"IOSerialBSDClient".as_ptr()),
                &mut iterator,
            )
        };
        if status != 0 || iterator == 0 {
            return Err(format!("IOKit serial enumeration failed ({status})"));
        }
        let iterator = IoObject(iterator);
        let key = CFString::new("IOCalloutDevice");
        let mut matches = Vec::new();
        loop {
            let entry = unsafe { IOIteratorNext(iterator.0) };
            if entry == 0 {
                break;
            }
            let entry = IoObject(entry);
            let property = unsafe {
                IORegistryEntryCreateCFProperty(
                    entry.0,
                    key.as_concrete_TypeRef(),
                    kCFAllocatorDefault,
                    0,
                )
            };
            if property.is_null() {
                continue;
            }
            let property = unsafe { CFType::wrap_under_create_rule(property) };
            let Some(path) = property.downcast::<CFString>() else {
                continue;
            };
            if path.to_string() != port_name {
                continue;
            }
            let mut registry_id = 0;
            let status = unsafe { IORegistryEntryGetRegistryEntryID(entry.0, &mut registry_id) };
            if status != 0 || registry_id == 0 {
                return Err("IOKit serial service has no valid registry identity".into());
            }
            let metadata = std::fs::symlink_metadata(port_name)
                .map_err(|e| format!("Unable to inspect serial device {port_name}: {e}"))?;
            if !metadata.file_type().is_char_device() {
                return Err("macOS USB-DMX path is not a real character device".into());
            }
            matches.push(Binding {
                port_name: port_name.into(),
                instance_id: format!("ioreg:{registry_id:016x}"),
                device_number: metadata.rdev(),
                usb_registry_id: usb_parent_id(entry.0)?,
            });
        }
        match matches.as_slice() {
            [binding] => Ok(binding.clone()),
            [] => Err("USB-DMX callout port is not present in IOKit; reconnect, close QLC+, and refresh serial devices".into()),
            _ => Err("USB-DMX callout port has ambiguous IOKit ownership".into()),
        }
    }

    pub(crate) fn opened_device_number(fd: RawFd) -> Result<u64, String> {
        let mut stat = MaybeUninit::<libc::stat>::uninit();
        if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } != 0 {
            return Err(format!(
                "Unable to verify opened USB-DMX descriptor: {}",
                std::io::Error::last_os_error()
            ));
        }
        let stat = unsafe { stat.assume_init() };
        if stat.st_mode & libc::S_IFMT != libc::S_IFCHR {
            return Err("Opened USB-DMX descriptor is not a character device".into());
        }
        Ok(stat.st_rdev as u64)
    }
}

#[cfg(target_os = "macos")]
pub(super) use native::{enumerate_ftdi, opened_device_number, resolve};

#[cfg(test)]
mod tests {
    use super::*;
    fn binding() -> Binding {
        Binding {
            port_name: "/dev/cu.usbserial-A".into(),
            instance_id: "ioreg:000000010000abcd".into(),
            device_number: 123,
            usb_registry_id: 456,
        }
    }

    #[test]
    fn exact_descriptor_and_iokit_generation_are_required() {
        let before = binding();
        assert!(verify_open(&before.instance_id, &before, 123, &before).is_ok());
        assert!(verify_open(&before.instance_id, &before, 456, &before).is_err());
        let mut changed = before.clone();
        changed.instance_id = "ioreg:000000010000abce".into();
        assert!(verify_open(&before.instance_id, &before, 123, &changed).is_err());
        changed = before.clone();
        changed.device_number = 456;
        assert!(verify_open(&before.instance_id, &before, 123, &changed).is_err());
        changed = before.clone();
        changed.port_name = "/dev/cu.usbserial-B".into();
        assert!(verify_open(&before.instance_id, &before, 123, &changed).is_err());
        assert!(verify_open("ioreg:000000010000ffff", &before, 123, &before).is_err());
    }

    #[test]
    fn missing_malformed_zero_or_dialin_identity_is_rejected() {
        for instance in [
            "",
            "ioreg:0000000000000000",
            "ioreg:1",
            "ioreg:00000000000000zz",
            "Windows-PnP",
        ] {
            assert!(!valid_instance("/dev/cu.usbserial-A", instance));
        }
        assert!(!valid_instance(
            "/dev/tty.usbserial-A",
            "ioreg:000000010000abcd"
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_iokit_missing_device_and_non_serial_descriptor_are_rejected() {
        use std::os::fd::AsRawFd;
        assert!(resolve("/dev/cu.syndocal-test-not-present").is_err());
        let file = std::fs::File::open("/dev/null").unwrap();
        // /dev/null is a character device, but cannot match any IOKit serial binding.
        assert!(verify_open(
            &binding().instance_id,
            &binding(),
            opened_device_number(file.as_raw_fd()).unwrap(),
            &binding()
        )
        .is_err());
    }
}
