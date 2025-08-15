# microkernelia

[![Repo en GitHub](https://img.shields.io/badge/github-gfpw%2Fmicrokernelia-blue?logo=github)](https://github.com/gfpw/microkernelia)

Unikernel Rust no_std orientado a AI y servidores MCP sobre virtio-vsock.

---

**Autor:** Germán Ferreyra  
**Paradigma:** Vibe coding (desarrollo iterativo, colaborativo y experimental, priorizando el flow y la creatividad técnica)  
**Herramientas:** Rust nightly, QEMU, Firecracker, LLVM/Clang/LLD, Powershell, WSL, GitHub, GitHub Copilot (asistente de IA para refactorización, debugging y documentación)

---

## Estructura del proyecto

- [`kernel/`](./kernel): Núcleo Rust no_std, memoria, scheduler, logging serial
- [`drivers-virtio/`](./drivers-virtio): Drivers virtio (vsock, fs, block opcional)
- [`mcp-core/`](./mcp-core): Tipos y lógica MCP (JSON-RPC, framing, schemas)
- [`mcp-vsock-transport/`](./mcp-vsock-transport): Transporte MCP sobre virtio-vsock
- [`ai-runtime/`](./ai-runtime): Motor de inferencia AI (stub inicial, integración futura)
- [`logging/`](./logging): Buffer de logs y métricas
- [`xtask/`](./xtask): Herramientas de build, imagen y microVM
- [`tools/mcp-cli/`](./tools/mcp-cli): CLI host-side para pruebas MCP

## Documentación

- [Arquitectura y detalles técnicos](./ARCHITECTURE.md)
- [Guía de build y requisitos](./BUILD.md)

## Build rápido

```powershell
rustup target add x86_64-unknown-none
cargo build --workspace --release
```

Para más detalles, ver [BUILD.md](./BUILD.md).

## Estado y roadmap

- [x] Scaffold inicial y crates
- [x] Kernel mínimo y logging serial
- [x] Drivers virtio básicos
- [ ] Integración AI runtime
- [ ] Pruebas de integración y documentación avanzada

---

Repositorio oficial: https://github.com/gfpw/microkernelia

> Consulta [ARCHITECTURE.md](./ARCHITECTURE.md) y [BUILD.md](./BUILD.md) para detalles técnicos y de compilación.
