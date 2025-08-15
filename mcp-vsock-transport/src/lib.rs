pub mod vsock_transport {
    pub fn init() {
        crate::serial_println!("[mcp-vsock] Transporte MCP/vsock inicializado (stub)");
        // Aquí se implementará el framing y la gestión de conexiones vsock
    }

    /// Framing MCP: lectura y escritura de mensajes length-prefixed (u32 big-endian)
    pub fn read_frame(buf: &mut [u8]) -> Option<&[u8]> {
        // Simulación: en una implementación real, leería de la cola vsock RX
        // Aquí solo retorna None (stub)
        None
    }

    pub fn write_frame(json: &[u8]) -> bool {
        if json.len() > 1024 * 1024 { return false; }
        // Simulación: en una implementación real, escribiría en la cola vsock TX
        crate::serial_println!("[mcp-vsock] Enviando frame de {} bytes", json.len());
        true
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
