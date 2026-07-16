use rusb::{Device, DeviceHandle, Error as UsbError, GlobalContext, devices};
use std::time::Duration;

const DATA_INTERFACE: u8 = 1;
const MODE_INTERFACE: u8 = 2;
const DATA_ENDPOINT: u8 = 0x84;
const ACTIVE_CONFIGURATION: u8 = 1;

// Complete 10moons-probe sequence for the Gotop/SZ PENG YI T501
// (08f2:6811). Sending only the second report made startup intermittent.
const FULL_AREA_REPORTS: [[u8; 8]; 4] = [
    [0x08, 0x04, 0x1d, 0x01, 0xff, 0xff, 0x06, 0x2e],
    [0x08, 0x03, 0x00, 0xff, 0xf0, 0x00, 0xff, 0xf0],
    [0x08, 0x06, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x08, 0x03, 0x00, 0xff, 0xf0, 0x00, 0xff, 0xf0],
];

pub struct PhysicalDevice {
    handle: DeviceHandle<GlobalContext>,
    detached_interfaces: Vec<u8>,
}

impl PhysicalDevice {
    /// Initialize the tablet with the same two-stage sequence used by the
    /// original DIGImend probe and T501 reader:
    ///
    /// 1. Send all four full-area reports through interface 2 and close it.
    /// 2. Reopen, reset, detach interfaces 0..2, apply configuration 1, and
    ///    claim the two HID interfaces before reading endpoint 0x84.
    pub fn connect(vid: u16, pid: u16) -> Result<Self, UsbError> {
        let device = Self::target_device(vid, pid)?;
        Self::send_full_area_reports(&device)?;
        Self::open_reader(&device)
    }

    /// Check that this exact tablet can be opened without changing its mode.
    /// This lets the service wait briefly for uaccess without repeating any
    /// feature reports, resets, or interface claims.
    pub fn check_access(vid: u16, pid: u16) -> Result<(), UsbError> {
        let device = Self::target_device(vid, pid)?;
        drop(device.open()?);
        Ok(())
    }

    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, UsbError> {
        self.handle
            .read_interrupt(DATA_ENDPOINT, buffer, Duration::from_secs(1))
    }

    fn send_full_area_reports(device: &Device<GlobalContext>) -> Result<(), UsbError> {
        let mut handle = device.open()?;
        handle.set_auto_detach_kernel_driver(true)?;
        handle.claim_interface(MODE_INTERFACE)?;

        for report in FULL_AREA_REPORTS {
            handle.write_control(
                0x21,
                0x09,
                0x0308,
                u16::from(MODE_INTERFACE),
                &report,
                Duration::from_millis(250),
            )?;
        }

        handle.release_interface(MODE_INTERFACE)?;
        Ok(())
    }

    fn open_reader(device: &Device<GlobalContext>) -> Result<Self, UsbError> {
        let mut handle = device.open()?;
        handle.reset()?;

        let detached_interfaces = Self::detach_kernel_interfaces(&mut handle)?;

        let setup_result = (|| {
            handle.set_active_configuration(ACTIVE_CONFIGURATION)?;
            handle.claim_interface(DATA_INTERFACE)?;
            if let Err(error) = handle.claim_interface(MODE_INTERFACE) {
                let _ = handle.release_interface(DATA_INTERFACE);
                return Err(error);
            }
            Ok(())
        })();

        if let Err(error) = setup_result {
            Self::reattach_kernel_interfaces(&mut handle, &detached_interfaces);
            return Err(error);
        }

        Ok(Self {
            handle,
            detached_interfaces,
        })
    }

    fn detach_kernel_interfaces(
        handle: &mut DeviceHandle<GlobalContext>,
    ) -> Result<Vec<u8>, UsbError> {
        let mut detached = Vec::new();

        for interface in 0..=MODE_INTERFACE {
            let active = match handle.kernel_driver_active(interface) {
                Ok(active) => active,
                Err(error) => {
                    Self::reattach_kernel_interfaces(handle, &detached);
                    return Err(error);
                }
            };

            if active {
                if let Err(error) = handle.detach_kernel_driver(interface) {
                    Self::reattach_kernel_interfaces(handle, &detached);
                    return Err(error);
                }
                detached.push(interface);
            }
        }

        Ok(detached)
    }

    fn reattach_kernel_interfaces(handle: &mut DeviceHandle<GlobalContext>, interfaces: &[u8]) {
        for &interface in interfaces.iter().rev() {
            let _ = handle.attach_kernel_driver(interface);
        }
    }

    fn target_device(vid: u16, pid: u16) -> Result<Device<GlobalContext>, UsbError> {
        devices()?
            .iter()
            .find(|device| {
                device.device_descriptor().is_ok_and(|descriptor| {
                    descriptor.vendor_id() == vid && descriptor.product_id() == pid
                })
            })
            .ok_or(UsbError::NoDevice)
    }
}

impl Drop for PhysicalDevice {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(MODE_INTERFACE);
        let _ = self.handle.release_interface(DATA_INTERFACE);
        Self::reattach_kernel_interfaces(&mut self.handle, &self.detached_interfaces);
    }
}

#[cfg(test)]
mod tests {
    use super::{DATA_ENDPOINT, DATA_INTERFACE, FULL_AREA_REPORTS, MODE_INTERFACE};

    #[test]
    fn full_area_sequence_matches_digimend_probe() {
        assert_eq!(
            FULL_AREA_REPORTS,
            [
                [0x08, 0x04, 0x1d, 0x01, 0xff, 0xff, 0x06, 0x2e],
                [0x08, 0x03, 0x00, 0xff, 0xf0, 0x00, 0xff, 0xf0],
                [0x08, 0x06, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
                [0x08, 0x03, 0x00, 0xff, 0xf0, 0x00, 0xff, 0xf0],
            ]
        );
    }

    #[test]
    fn reader_uses_full_area_interface_and_endpoint() {
        assert_eq!(DATA_INTERFACE, 1);
        assert_eq!(MODE_INTERFACE, 2);
        assert_eq!(DATA_ENDPOINT, 0x84);
    }
}
