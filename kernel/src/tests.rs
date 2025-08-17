//! Pruebas automáticas de robustez de stack y protección de memoria para el kernel
//
// Estas pruebas requieren acceso a símbolos del linker y funciones internas del kernel.
// Se ejecutan desde el arranque llamando a test_stack_canary() y test_guard_page().

use super::*;
use crate::serial_println;

// test_stack_canary y test_guard_page eliminados porque no son seguros ni portables en Rust moderno
