extern crate btleplug;

use btleplug::api::{Central, Characteristic, Peripheral, UUID};
#[cfg(target_os = "linux")]
use btleplug::bluez::{
    adapter::ConnectedAdapter, manager::Manager, peripheral::Peripheral as PeripheralStruct,
};
#[cfg(target_os = "macos")]
use btleplug::corebluetooth::{
    adapter::Adapter, manager::Manager, peripheral::Peripheral as PeripheralStruct,
};
#[cfg(target_os = "windows")]
use btleplug::winrtble::{
    adapter::Adapter, manager::Manager, peripheral::Peripheral as PeripheralStruct,
};
use std::thread;
use std::time::Duration;

// adapter retreival works differently depending on your platform right now.
// API needs to be aligned.

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn get_central(manager: &Manager) -> Adapter {
    let adapters = manager.adapters().unwrap();
    adapters.into_iter().nth(0).unwrap()
}

#[cfg(target_os = "linux")]
fn get_central(manager: &Manager) -> ConnectedAdapter {
    let adapters = manager.adapters().unwrap();
    let adapter = adapters.into_iter().nth(0).unwrap();
    adapter.connect().unwrap()
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
type BtCentral = Adapter;

#[cfg(target_os = "linux")]
type BtCentral = ConnectedAdapter;

const CONTROL_UUID: UUID = UUID::B128([
    0x8a, 0xf7, 0x15, 0x02, 0x9c, 0x00, 0x49, 0x8a, 0x24, 0x10, 0x8a, 0x33, 0x02, 0x00, 0xfa, 0x99,
]);

const STATUS_UUID: UUID = UUID::B128([
    0x8a, 0xf7, 0x15, 0x02, 0x9c, 0x00, 0x49, 0x8a, 0x24, 0x10, 0x8a, 0x33, 0x21, 0x00, 0xfa, 0x99,
]);
const POSITION_UUID: UUID = UUID::B128([
    0x8a, 0xf7, 0x15, 0x02, 0x9c, 0x00, 0x49, 0x8a, 0x24, 0x10, 0x8a, 0x33, 0x20, 0x00, 0xfa, 0x99,
]);

const UP: [u8; 2] = [0x47, 0x00];
const DOWN: [u8; 2] = [0x46, 0x00];
const STOP: [u8; 2] = [0xFF, 0x00];

pub const MIN_HEIGHT: f32 = 0.62;
pub const MAX_HEIGHT: f32 = 1.27;

/// convert desk response from bytes to meters
///
/// ```
/// assert_eq!(idasen::bytes_to_meters(&[0x64, 0x19, 0x00, 0x00]), idasen::MAX_HEIGHT);
/// assert_eq!(idasen::bytes_to_meters(&[0x00, 0x00, 0x00, 0x00]), idasen::MIN_HEIGHT);
/// assert_eq!(idasen::bytes_to_meters(&[0x51, 0x04, 0x00, 0x00]), 0.7305);
/// assert_eq!(idasen::bytes_to_meters(&[0x08, 0x08, 0x00, 0x00]), 0.8256);
/// ```
pub fn bytes_to_meters(bytes: &[u8]) -> f32 {
    let as_int = ((bytes[1] as u32) << 8) + bytes[0] as u32;
    (as_int as f32 / 10000.0) + MIN_HEIGHT
}

pub struct Idasen {
    manager: Manager,
    central: BtCentral,
    desk: PeripheralStruct,
    control_characteristic: Characteristic,
    status_characteristic: Characteristic,
}

impl Idasen {
    pub fn new() -> Self {
        let manager = Manager::new().unwrap();
        let central = get_central(&manager);
        central.start_scan().unwrap();

        thread::sleep(Duration::from_secs(2));

        let desk = central
            .peripherals()
            .into_iter()
            .find(|p| {
                p.properties()
                    .local_name
                    .iter()
                    .any(|name| name.contains("Desk"))
            })
            .unwrap();
        desk.connect().unwrap();

        let characteristics = desk.discover_characteristics().unwrap();
        let control_characteristic = characteristics
            .iter()
            .find(|characteristic| characteristic.uuid == CONTROL_UUID)
            .unwrap()
            .clone();
        let status_characteristic = characteristics
            .iter()
            .find(|characteristics| characteristics.uuid == STATUS_UUID)
            .unwrap()
            .clone();

        Self {
            manager,
            central,
            desk,
            control_characteristic,
            status_characteristic,
        }
    }

    pub fn up(&self) {
        self.desk.command(&self.control_characteristic, &UP);
    }

    pub fn down(&self) {
        self.desk.command(&self.control_characteristic, &DOWN);
    }

    pub fn stop(&self) {
        self.desk.command(&self.control_characteristic, &STOP);
    }

    pub fn height(&self) -> f32 {
        let response = self
            .desk
            .read_by_type(&self.status_characteristic, self.status_characteristic.uuid);
        bytes_to_meters(&response.unwrap())
    }
}
