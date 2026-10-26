mod physical_device;
mod virtual_device;

use rusb::Error as UsbError;
use signal_hook::consts::signal::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::flag::register;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

use physical_device::PhysicalDevice;
use virtual_device::{DeviceDispatcher, RawDataReader};

const VID: u16 = 0x08f2;
const PID: u16 = 0x6811;
const MIN_REPORT_LENGTH: usize = 13;
fn main() -> Result<(), Box<dyn Error>> {
    let stop = Arc::new(AtomicBool::new(false));
    for signal in [SIGINT, SIGTERM, SIGQUIT] {
        register(signal, Arc::clone(&stop))?;
    }

    println!("Tomate MTM-1106 driver daemon started.");

    while !stop.load(Ordering::Relaxed) {
        match PhysicalDevice::connect(VID, PID) {
            Ok(mut tablet) => {
                run_connected(&mut tablet, &stop);
                println!("Tablet reader exited; the USB port is free.");
            }
            Err(UsbError::NoDevice) => {
                sleep(Duration::from_millis(1000));
            }
            Err(UsbError::Access) => {
                sleep(Duration::from_millis(500));
            }
            Err(error) => {
                eprintln!("Could not connect to Tomate MTM-1106: {error}. Retrying in 1s...");
                sleep(Duration::from_millis(1000));
            }
        }
    }

    println!("Tomate MTM-1106 driver daemon stopped.");
    Ok(())
}

fn run_connected(tablet: &mut PhysicalDevice, stop: &AtomicBool) {
    let mut reader = RawDataReader::new();

    println!("Tomate MTM-1106 initialized in full-area mode.");

    let mut dispatcher = match DeviceDispatcher::new() {
        Ok(device) => {
            println!("Pen input is ready.");
            device
        }
        Err(error) => {
            eprintln!("Could not create virtual pen: {error}");
            return;
        }
    };

    while !stop.load(Ordering::Relaxed) {
        reader.data.fill(0);
        match tablet.read(&mut reader.data) {
            Ok(length) if length >= MIN_REPORT_LENGTH => {
                if let Err(error) = dispatcher.dispatch(&reader) {
                    eprintln!("Could not emit tablet input: {error}");
                    return;
                }
            }
            Ok(length) => eprintln!("Ignoring short tablet report ({length} bytes)."),
            Err(UsbError::Timeout) => {
                if let Err(error) = dispatcher.pen_leave_proximity() {
                    eprintln!("Could not emit proximity leave: {error}");
                    return;
                }
            }
            Err(error) => {
                eprintln!("Tomate MTM-1106 disconnected ({error}).");
                return;
            }
        }
    }
}
