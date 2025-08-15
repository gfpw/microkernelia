#![no_std]

#[macro_use]
extern crate logging;

use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{compiler_fence, Ordering};
use crate::pci::map_bar0_phys_to_virt;

// FFI boundary for kernel memory allocation
extern "Rust" {
    fn alloc_frame();
}

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
    use crate::alloc_frame;
    use core::ptr::{read_volatile, write_volatile};

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
                    let device = device as u16;
                    let log_bus = bus;
                    let log_slot = slot;
                    let log_device = device;
                    serial_println!(
                        "[virtio-pci] Encontrado dispositivo virtio: bus {} slot {} device {:04x}",
                        log_bus, log_slot, log_device
                    );
                }
            }
        }
    }

    pub fn find_virtio_devices_full() -> [Option<VirtioDevice>; 8] {
        let mut found: [Option<VirtioDevice>; 8] = [None, None, None, None, None, None, None, None];
        let mut idx = 0;
        let mut logs = [None; 8];
        for bus in 0..1 {
            for slot in 0..32 {
                let vendor = read_config(bus, slot, 0, 0) & 0xFFFF;
                if vendor == 0x1AF4 {
                    let device = (read_config(bus, slot, 0, 2) >> 16) & 0xFFFF;
                    let device = device as u16;
                    let bar0 = read_config(bus, slot, 0, 0x10);
                    if idx < found.len() {
                        found[idx] = Some(VirtioDevice {
                            bus, slot, func: 0, device_id: device, bar0
                        });
                        logs[idx] = Some((bus, slot, device, bar0));
                        idx += 1;
                    }
                }
            }
        }
        for log in logs.iter().flatten() {
            let (log_bus, log_slot, log_device, log_bar0) = *log;
            serial_println!(
                "[virtio-pci] Dispositivo virtio: bus {} slot {} dev {:04x} bar0 {:08x}",
                log_bus, log_slot, log_device, log_bar0
            );
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

    pub fn map_bar0_phys_to_virt(bar0_phys: u32, size: usize) -> *mut u8 {
        let start = bar0_phys as usize & !(2 * 1024 * 1024 - 1);
        let end = (bar0_phys as usize + size + 2 * 1024 * 1024 - 1) & !(2 * 1024 * 1024 - 1);
        let mut addr = start;
        while addr < end {
            unsafe { alloc_frame(); }
            addr += 2 * 1024 * 1024;
        }
        extern "Rust" { fn map_phys_to_virt(phys: usize, size: usize) -> *mut u8; }
        unsafe { map_phys_to_virt(bar0_phys as usize, size) }
    }
}

pub mod virtqueue {
    use super::{VirtqDesc, VirtqAvail, VirtqUsed};

    pub struct VirtQueue {
        pub desc: *mut VirtqDesc,
        pub avail: *mut VirtqAvail,
        pub used: *mut VirtqUsed,
        pub size: u16,
    }

    pub fn setup_virtqueue(_dev: &super::pci::VirtioDevice, _queue_idx: u16, queue_size: u16) -> VirtQueue {
        extern "Rust" {
            fn alloc_aligned(size: usize, align: usize) -> *mut u8;
        }
        let desc_ptr = unsafe { alloc_aligned(core::mem::size_of::<VirtqDesc>() * queue_size as usize, 16) } as *mut VirtqDesc;
        let avail_ptr = unsafe { alloc_aligned(core::mem::size_of::<super::VirtqAvail>(), 2) } as *mut super::VirtqAvail;
        let used_ptr = unsafe { alloc_aligned(core::mem::size_of::<super::VirtqUsed>(), 4) } as *mut super::VirtqUsed;
        let log_desc = desc_ptr;
        let log_avail = avail_ptr;
        let log_used = used_ptr;
        let vq = VirtQueue {
            desc: desc_ptr,
            avail: avail_ptr,
            used: used_ptr,
            size: queue_size,
        };
        serial_println!("[virtqueue] setup_virtqueue: desc={:p} avail={:p} used={:p}", log_desc, log_avail, log_used);
        vq
    }
}

pub mod vsock {
    use super::virtqueue::VirtQueue;
    use core::ptr::write_volatile;
    use core::sync::atomic::{compiler_fence, Ordering};
    use crate::pci::map_bar0_phys_to_virt;

    static mut VSOCK_TX: Option<VirtQueue> = None;
    static mut VSOCK_RX: Option<VirtQueue> = None;
    static mut VSOCK_BAR0: Option<*mut u8> = None;

    pub fn init() {
        let mut found_vsock = false;
        let devs = super::pci::find_virtio_devices_full();
        for dev in devs.iter().flatten() {
            let is_vsock = dev.device_id == 0x1040 || dev.device_id == 0x105A;
            if is_vsock {
                super::pci::enable_bus_master(dev.bus, dev.slot);
                let bar0_virt = map_bar0_phys_to_virt(dev.bar0 & 0xFFFF_FFF0, 0x1000);
                let tx = super::virtqueue::setup_virtqueue(dev, 0, 256);
                let rx = super::virtqueue::setup_virtqueue(dev, 1, 256);
                unsafe {
                    VSOCK_TX = Some(tx);
                    VSOCK_RX = Some(rx);
                    VSOCK_BAR0 = Some(bar0_virt);
                }
                found_vsock = true;
            }
        }
        if found_vsock {
            serial_println!("[virtio-vsock] Inicializando driver vsock");
        }
    }

    pub fn send(data: &[u8]) -> bool {
        let len = data.len();
        let result = unsafe {
            if let (Some(ref mut tx), Some(bar0)) = (VSOCK_TX.as_mut(), VSOCK_BAR0) {
                let desc = &mut *tx.desc;
                desc.addr = data.as_ptr() as u64;
                desc.len = len as u32;
                desc.flags = 0;
                desc.next = 0;
                let avail = &mut *tx.avail;
                let idx = avail.idx as usize % tx.size as usize;
                avail.ring[idx] = 0;
                compiler_fence(Ordering::SeqCst);
                avail.idx = avail.idx.wrapping_add(1);
                let notify_reg = bar0.add(0x50) as *mut u32;
                write_volatile(notify_reg, 1);
                true
            } else {
                false
            }
        };
        if result {
            serial_println!("[virtio-vsock] TX notificado: {} bytes", len);
        }
        result
    }

    pub fn recv(buf: &mut [u8]) -> Option<usize> {
        let mut result = None;
        let mut log_len = None;
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
                        log_len = Some(len);
                        result = Some(len);
                    }
                }
            }
        }
        if let Some(len) = log_len {
            serial_println!("[virtio-vsock] RX consumido: {} bytes", len);
        }
        result
    }
}

pub mod fs {
    use core::ptr::write_volatile;
    use core::sync::atomic::{compiler_fence, Ordering};
    use crate::pci::map_bar0_phys_to_virt;

    pub fn init() {
        let mut found_fs = false;
        let devs = super::pci::find_virtio_devices_full();
        for dev in devs.iter().flatten() {
            if dev.device_id == 0x1049 {
                super::pci::enable_bus_master(dev.bus, dev.slot);
                let _vq = super::virtqueue::setup_virtqueue(dev, 0, 256);
                found_fs = true;
            }
        }
        if found_fs {
            serial_println!("[virtio-fs] Inicializando driver virtio-fs");
        }
    }

    pub fn read_file(path: &str, buf: &mut [u8]) -> Option<usize> {
        let mut result = None;
        let mut log: Option<(usize, &str, usize)> = None;
        unsafe {
            let devs = super::pci::find_virtio_devices_full();
            for dev in devs.iter().flatten() {
                if dev.device_id == 0x1049 {
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
                    let bar0_virt = map_bar0_phys_to_virt(dev.bar0 & 0xFFFF_FFF0, 0x1000);
                    let notify_reg = bar0_virt.add(0x50) as *mut u32;
                    write_volatile(notify_reg, 1);
                    let used = &mut *vq.used;
                    let mut wait = 0;
                    while used.idx == 0 && wait < 1000000 {
                        compiler_fence(Ordering::SeqCst);
                        wait += 1;
                    }
                    if used.idx > 0 {
                        let len = desc.len as usize;
                        let log_len = buf.len();
                        let log_path = path;
                        let log_done = len;
                        log = Some((log_len, log_path, log_done));
                        result = Some(len);
                    }
                }
            }
        }
        if let Some((log_len, log_path, log_done)) = log {
            serial_println!("[virtio-fs] Lectura notificada de {} bytes de {}", log_len, log_path);
            serial_println!("[virtio-fs] Lectura completada de {} bytes de {}", log_done, log_path);
        }
        result
    }
}
