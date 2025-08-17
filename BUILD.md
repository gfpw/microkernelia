# BUILD.md

## Requisitos previos

- Windows, Linux o WSL con Rust nightly (`rustup toolchain install nightly && rustup default nightly`)
- LLVM, Clang, LLD (instalables vía [LLVM releases](https://github.com/llvm/llvm-project/releases) o tu gestor de paquetes)
- QEMU y/o Firecracker
- Powershell o bash/zsh como shell predeterminada

## Primer build

1. Instala los targets:
   ```sh
   rustup target add x86_64-unknown-none
   ```
2. Construye todos los crates:
   ```sh
   cargo build --workspace --release
   ```
3. Usa el comando xtask (por ejemplo, para crear imagen):
   ```sh
   cargo run -p xtask -- image
   ```
4. O usa cargo-make para automatizar:
   ```sh
   cargo install cargo-make
   cargo make build-all
   ```

## Automatización y scripts

Se provee un `Makefile.toml` compatible con [cargo-make](https://sagiegurari.github.io/cargo-make/):

- `cargo make build-all` — Compila todo el workspace
- `cargo make image` — Genera imagen de microVM
- `cargo make qemu` — Lanza QEMU con el kernel
- `cargo make firecracker` — Lanza Firecracker con la imagen

## Notas
- El kernel y los crates principales usan `no_std`.
- El CLI y xtask pueden usar `std`.
- Los scripts de integración y pruebas se agregarán en fases posteriores.
