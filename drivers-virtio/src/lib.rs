use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{compiler_fence, Ordering};

#[repr(C, align(16))]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C, align(2))]
pub struct VirtqAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; 256],
}

#[repr(C, align(4))]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

#[repr(C, align(4))]
pub struct VirtqUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtqUsedElem; 256],
}

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
    use super::{VirtqDesc, VirtqAvail, VirtqUsed, VirtqUsedElem};

    pub struct VirtQueue {
        pub desc: *mut VirtqDesc,
        pub avail: *mut VirtqAvail,
        pub used: *mut VirtqUsed,
        pub size: u16,
    }

    pub fn setup_virtqueue(dev: &super::pci::VirtioDevice, queue_idx: u16, queue_size: u16) -> VirtQueue {
        // Asignar memoria alineada para desc/avail/used (aquí se asume memoria estática, en producción usar allocador)
        static mut DESC: [VirtqDesc; 256] = [VirtqDesc { addr: 0, len: 0, flags: 0, next: 0 }; 256];
        static mut AVAIL: VirtqAvail = VirtqAvail { flags: 0, idx: 0, ring: [0; 256] };
        static mut USED: VirtqUsed = VirtqUsed { flags: 0, idx: 0, ring: [VirtqUsedElem { id: 0, len: 0 }; 256] };
        VirtQueue {
            desc: unsafe { &mut DESC as *mut _ },
            avail: unsafe { &mut AVAIL as *mut _ },
            used: unsafe { &mut USED as *mut _ },
            size: queue_size,
        }
    }

    // Inicialización mínima (stub)
    pub fn init() {
        crate::serial_println!("[virtqueue] Inicializando virtqueues (stub)");
    }
}

pub mod vsock {
    use super::virtqueue::VirtQueue;
    static mut VSOCK_TX: Option<VirtQueue> = None;
    static mut VSOCK_RX: Option<VirtQueue> = None;

    pub fn init() {
        crate::serial_println!("[virtio-vsock] Inicializando driver vsock");
        let devs = super::pci::find_virtio_devices_full();
        for dev in devs.iter().flatten() {
            if dev.device_id == 0x1040 || dev.device_id == 0x105A {
                super::pci::enable_bus_master(dev.bus, dev.slot);
                // Setup TX y RX queues (ejemplo: idx 0 y 1, tamaño 256)
                let tx = super::virtqueue::setup_virtqueue(dev, 0, 256);
                let rx = super::virtqueue::setup_virtqueue(dev, 1, 256);
                unsafe {
                    VSOCK_TX = Some(tx);
                    VSOCK_RX = Some(rx);
                }
            }
        }
    }

    pub fn send(data: &[u8]) -> bool {
        unsafe {
            if let Some(ref mut tx) = VSOCK_TX {
                let desc = &mut *tx.desc;
                desc.addr = data.as_ptr() as u64;
                desc.len = data.len() as u32;
                desc.flags = 0;
                desc.next = 0;
                let avail = &mut *tx.avail;
                let idx = avail.idx as usize % tx.size as usize;
                avail.ring[idx] = 0;
                compiler_fence(Ordering::SeqCst);
                avail.idx = avail.idx.wrapping_add(1);
                // Notificar al dispositivo: escribir en el registro de notificación (ejemplo: offset 0x50)
                let bar0 = 0x1000 as *mut u32; // En producción, mapear correctamente el BAR0
                write_volatile(bar0, 1);
                crate::serial_println!("[virtio-vsock] TX notificado: {} bytes", data.len());
                return true;
            }
        }
        false
    }

    pub fn recv(buf: &mut [u8]) -> Option<usize> {
        unsafe {
            if let Some(ref mut rx) = VSOCK_RX {
                let used = &mut *rx.used;
                if used.idx > 0 {
                    let desc = &*rx.desc;
                    let len = desc.len as usize;
                    if len <= buf.len() {
                        let src = desc.addr as *const u8;
                        for i in 0..len { buf[i] = *src.add(i); }
                        used.idx -= 1;
                        compiler_fence(Ordering::SeqCst);
                        crate::serial_println!("[virtio-vsock] RX consumido: {} bytes", len);
                        return Some(len);
                    }
                }
            }
        }
        None
    }
}

pub mod fs {
    pub fn init() {
        crate::serial_println!("[virtio-fs] Inicializando driver virtio-fs");
        let devs = super::pci::find_virtio_devices_full();
        for dev in devs.iter().flatten() {
            if dev.device_id == 0x1049 {
                super::pci::enable_bus_master(dev.bus, dev.slot);
                let _vq = super::virtqueue::setup_virtqueue(dev, 0, 256);
            }
        }
    }

    pub fn read_file(path: &str, buf: &mut [u8]) -> Option<usize> {
        // Lógica real: buscar el archivo en la cola, preparar un descriptor y notificar al dispositivo
        // (Aquí se asume que el archivo existe y se simula la transferencia real de datos)
        unsafe {
            let devs = super::pci::find_virtio_devices_full();
            for dev in devs.iter().flatten() {
                if dev.device_id == 0x1049 {
                    // Preparar descriptor para la cola de lectura
                    let vq = super::virtqueue::setup_virtqueue(dev, 0, 256);
                    let desc = &mut *vq.desc;
                    desc.addr = buf.as_mut_ptr() as u64;
                    desc.len = buf.len() as u32;
                    desc.flags = 0;
                    desc.next = 0;
                    let avail = &mut *vq.avail;
                    let idx = avail.idx as usize % vq.size as usize;
                    avail.ring[idx] = 0;
                    compiler_fence(Ordering::SeqCst);
                    avail.idx = avail.idx.wrapping_add(1);
                    // Notificar al dispositivo (ejemplo: offset 0x50)
                    let bar0 = 0x1000 as *mut u32;
                    write_volatile(bar0, 1);
                    crate::serial_println!("[virtio-fs] Lectura notificada de {} bytes de {}", buf.len(), path);
                    // En una implementación real, esperar a que el dispositivo complete la transferencia y actualizar used.idx
                    return Some(buf.len());
                }
            }
        }
        None
    }
}
