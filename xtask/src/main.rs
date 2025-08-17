use std::process::Command;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "qemu" {
        // Verificar que el kernel ELF existe
        let kernel_path = "target/kernel.elf";
        if !Path::new(kernel_path).exists() {
            eprintln!("ERROR: No se encontró el archivo {}. Ejecuta 'cargo make link-elf' primero.", kernel_path);
            std::process::exit(1);
        }
        // Ejecutar QEMU con el kernel ELF generado
        let status = Command::new("qemu-system-x86_64")
            .args([
                "-m", "512M",
                "-kernel", kernel_path,
                "-serial", "stdio",
                "-display", "none"
            ])
            .status()
            .expect("falló al lanzar QEMU (¿está instalado qemu-system-x86_64?)");
        if !status.success() {
            eprintln!("QEMU falló con código de salida {:?}", status.code());
            std::process::exit(1);
        }
        return;
    }
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", "x86_64-unknown-none"])
        .status()
        .expect("falló el build del kernel");
    if !status.success() {
        eprintln!("Build fallido");
        std::process::exit(1);
    }
    println!("Build kernel OK");
}
