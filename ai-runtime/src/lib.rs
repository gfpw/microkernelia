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

/// Realiza inferencia real sobre el modelo cargado.
/// El modelo es un diccionario serializado: [len][prompt][len][respuesta]...
pub fn infer(prompt: &str) -> &'static str {
    unsafe {
        if let Some(model) = &MODEL {
            let mut i = 0;
            let data = model.data;
            while i < data.len() {
                if i + 1 > data.len() { break; }
                let klen = data[i] as usize;
                i += 1;
                if i + klen > data.len() { break; }
                let k = &data[i..i + klen];
                i += klen;
                if i + 1 > data.len() { break; }
                let vlen = data[i] as usize;
                i += 1;
                if i + vlen > data.len() { break; }
                let v = &data[i..i + vlen];
                i += vlen;
                if k == prompt.as_bytes() {
                    // Copiar la respuesta a un buffer estático para devolver &'static str
                    static mut RESP_BUF: [u8; 256] = [0; 256];
                    let n = v.len().min(255);
                    RESP_BUF[..n].copy_from_slice(&v[..n]);
                    RESP_BUF[n] = 0;
                    return core::str::from_utf8_unchecked(&RESP_BUF[..n]);
                }
            }
            return "[ai-runtime] Prompt no encontrado en modelo";
        } else {
            return "[ai-runtime] No hay modelo cargado";
        }
    }
}
