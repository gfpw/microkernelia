# BUILD.md

## Requisitos previos

- Windows con Rust nightly (`rustup toolchain install nightly && rustup default nightly`)
- LLVM, Clang, LLD (instalables vía [LLVM releases](https://github.com/llvm/llvm-project/releases) o MSYS2)
- QEMU y/o Firecracker (descargar binarios para Windows)
- Powershell como shell predeterminada

## Primer build

1. Abre una terminal Powershell en la raíz del proyecto.
2. Instala los targets:
   ```powershell
   rustup target add x86_64-unknown-none
   ```
3. Construye todos los crates:
   ```powershell
   cargo build --workspace --release
   ```
4. Usa el comando xtask (por ejemplo, para crear imagen):
   ```powershell
   cargo run -p xtask -- image
   ```

## Notas
- El kernel y los crates principales usan `no_std`.
- El CLI y xtask pueden usar `std`.
- Los scripts de integración y pruebas se agregarán en fases posteriores.
