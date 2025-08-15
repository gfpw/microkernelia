# Unikernel-AI: Workspace

Este workspace contiene la estructura base para el desarrollo de un unikernel Rust (no_std) con motor de inferencia AI y servidor MCP sobre virtio-vsock.

## Estructura de crates

- kernel/
- drivers-virtio/
- mcp-core/
- mcp-vsock-transport/
- ai-runtime/
- logging/
- xtask/
- tools/mcp-cli/

## Próximos pasos

- [x] Inicializar cada crate con su respectivo Cargo.toml y archivo fuente principal.
- [x] Configurar el workspace en Cargo.toml raíz.
- [x] Añadir scripts de build y documentación.
- [x] Implementar kernel mínimo: entrypoint, panic handler, logging serial, linker script.
- [x] Comando de build reproducible vía xtask.
- [ ] Inicializar memoria y scheduler cooperativo.
- [ ] Integrar drivers virtio (vsock, fs).
- [ ] Implementar stub MCP y AI runtime.
- [ ] Pruebas de integración y documentación avanzada.
