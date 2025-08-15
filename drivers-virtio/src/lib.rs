use core::ptr::{read_volatile, write_volatile};

pub mod pci {
    const PCI_CONFIG_ADDRESS: u32 = 0xCF8;
    const PCI_CONFIG_DATA: u32 = 0xCFC;

    #[derive(Debug, Clone, Copy)]
    pub struct VirtioDevice {
        pub bus: u8,
        pub slot: u8,
        pub func: u8,
        pub device_id: u16,
        pub bar0: u32,
    }

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

    pub fn find_virtio_devices_full() -> [Option<VirtioDevice>; 8] {
        let mut found: [Option<VirtioDevice>; 8] = [None, None, None, None, None, None, None, None];
        let mut idx = 0;
        for bus in 0..1 {
            for slot in 0..32 {
                let vendor = read_config(bus, slot, 0, 0) & 0xFFFF;
                if vendor == 0x1AF4 {
                    let device = (read_config(bus, slot, 0, 2) >> 16) & 0xFFFF;
                    let bar0 = read_config(bus, slot, 0, 0x10);
                    crate::serial_println!("[virtio-pci] Dispositivo virtio: bus {} slot {} dev {:04x} bar0 {:08x}", bus, slot, device, bar0);
                    if idx < found.len() {
                        found[idx] = Some(VirtioDevice {
                            bus, slot, func: 0, device_id: device as u16, bar0
                        });
                        idx += 1;
                    }
                }
            }
        }
        found
    }

    pub fn enable_bus_master(bus: u8, slot: u8) {
        let mut cmd = read_config(bus, slot, 0, 4);
        cmd |= 0x4; // Bus Master Enable
        unsafe {
            write_volatile(PCI_CONFIG_ADDRESS as *mut u32, (1 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11) | 4);
            write_volatile(PCI_CONFIG_DATA as *mut u32, cmd);
        }
    }
}

pub mod virtqueue {
    pub struct VirtQueue {
        pub desc: *mut u8,
        pub avail: *mut u8,
        pub used: *mut u8,
        pub size: u16,
    }

    pub fn setup_virtqueue(_dev: &super::pci::VirtioDevice, queue_idx: u16, queue_size: u16) -> VirtQueue {
        // En una implementación real, mapear MMIO/PIO y asignar memoria alineada para desc/avail/used
        crate::serial_println!("[virtqueue] Setup virtqueue idx {} size {} (stub)", queue_idx, queue_size);
        VirtQueue {
            desc: 0 as *mut u8,
            avail: 0 as *mut u8,
            used: 0 as *mut u8,
            size: queue_size,
        }
    }

    // Inicialización mínima (stub)
    pub fn init() {
        crate::serial_println!("[virtqueue] Inicializando virtqueues (stub)");
    }
}

pub mod vsock {
    pub fn init() {
        crate::serial_println!("[virtio-vsock] Inicializando driver vsock");
        let devs = super::pci::find_virtio_devices_full();
        for dev in devs.iter().flatten() {
            if dev.device_id == 0x1040 || dev.device_id == 0x105A { // 0x1040: vsock, 0x105A: modern vsock
                super::pci::enable_bus_master(dev.bus, dev.slot);
                let _vq = super::virtqueue::setup_virtqueue(dev, 0, 256); // ejemplo: cola 0, tamaño 256
            }
        }
    }
}

pub mod fs {
    pub fn init() {
        crate::serial_println!("[virtio-fs] Inicializando driver virtio-fs");
        let devs = super::pci::find_virtio_devices_full();
        for dev in devs.iter().flatten() {
            if dev.device_id == 0x1049 { // 0x1049: virtio-fs
                super::pci::enable_bus_master(dev.bus, dev.slot);
                let _vq = super::virtqueue::setup_virtqueue(dev, 0, 256); // ejemplo: cola 0, tamaño 256
            }
        }
    }
}
