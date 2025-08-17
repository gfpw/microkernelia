// Servidor condicional Rust: acepta Unix socket en Unix y TCP en Windows
// Protocolo: frame de 4 bytes big-endian (longitud) seguido de JSON

#[cfg(unix)]
use std::os::unix::net::UnixListener;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;

fn handle_client<T: Read + Write>(mut stream: T) {
    // Lee longitud (4 bytes)
    let mut len_buf = [0u8; 4];
    if let Err(e) = stream.read_exact(&mut len_buf) {
        eprintln!("Error leyendo longitud: {e}");
        return;
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    if let Err(e) = stream.read_exact(&mut buf) {
        eprintln!("Error leyendo frame: {e}");
        return;
    }
    let req = String::from_utf8_lossy(&buf);
    println!("Recibido: {req}");
    // Responde con un JSON fijo (puedes adaptar la lógica)
    let resp = r#"{\"result\":\"¡Hola desde el servidor!\"}"#;
    let resp_len = (resp.len() as u32).to_be_bytes();
    if let Err(e) = stream.write_all(&resp_len) {
        eprintln!("Error enviando longitud de respuesta: {e}");
        return;
    }
    if let Err(e) = stream.write_all(resp.as_bytes()) {
        eprintln!("Error enviando respuesta: {e}");
    }
}

fn main() {
    #[cfg(unix)]
    {
        let path = "/tmp/vm.sock";
        let _ = std::fs::remove_file(path); // Borra si ya existe
        let listener = UnixListener::bind(path).expect("No se pudo crear el socket Unix");
        println!("Servidor Unix escuchando en {path}");
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(|| handle_client(stream));
                }
                Err(e) => eprintln!("Error de conexión: {e}"),
            }
        }
    }
    #[cfg(windows)]
    {
        let listener = TcpListener::bind("127.0.0.1:5000").expect("No se pudo crear el socket TCP");
        println!("Servidor TCP escuchando en 127.0.0.1:5000");
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(|| handle_client(stream));
                }
                Err(e) => eprintln!("Error de conexión: {e}"),
            }
        }
    }
}
