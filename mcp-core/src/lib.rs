#![no_std]

pub mod mcp_server {
    use core::sync::atomic::{AtomicBool, Ordering};
    use crate::ai_runtime;

    static READY: AtomicBool = AtomicBool::new(false);

    pub struct McpTool<'a> {
        pub name: &'a str,
        pub handler: fn(&[u8]) -> Option<Vec<u8>>,
    }

    static TOOLS: &[McpTool] = &[
        McpTool { name: "infer", handler: handle_infer },
        McpTool { name: "health", handler: handle_health },
        McpTool { name: "metadata", handler: handle_metadata },
        McpTool { name: "load_model", handler: handle_load_model },
        McpTool { name: "logs", handler: handle_logs },
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
        let ai_result = ai_runtime::infer(req.prompt);
        let resp = crate::ai_stub::InferResponse {
            text: ai_result,
            tokens: ai_result.split_whitespace().count() as u32,
            latency_ms: 1,
        };
        // Serializa manualmente a JSON mínimo (solo para el caso esperado)
        let mut buf = [0u8; 256];
        let n = crate::ai_stub::serialize_infer_response(&resp, &mut buf);
        Some(buf[..n].to_vec())
    }

    fn handle_health(_input: &[u8]) -> Option<Vec<u8>> {
        let status = if ai_runtime::MODEL.is_some() { "ok" } else { "not_loaded" };
        let details = if ai_runtime::MODEL.is_some() { "modelo cargado" } else { "sin modelo" };
        let resp = crate::ai_stub::HealthResponse { status, details };
        let mut buf = [0u8; 128];
        let n = crate::ai_stub::serialize_health_response(&resp, &mut buf);
        Some(buf[..n].to_vec())
    }

    fn handle_metadata(_input: &[u8]) -> Option<Vec<u8>> {
        let (model_name, quantization) = if let Some(model) = unsafe { ai_runtime::MODEL.as_ref() } {
            ("modelo-bin", "none")
        } else {
            ("not_loaded", "none")
        };
        let resp = crate::ai_stub::MetadataResponse {
            model_name,
            quantization,
            arch: "x86_64",
            features: &["SSE2"],
            build: "dev",
        };
        let mut buf = [0u8; 128];
        let n = crate::ai_stub::serialize_metadata_response(&resp, &mut buf);
        Some(buf[..n].to_vec())
    }

    fn handle_load_model(input: &[u8]) -> Option<Vec<u8>> {
        // Espera JSON: {"path": "ruta/modelo.bin"}
        let path = crate::ai_stub::parse_path_field(input)?;
        match ai_runtime::load_model(path) {
            Ok(()) => {
                let n = crate::ai_stub::serialize_status_ok(path, input);
                Some(input[..n].to_vec())
            },
            Err(e) => {
                let n = crate::ai_stub::serialize_status_error(e, input);
                Some(input[..n].to_vec())
            }
        }
    }

    fn handle_logs(_input: &[u8]) -> Option<Vec<u8>> {
        // Devuelve los últimos logs del ring buffer
        let mut buf = [0u8; 1024];
        let n = logging::log_read(&mut buf);
        Some(buf[..n].to_vec())
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
                if let Some((method, params)) = crate::ai_stub::parse_json_rpc(frame) {
                    if !["infer", "health", "metadata", "load_model"].contains(&method) {
                        crate::serial_println!("[mcp] Método desconocido: {}", method);
                        continue;
                    }
                    if let Some(resp) = crate::mcp_server::dispatch(method, params) {
                        let _ = crate::mcp_vsock_transport::write_frame(&resp);
                    }
                } else {
                    crate::serial_println!("[mcp] JSON-RPC inválido");
                }
            }
        }
    }
}

pub mod ai_stub {
    // miniserde eliminado, serialización/deserialización manual mínima
    #[derive(Debug)]
    pub struct InferRequest<'a> {
        pub prompt: &'a str,
        pub params: Option<InferParams>,
    }

    #[derive(Debug)]
    pub struct InferParams {
        pub max_tokens: Option<u32>,
        pub temperature: Option<f32>,
    }

    #[derive(Debug)]
    pub struct InferResponse<'a> {
        pub text: &'a str,
        pub tokens: u32,
        pub latency_ms: u32,
    }

    #[derive(Debug)]
    pub struct HealthResponse<'a> {
        pub status: &'a str,
        pub details: &'a str,
    }

    #[derive(Debug)]
    pub struct MetadataResponse<'a> {
        pub model_name: &'a str,
        pub quantization: &'a str,
        pub arch: &'a str,
        pub features: &'a [&'a str],
        pub build: &'a str,
    }

    // Funciones de serialización/deserialización mínima (solo para los campos usados)
    pub fn parse_infer_req(_json_bytes: &[u8]) -> Option<InferRequest> {
        // Implementar parser mínimo para {"prompt": "..."}
        None // TODO: implementar
    }

    pub fn serialize_infer_response(_resp: &InferResponse, _buf: &mut [u8]) -> usize {
        0 // TODO: implementar
    }

    pub fn serialize_health_response(_resp: &HealthResponse, _buf: &mut [u8]) -> usize {
        0 // TODO: implementar
    }

    pub fn serialize_metadata_response(_resp: &MetadataResponse, _buf: &mut [u8]) -> usize {
        0 // TODO: implementar
    }

    pub fn parse_path_field(_json_bytes: &[u8]) -> Option<&str> {
        None // TODO: implementar
    }

    pub fn serialize_status_ok(_path: &str, _buf: &[u8]) -> usize {
        0 // TODO: implementar
    }

    pub fn serialize_status_error(_err: &str, _buf: &[u8]) -> usize {
        0 // TODO: implementar
    }

    pub fn parse_json_rpc(_frame: &[u8]) -> Option<(&str, &[u8])> {
        None // TODO: implementar
    }

    pub fn infer(prompt: &str) -> &'static str {
        crate::serial_println!("[ai] Recibido prompt: {}", prompt);
        "[ai] Respuesta de ejemplo"
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
