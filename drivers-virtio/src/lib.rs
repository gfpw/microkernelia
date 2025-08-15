use core::ptr::{read_volatile, write_volatile};

pub mod pci {
    const PCI_CONFIG_ADDRESS: u32 = 0xCF8;
    const PCI_CONFIG_DATA: u32 = 0xCFC;

    pub fn read_config(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
        let address = (1 << 31)
            | ((bus as u32) << 16)
            | ((slot as u32) << 11)
            | ((func as u32) << 8)
            | ((offset as u32) & 0xFC);
        unsafe {
            write_volatile(PCI_CONFIG_ADDRESS as *mut u32, address);
            read_volatile(PCI_CONFIG_DATA as *const u32)
        }
    }

    pub fn find_virtio_devices() {
        for bus in 0..1 {
            for slot in 0..32 {
                let vendor = read_config(bus, slot, 0, 0) & 0xFFFF;
                if vendor == 0x1AF4 {
                    let device = (read_config(bus, slot, 0, 2) >> 16) & 0xFFFF;
                    crate::serial_println!("[virtio-pci] Encontrado dispositivo virtio: bus {} slot {} device {:04x}", bus, slot, device);
                }
            }
        }
    }
}

pub mod virtqueue {
    pub struct VirtQueue {
        pub desc: *mut u8,
        pub avail: *mut u8,
        pub used: *mut u8,
    }
    // Inicialización mínima (stub)
    pub fn init() {
        crate::serial_println!("[virtqueue] Inicializando virtqueues (stub)");
    }
}

pub mod vsock {
    pub fn init() {
        crate::serial_println!("[virtio-vsock] Inicializando driver vsock");
        super::pci::find_virtio_devices();
        super::virtqueue::init();
    }
}

pub mod fs {
    pub fn init() {
        crate::serial_println!("[virtio-fs] Inicializando driver virtio-fs");
        super::pci::find_virtio_devices();
        super::virtqueue::init();
    }
}
