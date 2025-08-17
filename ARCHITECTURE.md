# Unikernel-AI: Arquitectura y Estructura

## Estructura del workspace

- `kernel/`: Núcleo Rust no_std, entrypoint, memoria, scheduler, logging serial.
- `drivers_virtio/`: Drivers virtio (vsock, fs, block opcional).
- `mcp_core/`: Tipos y lógica MCP (JSON-RPC, framing, schemas).
- `mcp_vsock_transport/`: Transporte MCP sobre virtio-vsock.
- `ai_runtime/`: Motor de inferencia AI (stub inicial, integración futura con ggml/candle).
- `logging/`: Buffer de logs y métricas.
- `xtask/`: Herramientas de build, imagen y ejecución de microVM.
- `tools/mcp-cli/`: CLI host-side para pruebas MCP.

## Flujo de desarrollo

1. Scaffold inicial (completado).
2. Implementar kernel mínimo (arranque, panic handler, logging serial).
3. Integrar drivers virtio y transporte MCP.
4. Añadir stub de AI runtime y exponer herramientas MCP.
5. Scripts de build y pruebas de integración.

## Build y dependencias

- El kernel debe compilarse con la feature `global-allocator`:
  ```sh
  cargo build -p kernel --target x86_64-unknown-none --release --features global-allocator
  ```
- `drivers_virtio` solo define panic handler si se compila como crate raíz y no como dependencia del kernel.
- Para compilar binarios de usuario:
  ```sh
  cargo build -p mcp-cli
  cargo build -p xtask
  ```

## Protección de memoria y robustez de stack

El kernel implementa una MMU x86_64 de producción con:
- Tablas PML4, PDPT, PD y soporte para mapeo de 4KiB y 2MiB.
- Mapeo alto para el kernel (por encima de 0xFFFF_8000_0000_0000).
- No se mapea la página cero para atrapar accesos nulos.
- Mapeo dedicado para regiones MMIO (por ejemplo, BAR0 de dispositivos virtio).
- Protección de páginas: código RX, datos/stack/heap RW y NX, guard pages en stacks.
- Desmapeo seguro de memoria liberada.

### Guard pages y canarios de stack
- Cada stack de tarea (y el stack principal) termina en una guard page (página no mapeada) para detectar desbordamientos.
- Se inicializa un canario de pila (valor fijo) al inicio de cada stack. El kernel verifica su integridad al hacer context switch o finalizar la tarea.
- Si el canario es sobrescrito, se reporta corrupción de stack.

### Pruebas automáticas
- Al arrancar, el kernel ejecuta pruebas automáticas:
    - Verifica el canario de stack y simula su corrupción para asegurar la detección.
    - (Opcional) Intenta escribir en la guard page para provocar un page fault y validar la protección.
- Los resultados de las pruebas se reportan por log serial y buffer de logs MCP.

### Ejemplo de log de arranque
```
[test] Verificando canario de stack...
[test] Canario OK antes de corrupción
[test] Canario detectó corrupción correctamente
[unikernel-ai] Kernel booting...
```

## Referencias
- [Toro Kernel](https://github.com/torokernel/torokernel)
- [Model Context Protocol](https://modelcontextprotocol.io/specification/2025-03-26)
- [rust-vmm/vm-virtio](https://github.com/rust-vmm/vm-virtio)

---

Este documento se irá ampliando en cada fase.
