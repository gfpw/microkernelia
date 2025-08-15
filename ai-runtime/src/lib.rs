use drivers_virtio::fs;

pub struct Model {
    pub data: &'static [u8],
    pub size: usize,
}

static mut MODEL: Option<Model> = None;

/// Carga un modelo AI desde virtio-fs y lo mapea en memoria contigua.
pub fn load_model(path: &str) -> Result<(), &'static str> {
    // Asume que el buffer máximo de modelo es 8 MiB
    const MAX_MODEL_SIZE: usize = 8 * 1024 * 1024;
    // Reservar buffer estático (en producción, usar allocador real)
    static mut MODEL_BUF: [u8; MAX_MODEL_SIZE] = [0; MAX_MODEL_SIZE];
    let buf = unsafe { &mut MODEL_BUF };
    let read = fs::read_file(path, buf).ok_or("fs read error")?;
    unsafe {
        MODEL = Some(Model {
            data: &buf[..read],
            size: read,
        });
    }
    Ok(())
}

/// Realiza inferencia real sobre el modelo cargado (stub: solo muestra el tamaño del modelo)
pub fn infer(prompt: &str) -> &'static str {
    unsafe {
        if let Some(model) = &MODEL {
            // Aquí se integraría el FFI a ggml/llama.cpp o núcleo Rust real
            // Por ahora, solo muestra el tamaño del modelo cargado
            if model.size > 0 {
                return "[ai-runtime] Inferencia ejecutada (modelo cargado)";
            } else {
                return "[ai-runtime] Modelo vacío";
            }
        } else {
            return "[ai-runtime] No hay modelo cargado";
        }
    }
}

pub fn infer_stub(prompt: &str) -> &'static str {
    // Simula una inferencia AI mínima
    "[ai-runtime] Respuesta de ejemplo"
}
