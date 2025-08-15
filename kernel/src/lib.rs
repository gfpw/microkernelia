#![no_std]
#![no_main]

use core::panic::PanicInfo;
extern crate drivers_virtio;
extern crate mcp_core;
extern crate mcp_vsock_transport;
extern crate ai_runtime;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("\n[unikernel-ai] Kernel booting...");
    drivers_virtio::vsock::init();
    drivers_virtio::fs::init();
    mcp_vsock_transport::vsock_transport::init();
    mcp_core::mcp_server::init();
    spawn(ejemplo_task, "ejemplo");
    run_scheduler();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[PANIC] Kernel panic!");
    loop {}
}

// Serial logging (mínimo, sin dependencias externas)
#[macro_export]
macro_rules! serial_println {
    ($($arg:tt)*) => {{
        let msg = alloc::format!(concat!($($arg)*, "\n"));
        $crate::logging::log_write(&msg);
        $crate::serial::_print(format_args_nl!($($arg)*));
    }};
}

pub mod serial {
    use core::fmt::{self, Write};
    const PORT: u16 = 0x3F8;
    pub fn _print(args: fmt::Arguments) {
        let _ = SerialPort.write_fmt(args);
    }
    struct SerialPort;
    impl Write for SerialPort {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for byte in s.bytes() {
                unsafe { core::arch::asm!("out dx, al", in("dx") PORT, in("al") byte); }
            }
            Ok(())
        }
    }
}

// Frame allocator simple (bitmap, 2MiB-aligned)
const FRAME_SIZE: usize = 2 * 1024 * 1024;
const MAX_FRAMES: usize = 128;
static mut FRAME_BITMAP: [u8; MAX_FRAMES] = [0; MAX_FRAMES];

pub fn alloc_frame() -> Option<usize> {
    unsafe {
        for (i, used) in FRAME_BITMAP.iter_mut().enumerate() {
            if *used == 0 {
                *used = 1;
                return Some(i * FRAME_SIZE);
            }
        }
        None
    }
}

pub fn free_frame(addr: usize) {
    unsafe {
        let i = addr / FRAME_SIZE;
        if i < MAX_FRAMES {
            FRAME_BITMAP[i] = 0;
        }
    }
}

/// Asigna memoria alineada de tamaño `size` y alineación `align` usando frames del kernel.
#[no_mangle]
pub extern "Rust" fn alloc_aligned(size: usize, align: usize) -> *mut u8 {
    // Solo soporta alineaciones potencias de 2 y múltiplos de FRAME_SIZE.
    // Busca suficientes frames contiguos para satisfacer la petición.
    let frames_needed = (size + FRAME_SIZE - 1) / FRAME_SIZE;
    let mut start = None;
    unsafe {
        let mut count = 0;
        for i in 0..MAX_FRAMES {
            if FRAME_BITMAP[i] == 0 {
                if start.is_none() { start = Some(i); }
                count += 1;
                if count == frames_needed {
                    let base = start.unwrap();
                    for j in base..(base + frames_needed) { FRAME_BITMAP[j] = 1; }
                    let addr = base * FRAME_SIZE;
                    // Ajustar alineación si es necesario
                    let aligned_addr = (addr + align - 1) & !(align - 1);
                    return aligned_addr as *mut u8;
                }
            } else {
                start = None;
                count = 0;
            }
        }
    }
    core::ptr::null_mut()
}

// MMU x86_64: soporte de producción para mapeo físico→virtual (4KiB y 2MiB)
const PAGE_SIZE: usize = 4096;
const PAGE_ENTRIES: usize = 512;

#[repr(align(4096))]
pub struct PageTable([u64; PAGE_ENTRIES]);

static mut PML4: PageTable = PageTable([0; PAGE_ENTRIES]);
static mut PDPT: [PageTable; PAGE_ENTRIES] = [PageTable([0; PAGE_ENTRIES]); PAGE_ENTRIES];
static mut PD: [PageTable; PAGE_ENTRIES * PAGE_ENTRIES] = [PageTable([0; PAGE_ENTRIES]); PAGE_ENTRIES * PAGE_ENTRIES];

// Dirección base alta para el kernel (por ejemplo, 0xFFFF_8000_0000_0000)
const KERNEL_VIRT_BASE: usize = 0xFFFF_8000_0000_0000;

/// Inicializa la MMU con mapeo identidad temporal y mapeo alto para el kernel
pub fn mmu_init() {
    unsafe {
        // No mapear la página cero (0x0) para atrapar accesos nulos
        PML4.0[0] = 0;
        // Mapeo identidad (temporal, solo para arranque, excepto página cero)
        PDPT[0].0[0] = (&PD[0] as *const _ as u64) | 0b11;
        for i in 1..PAGE_ENTRIES {
            PD[0].0[i] = ((i as u64) << 21) | 0b10000011;
        }
        // Mapeo alto para el kernel
        let pml4_idx = (KERNEL_VIRT_BASE >> 39) & 0x1FF;
        PML4.0[pml4_idx] = (&PDPT[1] as *const _ as u64) | 0b11;
        PDPT[1].0[0] = (&PD[1] as *const _ as u64) | 0b11;
        for i in 0..PAGE_ENTRIES {
            PD[1].0[i] = ((i as u64) << 21) | 0b10000011;
        }
        // Cargar CR3
        let pml4_phys = &PML4 as *const _ as u64;
        core::arch::asm!("mov cr3, {}", in(reg) pml4_phys, options(nostack, preserves_flags));
    }
}

/// Mapea una región física a virtual (soporta 4KiB y 2MiB, RW, Present)
pub fn map_phys_to_virt(phys: usize, size: usize) -> *mut u8 {
    unsafe {
        let mut offset = 0;
        while offset < size {
            let virt = phys + offset;
            let pml4_idx = (virt >> 39) & 0x1FF;
            let pdpt_idx = (virt >> 30) & 0x1FF;
            let pd_idx = (virt >> 21) & 0x1FF;
            let pt_idx = (virt >> 12) & 0x1FF;
            // Asegura PDPT y PD
            if PML4.0[pml4_idx] & 1 == 0 {
                PML4.0[pml4_idx] = (&PDPT[pdpt_idx] as *const _ as u64) | 0b11;
            }
            if PDPT[pdpt_idx].0[pdpt_idx] & 1 == 0 {
                PDPT[pdpt_idx].0[pdpt_idx] = (&PD[pdpt_idx * PAGE_ENTRIES + pd_idx] as *const _ as u64) | 0b11;
            }
            // Si está alineado a 2MiB y size >= 2MiB, usa hugepage
            if (virt & (0x1FFFFF)) == 0 && (size - offset) >= 2 * 1024 * 1024 {
                PD[pdpt_idx * PAGE_ENTRIES + pd_idx].0[pd_idx] = (phys as u64 + offset as u64) | 0b10000011;
                offset += 2 * 1024 * 1024;
            } else {
                // 4KiB page
                // Asume que existe una tabla PT para este PD
                let pt_base = &mut PD[pdpt_idx * PAGE_ENTRIES + pd_idx] as *mut _ as *mut PageTable;
                let pt = &mut (*pt_base).0;
                pt[pt_idx] = (phys as u64 + offset as u64) | 0b11;
                offset += 4096;
            }
        }
        // Invalida TLB para la región
        core::arch::asm!("invlpg [{}]", in(reg) phys, options(nostack, preserves_flags));
        phys as *mut u8
    }
}

// Tarea kernel cooperativa
pub struct Task {
    pub entry: fn(),
    pub name: &'static str,
    pub finished: bool,
}

static mut TASKS: [Option<Task>; 4] = [None, None, None, None];
static mut CURRENT: usize = 0;

pub fn spawn(entry: fn(), name: &'static str) {
    unsafe {
        for slot in TASKS.iter_mut() {
            if slot.is_none() {
                *slot = Some(Task { entry, name, finished: false });
                break;
            }
        }
    }
}

pub fn run_scheduler() -> ! {
    loop {
        unsafe {
            for (i, task) in TASKS.iter_mut().enumerate() {
                if let Some(t) = task {
                    if !t.finished {
                        CURRENT = i;
                        (t.entry)();
                        t.finished = true;
                    }
                }
            }
        }
    }
}
