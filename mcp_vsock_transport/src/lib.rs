#![no_std]

pub mod vsock_transport {
    pub fn init() {
        // Si se requiere log, usar el sistema de logging global o dejar vacío
        // log::info!("[mcp-vsock] Transporte MCP/vsock inicializado");
    }

    use drivers_virtio::vsock;

    /// Framing MCP: lectura y escritura de mensajes length-prefixed (u32 big-endian)
    pub fn read_frame(buf: &mut [u8]) -> Option<&[u8]> {
        let mut header = [0u8; 4];
        let n = vsock::recv(&mut header);
        if n != Some(4) { return None; }
        let len = u32::from_be_bytes(header) as usize;
        if len > buf.len() || len > 1024 * 1024 { return None; }
        let n = vsock::recv(&mut buf[..len]);
        if n != Some(len) { return None; }
        Some(&buf[..len])
    }

    pub fn write_frame(json: &[u8]) -> bool {
        if json.len() > 1024 * 1024 { return false; }
        let mut frame = [0u8; 4096]; // 4 KiB, suficiente para la mayoría de mensajes
        let len = (json.len() as u32).to_be_bytes();
        frame[..4].copy_from_slice(&len);
        frame[4..4+json.len()].copy_from_slice(json);
        vsock::send(&frame[..4+json.len()])
    }

    pub fn frame_message<'a>(json: &'a [u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
        if json.len() > 1024 * 1024 { return None; }
        if out.len() < json.len() + 4 { return None; }
        let len = json.len() as u32;
        out[0..4].copy_from_slice(&len.to_be_bytes());
        out[4..4+json.len()].copy_from_slice(json);
        Some(&out[..json.len()+4])
    }

    // pub use read_frame;
    // pub use write_frame;
}
