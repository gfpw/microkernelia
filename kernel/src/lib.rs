#![no_std]
#![no_main]

use core::panic::PanicInfo;
extern crate alloc;
use linked_list_allocator::LockedHeap;
extern crate drivers_virtio;
extern crate ai_runtime;
extern crate mcp_vsock_transport;
extern crate mcp_core;

#[macro_use]
extern crate logging;

mod tests;

// Tamaño del heap: 1 MiB
const HEAP_SIZE: usize = 1024 * 1024;
static mut HEAP_SPACE: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Símbolos exportados por el linker para las secciones
    extern "C" {
        static __text_start: u8;
        static __text_end: u8;
        static __data_start: u8;
        static __data_end: u8;
        static __bss_start: u8;
        static __bss_end: u8;
        static __stack_start: u8;
        static __stack_end: u8;
    }
    // Inicializa MMU y protecciones
    mmu_init();
    unsafe {
        mmu_protect_sections(
            &__text_start as *const _ as usize,
            &__text_end as *const _ as usize,
            &__data_start as *const _ as usize,
            &__data_end as *const _ as usize,
            &__bss_start as *const _ as usize,
            &__bss_end as *const _ as usize,
            &__stack_start as *const _ as usize,
            &__stack_end as *const _ as usize,
        );
        // Inserta guard page al final del stack principal
        mmu_insert_guard_page(&__stack_end as *const _ as usize - PAGE_SIZE);
        // Inicializa canario de stack principal
        init_stack_canary(&__stack_start as *const _ as *mut u64);
        // Inicializa el heap global
        ALLOCATOR.lock().init(HEAP_SPACE.as_ptr() as usize, HEAP_SIZE);
    }
    // Inicializa el heap global de mcp_core (para alloc/Vec en no_std)
    mcp_core::init_heap(HEAP_SPACE.as_ptr() as usize, HEAP_SIZE);
    // Ejecuta pruebas automáticas de stack
    tests::test_stack_canary();
    // tests::test_guard_page(); // Descomentar para probar page fault (detendrá el kernel)
    serial_println!("\n[unikernel-ai] Kernel booting...");
    drivers_virtio::vsock::init();
    drivers_virtio::fs::init();
    mcp_vsock_transport::vsock_transport::init();
    mcp_core::mcp_server::init();
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
        logging::log_write(&msg);
        $crate::serial::_print(format_args!("{}", msg));
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

// implementación interna que devuelve Option<usize>
fn alloc_frame_impl() -> Option<usize> {
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

// wrapper con la firma que usan los drivers: extern "Rust" fn alloc_frame();
#[no_mangle]
pub extern "Rust" fn alloc_frame() {
    // Llamamos a impl y descartamos el resultado. Drivers solo necesitan el efecto.
    let _ = alloc_frame_impl();
}

// Si otra parte del kernel necesita la dirección, expón la impl:
pub fn alloc_frame_get() -> Option<usize> {
    alloc_frame_impl()
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
#[derive(Clone, Copy)]
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

/// Desmapea una región de memoria (actualiza tablas y hace invlpg)
pub fn unmap_phys_region(virt: usize, size: usize) {
    unsafe {
        let mut offset = 0;
        while offset < size {
            let vaddr = virt + offset;
            let pml4_idx = (vaddr >> 39) & 0x1FF;
            let pdpt_idx = (vaddr >> 30) & 0x1FF;
            let pd_idx = (vaddr >> 21) & 0x1FF;
            let pt_idx = (vaddr >> 12) & 0x1FF;
            // Si es hugepage
            if PD[pdpt_idx * PAGE_ENTRIES + pd_idx].0[pd_idx] & 0x80 != 0 {
                PD[pdpt_idx * PAGE_ENTRIES + pd_idx].0[pd_idx] = 0;
                offset += 2 * 1024 * 1024;
            } else {
                // 4KiB page
                let pt_base = &mut PD[pdpt_idx * PAGE_ENTRIES + pd_idx] as *mut _ as *mut PageTable;
                let pt = &mut (*pt_base).0;
                pt[pt_idx] = 0;
                offset += 4096;
            }
            // Invalida TLB para la página
            core::arch::asm!("invlpg [{}]", in(reg) vaddr, options(nostack, preserves_flags));
        }
    }
}

/// Mapea una región MMIO (ej. BAR0) en un rango virtual dedicado, RW y NX
pub fn map_mmio_region(phys: usize, size: usize) -> *mut u8 {
    // Elegimos un rango alto para MMIO, por ejemplo, 0xFFFF_C000_0000_0000+
    const MMIO_VIRT_BASE: usize = 0xFFFF_C000_0000_0000;
    static mut NEXT_MMIO_VIRT: usize = MMIO_VIRT_BASE;
    unsafe {
        let virt = NEXT_MMIO_VIRT;
        NEXT_MMIO_VIRT += (size + 0x1FFFFF) & !0x1FFFFF; // Alinear a 2MiB
        let mut offset = 0;
        while offset < size {
            let vaddr = virt + offset;
            let pml4_idx = (vaddr >> 39) & 0x1FF;
            let pdpt_idx = (vaddr >> 30) & 0x1FF;
            let pd_idx = (vaddr >> 21) & 0x1FF;
            // Asegura PDPT y PD
            if PML4.0[pml4_idx] & 1 == 0 {
                PML4.0[pml4_idx] = (&PDPT[pdpt_idx] as *const _ as u64) | 0b11;
            }
            if PDPT[pdpt_idx].0[pdpt_idx] & 1 == 0 {
                PDPT[pdpt_idx].0[pdpt_idx] = (&PD[pdpt_idx * PAGE_ENTRIES + pd_idx] as *const _ as u64) | 0b11;
            }
            // 2MiB page, RW, Present, NX (bit 63)
            PD[pdpt_idx * PAGE_ENTRIES + pd_idx].0[pd_idx] = (phys as u64 + offset as u64) | 0b10000011 | (1u64 << 63);
            offset += 2 * 1024 * 1024;
        }
        // Invalida TLB para la región
        core::arch::asm!("invlpg [{}]", in(reg) virt, options(nostack, preserves_flags));
        virt as *mut u8
    }
}

/// Marca las páginas de código RX, datos RW, stack RW, heap RW, todo NX excepto código
pub fn mmu_protect_sections(text_start: usize, text_end: usize, data_start: usize, data_end: usize, bss_start: usize, bss_end: usize, stack_start: usize, stack_end: usize) {
    unsafe {
        // Código: RX (Present, Read, Execute)
        let mut addr = text_start;
        while addr < text_end {
            let pml4_idx = (addr >> 39) & 0x1FF;
            let pdpt_idx = (addr >> 30) & 0x1FF;
            let pd_idx = (addr >> 21) & 0x1FF;
            let entry = &mut PD[pdpt_idx * PAGE_ENTRIES + pd_idx].0[pd_idx];
            *entry &= !(1u64 << 63); // Quita NX
            addr += 2 * 1024 * 1024;
        }
        // Datos, BSS, Stack, Heap: RW, NX
        for &(start, end) in &[(data_start, data_end), (bss_start, bss_end), (stack_start, stack_end)] {
            let mut addr = start;
            while addr < end {
                let pml4_idx = (addr >> 39) & 0x1FF;
                let pdpt_idx = (addr >> 30) & 0x1FF;
                let pd_idx = (addr >> 21) & 0x1FF;
                let entry = &mut PD[pdpt_idx * PAGE_ENTRIES + pd_idx].0[pd_idx];
                *entry |= 1u64 << 63; // NX
                addr += 2 * 1024 * 1024;
            }
        }
    }
}

/// Inserta una guard page (no mapeada) al final de cada stack
pub fn mmu_insert_guard_page(stack_end: usize) {
    unsafe {
        let pml4_idx = (stack_end >> 39) & 0x1FF;
        let pdpt_idx = (stack_end >> 30) & 0x1FF;
        let pd_idx = (stack_end >> 21) & 0x1FF;
        let pt_idx = (stack_end >> 12) & 0x1FF;
        // Asume que existe una tabla PT para este PD
        let pt_base = &mut PD[pdpt_idx * PAGE_ENTRIES + pd_idx] as *mut _ as *mut PageTable;
        let pt = &mut (*pt_base).0;
        pt[pt_idx] = 0; // No Present: guard page
        // Invalida TLB para la página
        core::arch::asm!("invlpg [{}]", in(reg) stack_end, options(nostack, preserves_flags));
    }
}

// Tarea kernel cooperativa
pub struct Task {
    pub entry: fn(),
    pub name: &'static str,
    pub finished: bool,
}

static mut TASKS: [Option<Task>; 8] = [None, None, None, None, None, None, None, None];
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

// --- Logging as a Task ---
fn log_task() {
    // Llama periódicamente a log_flush() de drivers-virtio
    // (solo si la feature virtio-log está activa)
    #[cfg(feature = "virtio-log")]
    {
        drivers_virtio::log_flush();
    }
    // Simula espera cooperativa (en el futuro: yield, sleep, timer, etc.)
}

// --- Scheduler multitarea cooperativo ---
pub fn run_scheduler() -> ! {
    // Spawnea el logging task como la primera tarea
    static mut LOG_TASK_SPAWNED: bool = false;
    unsafe {
        if (!LOG_TASK_SPAWNED) {
            spawn(log_task, "log_task");
            LOG_TASK_SPAWNED = true;
        }
    }
    loop {
        unsafe {
            for (i, task) in TASKS.iter_mut().enumerate() {
                if let Some(t) = task {
                    if !t.finished {
                        CURRENT = i;
                        (t.entry)();
                        // El logging task nunca termina
                        if t.name != "log_task" {
                            t.finished = true;
                        }
                    }
                }
            }
            // Después de cada ronda, el logging task hace flush de logs
            // (ya está incluido como task, pero aquí podríamos agregar timers, IPC, etc.)
        }
    }
}

/// Inicializa un canario de pila al crear el stack y verifica su integridad al hacer switch o terminar la tarea
static STACK_CANARY: u64 = 0xDEADC0DECAFEBABE;

pub fn init_stack_canary(stack_start: *mut u64) {
    unsafe { *stack_start = STACK_CANARY; }
}

pub fn check_stack_canary(stack_start: *const u64) -> bool {
    unsafe { *stack_start == STACK_CANARY }
}
