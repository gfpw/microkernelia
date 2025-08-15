// Ring buffer de logs para observabilidad MCP
use core::sync::atomic::{AtomicUsize, Ordering};

const LOG_BUF_SIZE: usize = 4096;
static mut LOG_BUF: [u8; LOG_BUF_SIZE] = [0; LOG_BUF_SIZE];
static LOG_HEAD: AtomicUsize = AtomicUsize::new(0);
static LOG_TAIL: AtomicUsize = AtomicUsize::new(0);

/// Agrega un mensaje al ring buffer de logs (llamado desde serial_println!)
pub fn log_write(msg: &str) {
    let bytes = msg.as_bytes();
    unsafe {
        for &b in bytes {
            let head = LOG_HEAD.load(Ordering::Relaxed);
            LOG_BUF[head % LOG_BUF_SIZE] = b;
            LOG_HEAD.store((head + 1) % LOG_BUF_SIZE, Ordering::Relaxed);
        }
    }
}

/// Lee los logs acumulados en el ring buffer (para exponer por MCP)
pub fn log_read(out: &mut [u8]) -> usize {
    let mut n = 0;
    unsafe {
        let mut tail = LOG_TAIL.load(Ordering::Relaxed);
        let head = LOG_HEAD.load(Ordering::Acquire);
        while tail != head && n < out.len() {
            out[n] = LOG_BUF[tail % LOG_BUF_SIZE];
            tail = (tail + 1) % LOG_BUF_SIZE;
            n += 1;
        }
        LOG_TAIL.store(tail, Ordering::Release);
    }
    n
}
