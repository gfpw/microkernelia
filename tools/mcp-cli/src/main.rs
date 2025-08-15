use std::io::{self, Write, Read};
use std::os::unix::net::UnixStream;

fn main() {
    let mut stream = UnixStream::connect("/tmp/vm.sock").expect("No se pudo conectar al vsock socket");
    let infer_req = r#"{\"method\":\"infer\",\"params\":{\"prompt\":\"Hola AI desde host\"}}"#;
    let mut frame = Vec::with_capacity(4 + infer_req.len());
    let len = (infer_req.len() as u32).to_be_bytes();
    frame.extend_from_slice(&len);
    frame.extend_from_slice(infer_req.as_bytes());
    stream.write_all(&frame).expect("Error enviando frame");
    let mut resp_len = [0u8; 4];
    stream.read_exact(&mut resp_len).expect("Error leyendo longitud de respuesta");
    let resp_len = u32::from_be_bytes(resp_len) as usize;
    let mut resp = vec![0u8; resp_len];
    stream.read_exact(&mut resp).expect("Error leyendo respuesta");
    println!("Respuesta infer: {}", String::from_utf8_lossy(&resp));
}
