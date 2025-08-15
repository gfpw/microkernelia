pub mod vsock_transport {
    pub fn init() {
        crate::serial_println!("[mcp-vsock] Transporte MCP/vsock inicializado (stub)");
        // Aquí se implementará el framing y la gestión de conexiones vsock
    }

    pub fn read_frame(buf: &mut [u8]) -> Option<usize> {
        // Simula la lectura de un frame MCP (stub)
        None
    }

    pub fn write_frame(data: &[u8]) -> bool {
        // Simula el envío de un frame MCP (stub)
        true
    }

    pub fn frame_message(json: &[u8], out: &mut [u8]) -> Option<usize> {
        if json.len() > 1024 * 1024 { return None; }
        if out.len() < json.len() + 4 { return None; }
        let len = json.len() as u32;
        out[0..4].copy_from_slice(&len.to_be_bytes());
        out[4..4+json.len()].copy_from_slice(json);
        Some(json.len() + 4)
    }
}
