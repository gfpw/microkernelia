# Unikernel-AI: Arquitectura y Estructura

## Estructura del workspace

- `kernel/`: Núcleo Rust no_std, entrypoint, memoria, scheduler, logging serial.
- `drivers-virtio/`: Drivers virtio (vsock, fs, block opcional).
- `mcp-core/`: Tipos y lógica MCP (JSON-RPC, framing, schemas).
- `mcp-vsock-transport/`: Transporte MCP sobre virtio-vsock.
- `ai-runtime/`: Motor de inferencia AI (stub inicial, integración futura con ggml/candle).
- `logging/`: Buffer de logs y métricas.
- `xtask/`: Herramientas de build, imagen y ejecución de microVM.
- `tools/mcp-cli/`: CLI host-side para pruebas MCP.

## Flujo de desarrollo

1. Scaffold inicial (completado).
2. Implementar kernel mínimo (arranque, panic handler, logging serial).
3. Integrar drivers virtio y transporte MCP.
4. Añadir stub de AI runtime y exponer herramientas MCP.
5. Scripts de build y pruebas de integración.

## Referencias
- [Toro Kernel](https://github.com/torokernel/torokernel)
- [Model Context Protocol](https://modelcontextprotocol.io/specification/2025-03-26)
- [rust-vmm/vm-virtio](https://github.com/rust-vmm/vm-virtio)

---

Este documento se irá ampliando en cada fase.
