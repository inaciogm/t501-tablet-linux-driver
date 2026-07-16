mod physical_device;
mod virtual_device;

use rusb::Error as UsbError;
use signal_hook::consts::signal::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::flag::register;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

use physical_device::PhysicalDevice;
use virtual_device::{DeviceDispatcher, RawDataReader};

const VID: u16 = 0x08f2;
const PID: u16 = 0x6811;
const MIN_REPORT_LENGTH: usize = 13;
const ACCESS_WAIT: Duration = Duration::from_secs(5);
const ACCESS_RETRY: Duration = Duration::from_millis(100);

fn main() -> Result<(), Box<dyn Error>> {
    let stop = Arc::new(AtomicBool::new(false));
    for signal in [SIGINT, SIGTERM, SIGQUIT] {
        register(signal, Arc::clone(&stop))?;
    }

    if !wait_for_tablet_access(&stop)? {
        println!("Tomate MTM-1106 is not connected; nothing to do.");
        return Ok(());
    }

    if stop.load(Ordering::Relaxed) {
        return Ok(());
    }

    // Initialization is deliberately attempted once per device connection.
    // A later USB add event starts a fresh process after reconnection.
    let mut tablet = PhysicalDevice::connect(VID, PID)?;
    run_connected(&mut tablet, &stop);
    println!("Tablet reader exited; the USB port is free.");
    Ok(())
}

fn wait_for_tablet_access(stop: &AtomicBool) -> Result<bool, UsbError> {
    let deadline = Instant::now() + ACCESS_WAIT;

    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(false);
        }

        match PhysicalDevice::check_access(VID, PID) {
            Ok(()) => return Ok(true),
            Err(UsbError::NoDevice) => return Ok(false),
            Err(UsbError::Access) if Instant::now() < deadline => sleep(ACCESS_RETRY),
            Err(error) => return Err(error),
        }
    }
}

fn run_connected(tablet: &mut PhysicalDevice, stop: &AtomicBool) {
    let mut reader = RawDataReader::new();
    let mut dispatcher = None;

    println!("Tomate MTM-1106 initialized in full-area mode.");

    while !stop.load(Ordering::Relaxed) {
        reader.data.fill(0);
        match tablet.read(&mut reader.data) {
            Ok(length) if length >= MIN_REPORT_LENGTH => {
                if dispatcher.is_none() {
                    match DeviceDispatcher::new() {
                        Ok(device) => {
                            dispatcher = Some(device);
                            println!("Pen input is ready.");
                        }
                        Err(error) => {
                            eprintln!("Could not create virtual pen: {error}");
                            return;
                        }
                    }
                }

                if let Some(device) = dispatcher.as_mut()
                    && let Err(error) = device.dispatch(&reader)
                {
                    eprintln!("Could not emit tablet input: {error}");
                    return;
                }
            }
            Ok(length) => eprintln!("Ignoring short tablet report ({length} bytes)."),
            Err(UsbError::Timeout) => {}
            Err(error) => {
                eprintln!("Tomate MTM-1106 disconnected ({error}).");
                return;
            }
        }
    }
}
