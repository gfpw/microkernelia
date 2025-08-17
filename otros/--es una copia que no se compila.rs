#![no_std]

mod log_helpers;

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

#[repr(C)]
pub struct VirtqAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; 0], // tamaño dinámico
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VirtqUsedElem {
    pub id: u32,
    pub len: u32,
}

#[repr(C)]
pub struct VirtqUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtqUsedElem; 0], // dinámico
}

#[cfg(feature = "virtio-log")]
#[derive(Debug, Clone, Copy)]
pub enum LogEntry {
    FsReadNotified { requested: usize, path: LogPath },
    FsReadDone { done: usize, path: LogPath },
    VsockTx { len: usize },
    VsockRx { len: usize },
}

#[cfg(feature = "virtio-log")]
#[derive(Debug, Clone, Copy)]
pub struct LogPath {
    pub buf: [u8; 64],
    pub len: usize,
}

#[cfg(feature = "virtio-log")]
impl LogPath {
    pub fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let mut buf = [0u8; 64];
        let len = bytes.len().min(63);
        buf[..len].copy_from_slice(&bytes[..len]);
        Self { buf, len }
    }
}

#[cfg(feature = "virtio-log")]
impl core::fmt::Display for LogPath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = core::str::from_utf8(&self.buf[..self.len]).unwrap_or("");
        write!(f, "{}", s)
    }
}

#[cfg(feature = "virtio-log")]
const LOG_BUFFER_SIZE: usize = 256;
#[cfg(feature = "virtio-log")]
static mut LOG_BUFFER: [Option<LogEntry>; LOG_BUFFER_SIZE] = [None; LOG_BUFFER_SIZE];
#[cfg(feature = "virtio-log")]
static mut LOG_HEAD: usize = 0;
#[cfg(feature = "virtio-log")]
static mut LOG_TAIL: usize = 0;

#[cfg(feature = "virtio-log")]
pub fn log_enqueue(entry: LogEntry) {
    unsafe {
        LOG_BUFFER[LOG_HEAD] = Some(entry);
        LOG_HEAD = (LOG_HEAD + 1) % LOG_BUFFER_SIZE;
        if LOG_HEAD == LOG_TAIL {
            LOG_TAIL = (LOG_TAIL + 1) % LOG_BUFFER_SIZE;
        }
    }
}

#[cfg(not(feature = "virtio-log"))]
pub fn log_enqueue<T>(_entry: T) {}

#[cfg(feature = "virtio-log")]
pub fn log_flush() {
    unsafe {
        while LOG_TAIL != LOG_HEAD {
            if let Some(entry) = LOG_BUFFER[LOG_TAIL].take() {
                match entry {
                    LogEntry::FsReadNotified { requested, path } => {
                        serial_println!("[virtio-fs] Lectura notificada de {} bytes de {}", requested, path);
                    }
                    LogEntry::FsReadDone { done, path } => {
                        serial_println!("[virtio-fs] Lectura completada de {} bytes de {}", done, path);
                    }
                    LogEntry::VsockTx { len } => {
                        serial_println!("[virtio-vsock] TX notificado: {} bytes", len);
                    }
                    LogEntry::VsockRx { len } => {
                        serial_println!("[virtio-vsock] RX consumido: {} bytes", len);
                    }
                }
            }
            LOG_TAIL = (LOG_TAIL + 1) % LOG_BUFFER_SIZE;
        }
    }
}

#[cfg(not(feature = "virtio-log"))]
pub fn log_flush() {}

// --- PANIC HANDLER LOGIC ---
// Panic handler SOLO si somos crate raíz y target bare-metal (nunca si feature kernel)
#[cfg(all(
    not(feature = "kernel"),
    not(test),
    not(doctest),
    target_os = "none"
))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
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
        let mut logs: [Option<(u8, u8, u16)>; 32] = [None; 32];
        let mut idx = 0;
        for bus in 0..1 {
            for slot in 0..32 {
                let vendor = read_config(bus, slot, 0, 0) & 0xFFFF;
                if vendor == 0x1AF4 {
                    let device = ((read_config(bus, slot, 0, 2) >> 16) & 0xFFFF) as u16;
                    logs[idx] = Some((bus, slot, device));
                    idx += 1;
                }
            }
        }
    }

    pub fn find_virtio_devices_full() -> [Option<VirtioDevice>; 8] {
        let mut found: [Option<VirtioDevice>; 8] = [None; 8];
        let mut idx = 0;

        for bus in 0..1 {
            for slot in 0..32 {
                let vendor = read_config(bus, slot, 0, 0) & 0xFFFF;
                if vendor == 0x1AF4 {
                    let device = ((read_config(bus, slot, 0, 2) >> 16) & 0xFFFF) as u16;
                    let bar0 = read_config(bus, slot, 0, 0x10);
                    if idx < found.len() {
                        found[idx] = Some(VirtioDevice {
                            bus, slot, func: 0, device_id: device, bar0
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
    use super::{VirtqDesc, VirtqAvail, VirtqUsed, VirtqUsedElem};

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
        
        let desc_ptr = unsafe { 
            alloc_aligned(core::mem::size_of::<VirtqDesc>() * queue_size as usize, 16) 
        } as *mut VirtqDesc;
        
        let avail_size = core::mem::size_of::<VirtqAvail>() + (queue_size as usize) * core::mem::size_of::<u16>();
        let avail_ptr = unsafe { alloc_aligned(avail_size, 2) } as *mut VirtqAvail;
        
        let used_size = core::mem::size_of::<VirtqUsed>() + (queue_size as usize) * core::mem::size_of::<VirtqUsedElem>();
        let used_ptr = unsafe { alloc_aligned(used_size, 4) } as *mut VirtqUsed;
        
        VirtQueue {
            desc: desc_ptr,
            avail: avail_ptr,
            used: used_ptr,
            size: queue_size,
        }
    }
}

pub mod vsock {
    #[cfg(feature = "virtio-log")]
    use super::{LogEntry, log_enqueue};
    use super::virtqueue::VirtQueue;
    use core::ptr::write_volatile;
    use super::pci::map_bar0_phys_to_virt;

    static mut VSOCK_TX: Option<VirtQueue> = None;
    static mut VSOCK_RX: Option<VirtQueue> = None;
    static mut VSOCK_BAR0: Option<*mut u8> = None;

    pub fn init() {
        let mut _found_vsock = false;
        let mut _log_needed = false;
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
                _found_vsock = true;
                _log_needed = true;
            }
        }
    }

    pub fn send(data: &[u8]) -> bool {
        let len = data.len();
        let result = unsafe {
            // Acceso directo a static mut, documentado para Rust 2024
            if let (Some(ref mut tx), Some(bar0)) = (VSOCK_TX.as_mut(), VSOCK_BAR0.as_mut()) {
                {
                    let desc = &mut *tx.desc;
                    desc.addr = data.as_ptr() as u64;
                    desc.len = len as u32;
                    desc.flags = 0;
                    desc.next = 0;
                    let avail = &mut *tx.avail;
                    let idx = avail.idx as usize % tx.size as usize;
                    let ring = avail.ring.as_mut_ptr();
                    *ring.add(idx) = 0;
                    avail.idx = avail.idx.wrapping_add(1);
                    let notify_reg = bar0.add(0x50) as *mut u32;
                    write_volatile(notify_reg, 1);
                }
                true
            } else {
                false
            }
        };
        #[cfg(feature = "virtio-log")]
        if result {
            log_enqueue(LogEntry::VsockTx { len });
        }
        result
    }

    pub fn recv(buf: &mut [u8]) -> Option<usize> {
        let mut result = None;
        let mut _len_to_log = None;
        unsafe {
            if let Some(ref mut rx) = VSOCK_RX {
                let used = &mut *rx.used;
                if used.idx > 0 {
                    let desc = &*rx.desc;
                    let len = desc.len as usize;
                    if len <= buf.len() {
                        let src = desc.addr as *const u8;
                        for i in 0..len {
                            buf[i] = *src.add(i);
                        }
                        used.idx -= 1;
                        _len_to_log = Some(len);
                        result = Some(len);
                    }
                }
            }
        }
        #[cfg(feature = "virtio-log")]
        if let Some(len) = _len_to_log {
            log_enqueue(LogEntry::VsockRx { len });
        }
        result
    }
}

pub mod fs {
    #[cfg(feature = "virtio-log")]
    use super::{LogEntry, LogPath, log_enqueue};
    use core::ptr::write_volatile;
    use super::pci::map_bar0_phys_to_virt;
    use core::mem;
    use super::{VirtqAvail, VirtqDesc, VirtqUsed, VirtqUsedElem};

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
        
        let desc_ptr = unsafe {
            alloc_aligned(mem::size_of::<VirtqDesc>() * (queue_size as usize), 16)
        } as *mut VirtqDesc;
        
        let avail_size = mem::size_of::<VirtqAvail>() + (queue_size as usize) * mem::size_of::<u16>();
        let avail_ptr = unsafe { alloc_aligned(avail_size, 2) } as *mut VirtqAvail;
        
        let used_size = mem::size_of::<VirtqUsed>() + (queue_size as usize) * mem::size_of::<VirtqUsedElem>();
        let used_ptr = unsafe { alloc_aligned(used_size, 4) } as *mut VirtqUsed;
        
        let vq = VirtQueue {
            desc: desc_ptr,
            avail: avail_ptr,
            used: used_ptr,
            size: queue_size,
        };
        
        vq
    }

    pub fn init() {
        let mut _found_fs = false;
        let mut _log_needed = false;
        let devs = super::pci::find_virtio_devices_full();
        for dev in devs.iter().flatten() {
            if dev.device_id == 0x1049 {
                super::pci::enable_bus_master(dev.bus, dev.slot);
                let _vq = setup_virtqueue(dev, 0, 256);
                _found_fs = true;
                _log_needed = true;
            }
        }
    }

    #[cfg(feature = "virtio-log")]
    pub fn read_file(path: &str, buf: &mut [u8]) -> Option<usize> {
        let mut result = None;
        let mut log_data: Option<(usize, LogPath, usize)> = None;
        unsafe {
            let devs = super::pci::find_virtio_devices_full();
            for dev in devs.iter().flatten() {
                if dev.device_id == 0x1049 {
                    let vq = setup_virtqueue(dev, 0, 256);
                    let desc = &mut *vq.desc;
                    desc.addr = buf.as_mut_ptr() as u64;
                    desc.len = buf.len() as u32;
                    desc.flags = 0;
                    desc.next = 0;
                    let avail = &mut *vq.avail;
                    let idx = avail.idx as usize % vq.size as usize;
                    let ring = avail.ring.as_mut_ptr();
                    *ring.add(idx) = 0;
                    avail.idx = avail.idx.wrapping_add(1);
                    let bar0_virt = map_bar0_phys_to_virt(dev.bar0 & 0xFFFF_FFF0, 0x1000);
                    let notify_reg = bar0_virt.add(0x50) as *mut u32;
                    write_volatile(notify_reg, 1);
                    let used = &mut *vq.used;
                    let mut wait = 0;
                    while used.idx == 0 && wait < 1000000 {
                        wait += 1;
                    }
                    if used.idx > 0 {
                        let len = desc.len as usize;
                        log_data = Some((buf.len(), LogPath::from_str(path), len));
                        result = Some(len);
                    }
                }
            }
        }
        if let Some((log_len, log_path, log_done)) = log_data {
            log_enqueue(LogEntry::FsReadNotified { requested: log_len, path: log_path });
            log_enqueue(LogEntry::FsReadDone { done: log_done, path: log_path });
        }
        result
    }

    #[cfg(not(feature = "virtio-log"))]
    pub fn read_file(_path: &str, buf: &mut [u8]) -> Option<usize> {
        let mut result = None;
        unsafe {
            let devs = super::pci::find_virtio_devices_full();
            for dev in devs.iter().flatten() {
                if dev.device_id == 0x1049 {
                    let vq = setup_virtqueue(dev, 0, 256);
                    let desc = &mut *vq.desc;
                    desc.addr = buf.as_mut_ptr() as u64;
                    desc.len = buf.len() as u32;
                    desc.flags = 0;
                    desc.next = 0;
                    let avail = &mut *vq.avail;
                    let idx = avail.idx as usize % vq.size as usize;
                    let ring = avail.ring.as_mut_ptr();
                    *ring.add(idx) = 0;
                    avail.idx = avail.idx.wrapping_add(1);
                    let bar0_virt = map_bar0_phys_to_virt(dev.bar0 & 0xFFFF_FFF0, 0x1000);
                    let notify_reg = bar0_virt.add(0x50) as *mut u32;
                    write_volatile(notify_reg, 1);
                    let used = &mut *vq.used;
                    let mut wait = 0;
                    while used.idx == 0 && wait < 1000000 {
                        wait += 1;
                    }
                    if used.idx > 0 {
                        let len = desc.len as usize;
                        result = Some(len);
                    }
                }
            }
        }
        result
    }
}
