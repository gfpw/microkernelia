pub mod mcp_server {
    use core::sync::atomic::{AtomicBool, Ordering};
    static READY: AtomicBool = AtomicBool::new(false);
    pub fn init() {
        crate::serial_println!("[mcp] Servidor MCP inicializado (stub)");
        READY.store(true, Ordering::SeqCst);
    }
    pub fn is_ready() -> bool {
        READY.load(Ordering::SeqCst)
    }
}

pub mod ai_stub {
    use miniserde::{json, Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    pub struct InferRequest<'a> {
        pub prompt: &'a str,
        pub params: Option<InferParams>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct InferParams {
        pub max_tokens: Option<u32>,
        pub temperature: Option<f32>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct InferResponse<'a> {
        pub text: &'a str,
        pub tokens: u32,
        pub latency_ms: u32,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct HealthResponse<'a> {
        pub status: &'a str,
        pub details: &'a str,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct MetadataResponse<'a> {
        pub model_name: &'a str,
        pub quantization: &'a str,
        pub arch: &'a str,
        pub features: &'a [&'a str],
        pub build: &'a str,
    }

    pub fn parse_infer_req(json_bytes: &[u8]) -> Option<InferRequest> {
        json::from_slice(json_bytes).ok()
    }

    pub fn parse_health_req(json_bytes: &[u8]) -> bool {
        // health no tiene input, solo output
        json_bytes.is_empty() || json_bytes == b"{}"
    }

    pub fn parse_metadata_req(json_bytes: &[u8]) -> bool {
        // metadata no tiene input, solo output
        json_bytes.is_empty() || json_bytes == b"{}"
    }

    pub fn infer(prompt: &str) -> &'static str {
        crate::serial_println!("[ai] Recibido prompt: {}", prompt);
        "[ai] Respuesta de ejemplo"
    }
}
