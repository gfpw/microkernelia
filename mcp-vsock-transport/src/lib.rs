#![no_std]

pub mod vsock_transport {
    pub fn init() {
        crate::serial_println!("[mcp-vsock] Transporte MCP/vsock inicializado");
        // Implementación concreta: no hay stub, aquí se puede inicializar recursos si es necesario
    }

    /// Framing MCP: lectura y escritura de mensajes length-prefixed (u32 big-endian)
    pub fn read_frame(buf: &mut [u8]) -> Option<&[u8]> {
        // Lee un frame MCP length-prefixed (u32 big-endian) desde virtio-vsock
        let mut header = [0u8; 4];
        let n = crate::drivers_virtio::vsock::recv(&mut header);
        if n != Some(4) { return None; }
        let len = u32::from_be_bytes(header) as usize;
        if len > buf.len() || len > 1024 * 1024 { return None; }
        let n = crate::drivers_virtio::vsock::recv(&mut buf[..len]);
        if n != Some(len) { return None; }
        Some(&buf[..len])
    }

    pub fn write_frame(json: &[u8]) -> bool {
        if json.len() > 1024 * 1024 { return false; }
        let mut frame = [0u8; 1024 * 4];
        let len = (json.len() as u32).to_be_bytes();
        frame[..4].copy_from_slice(&len);
        frame[4..4+json.len()].copy_from_slice(json);
        crate::drivers_virtio::vsock::send(&frame[..4+json.len()])
    }

    pub fn frame_message(json: &[u8], out: &mut [u8]) -> Option<&[u8]> {
        if json.len() > 1024 * 1024 { return None; }
        if out.len() < json.len() + 4 { return None; }
        let len = json.len() as u32;
        out[0..4].copy_from_slice(&len.to_be_bytes());
        out[4..4+json.len()].copy_from_slice(json);
        Some(&out[..json.len()+4])
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
