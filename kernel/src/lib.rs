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

// Ejemplo de tarea
fn ejemplo_task() {
    serial_println!("[task] Ejecutando tarea de ejemplo");
    let respuesta = ai_runtime::infer_stub("Hola AI");
    serial_println!("[task] Respuesta AI: {}", respuesta);
}
