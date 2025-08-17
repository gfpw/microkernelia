//! Pruebas automáticas de robustez de stack y protección de memoria para el kernel
//
// Estas pruebas requieren acceso a símbolos del linker y funciones internas del kernel.
// Se ejecutan desde el arranque llamando a test_stack_canary() y test_guard_page().

use super::*;
use crate::serial_println;

/// Prueba de detección de corrupción de canario de stack
pub fn test_stack_canary() {
    extern "C" { static __stack_start: u8; }
    let stack_start = unsafe { &__stack_start as *const _ as *mut u64 };
    serial_println!("[test] Verificando canario de stack...");
    if check_stack_canary(stack_start) {
        serial_println!("[test] Canario OK antes de corrupción");
    } else {
        serial_println!("[test] FALLO: Canario corrupto antes de tiempo");
    }
    // Corrompe el canario
    unsafe { *stack_start = 0xBAD0BAD0BAD0BAD0; }
    if !check_stack_canary(stack_start) {
        serial_println!("[test] Canario detectó corrupción correctamente");
    } else {
        serial_println!("[test] FALLO: Canario NO detectó corrupción");
    }
}

/// Prueba de acceso a guard page (debe causar page fault)
pub fn test_guard_page() {
    extern "C" { static __stack_end: u8; }
    let guard_addr = unsafe { (&__stack_end as *const _ as usize) - PAGE_SIZE };
    serial_println!("[test] Accediendo guard page (debe causar page fault)...");
    unsafe {
        let p = guard_addr as *mut u8;
        // Voluntariamente accede la guard page
        core::ptr::write_volatile(p, 0xAA);
    }
    serial_println!("[test] FALLO: Acceso a guard page NO causó page fault");
}
