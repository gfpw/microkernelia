pub mod mcp_server {
    use core::sync::atomic::{AtomicBool, Ordering};
    use miniserde::json;

    static READY: AtomicBool = AtomicBool::new(false);

    pub struct McpTool<'a> {
        pub name: &'a str,
        pub handler: fn(&[u8]) -> Option<Vec<u8>>,
    }

    static TOOLS: &[McpTool] = &[
        McpTool { name: "infer", handler: handle_infer },
        McpTool { name: "health", handler: handle_health },
        McpTool { name: "metadata", handler: handle_metadata },
    ];

    pub fn init() {
        crate::serial_println!("[mcp] Servidor MCP inicializado (stub)");
        READY.store(true, Ordering::SeqCst);
    }

    pub fn is_ready() -> bool {
        READY.load(Ordering::SeqCst)
    }

    fn handle_infer(input: &[u8]) -> Option<Vec<u8>> {
        let req = crate::ai_stub::parse_infer_req(input)?;
        let resp = crate::ai_stub::InferResponse {
            text: "[ai] Respuesta de ejemplo",
            tokens: 3,
            latency_ms: 1,
        };
        Some(json::to_vec(&resp))
    }

    fn handle_health(_input: &[u8]) -> Option<Vec<u8>> {
        let resp = crate::ai_stub::HealthResponse { status: "ok", details: "stub" };
        Some(json::to_vec(&resp))
    }

    fn handle_metadata(_input: &[u8]) -> Option<Vec<u8>> {
        let resp = crate::ai_stub::MetadataResponse {
            model_name: "stub-model",
            quantization: "none",
            arch: "x86_64",
            features: &["SSE2"],
            build: "dev",
        };
        Some(json::to_vec(&resp))
    }

    pub fn dispatch(tool: &str, input: &[u8]) -> Option<Vec<u8>> {
        for t in TOOLS {
            if t.name == tool {
                return (t.handler)(input);
            }
        }
        None
    }

    pub fn mcp_server_loop() {
        crate::serial_println!("[mcp] MCP server loop iniciado");
        let mut buf = [0u8; 4096];
        loop {
            if let Some(frame) = crate::mcp_vsock_transport::read_frame(&mut buf) {
                // Parsear JSON-RPC: {"method":..., "params":...}
                if let Ok(req) = miniserde::json::from_slice::<miniserde::json::Value>(frame) {
                    if let Some(method) = req.get("method").and_then(|m| m.as_str()) {
                        let params = req.get("params").and_then(|p| p.as_object()).map(|_| frame).unwrap_or(&[]);
                        if let Some(resp) = crate::mcp_server::dispatch(method, params) {
                            let _ = crate::mcp_vsock_transport::write_frame(&resp);
                        } else {
                            crate::serial_println!("[mcp] Método desconocido: {}", method);
                        }
                    }
                }
            }
        }
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
