#![no_std]
extern crate alloc;

#[cfg(all(feature = "global-allocator", not(test)))]
use linked_list_allocator::LockedHeap;

pub mod mcp_server {
    use core::sync::atomic::{AtomicBool, Ordering};
    use alloc::vec::Vec;

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
        READY.store(true, Ordering::SeqCst);
        logging::log_write("[mcp] Servidor MCP inicializado (stub)");
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
        let mut buf = [0u8; 256];
        let n = crate::ai_stub::serialize_infer_response(&resp, &mut buf);
        Some(buf[..n].to_vec())
    }

    fn handle_health(_input: &[u8]) -> Option<Vec<u8>> {
        let status = if unsafe { core::ptr::addr_of!(ai_runtime::MODEL).as_ref().is_some() } { "ok" } else { "not_loaded" };
        let details = if unsafe { core::ptr::addr_of!(ai_runtime::MODEL).as_ref().is_some() } { "modelo cargado" } else { "sin modelo" };
        let resp = crate::ai_stub::HealthResponse { status, details };
        let mut buf = [0u8; 128];
        let n = crate::ai_stub::serialize_health_response(&resp, &mut buf);
        Some(buf[..n].to_vec())
    }

    fn handle_metadata(_input: &[u8]) -> Option<Vec<u8>> {
        let (model_name, quantization) = if let Some(_model) = unsafe { core::ptr::addr_of!(ai_runtime::MODEL).as_ref() } {
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
        use mcp_vsock_transport::vsock_transport::{read_frame, write_frame};
        let mut buf = [0u8; 4096];
        logging::log_write("[mcp] MCP server loop iniciado");
        loop {
            let mut log_json_invalid = false;
            if let Some(frame) = read_frame(&mut buf) {
                if let Some((method, params)) = crate::ai_stub::parse_json_rpc(frame) {
                    if !["infer", "health", "metadata", "load_model"].contains(&method) {
                        logging::log_write("[mcp] Método desconocido");
                        continue;
                    }
                    if let Some(resp) = crate::mcp_server::dispatch(method, params) {
                        let _ = write_frame(&resp);
                    }
                } else {
                    log_json_invalid = true;
                }
            }
            if log_json_invalid {
                logging::log_write("[mcp] JSON-RPC inválido");
            }
        }
    }
}

pub mod ai_stub {
    use logging::log_write;

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

    pub fn parse_infer_req(_json_bytes: &[u8]) -> Option<InferRequest<'_>> {
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
        log_write("[ai] Recibido prompt: ");
        log_write(prompt);
        "[ai] Respuesta de ejemplo"
    }
}
