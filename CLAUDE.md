# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A high-performance VLESS protocol server implemented in Rust with Tokio. Follows xray-core VLESS protocol specification (versions 0 and 1).

**Key Features:**
- TCP and WebSocket transport support
- UDP over TCP (UoT) proxy
- Multi-user UUID authentication
- TUI terminal interface with log scrolling
- HTTP API for VLESS link generation
- Linux service management (systemd/OpenRC)

## Build Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build (optimized with lto=thin, codegen-units=1)

# Run
cargo run                # Run with config.json (auto-starts wizard if missing)
cargo run -- /path/to/config.json    # Specify config file
cargo run -- --no-tui    # Disable TUI interface
DISABLE_TUI=1 cargo run  # Alternative way to disable TUI

# Test
cargo test                              # Run all tests
cargo test test_name                    # Run specific test
cargo test -- --nocapture              # Show println! output
cargo test --test integration_test      # Run specific test file

# Lint & Format
cargo fmt                # Format code
cargo fmt --check        # Check formatting (CI)
cargo clippy             # Run linter
cargo clippy -- -D warnings    # Fail on warnings

# Check
cargo check              # Fast syntax/type check
cargo clean              # Remove build artifacts
```

## Architecture

### Connection Lifecycle

```
main.rs → TcpListener
    ↓
server.rs:handle_connection() → Detect protocol type
    ↓
    ├─ HTTP request → api.rs (info page / VLESS link generation)
    ├─ WebSocket → ws.rs → tcp.rs (proxy logic)
    └─ VLESS TCP → tcp.rs (proxy logic)
```

### Module Responsibilities

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point, graceful shutdown signal handling, TUI setup |
| `server.rs` | Connection dispatcher, protocol detection (HTTP vs VLESS vs WebSocket) |
| `tcp.rs` | VLESS protocol handshake, TCP/UDP proxy forwarding |
| `ws.rs` | WebSocket upgrade handling, bridges to TCP proxy logic |
| `protocol.rs` | VLESS protocol encoding/decoding, `VlessRequest`/`VlessResponse` structs |
| `http.rs` | HTTP request detection, path/query extraction |
| `api.rs` | HTTP handlers for info page and VLESS link generation |
| `config.rs` | Configuration parsing with serde, defaults validation |
| `address.rs` | DNS resolution, target address parsing |
| `socket.rs` | TCP socket option configuration (nodelay, buffer sizes) |
| `service.rs` | Linux service install/uninstall (systemd/OpenRC auto-detect) |

### Key Design Patterns

**Protocol Detection:**
- `http::is_http_request()` inspects first bytes for HTTP method signatures
- WebSocket detected via `Upgrade: websocket` header after HTTP parsing
- VLESS protocol determined by UUID-based header validation

**Proxy Data Flow:**
- Bidirectional copy via `tokio::spawn` for each direction
- Configurable buffer size (default 128KB from `performance.buffer_size`)
- Connection terminates when either direction closes

**Configuration:**
- JSON config with `server`, `users`, `performance` sections
- Auto-generates via interactive wizard if `config.json` missing
- Performance tuning: TCP nodelay, buffer sizes, UDP timeout

## Testing

**Test Locations:**
- Unit tests: Inline `#[cfg(test)]` modules in source files
- Integration tests: `tests/` directory (`protocol_test.rs`, `tcp_test.rs`, etc.)

**Test Patterns:**
```bash
# Async tests use #[tokio::test]
# Return anyhow::Result for ? operator support
# tempfile crate for file-based tests
```

## Platform-Specific Notes

**Memory Allocator:**
- Uses `mimalloc` on non-musl targets only
- Musl targets use default allocator

**Signal Handling:**
- Unix: SIGINT, SIGTERM with graceful shutdown
- Windows: Ctrl+C only

**Service Management:**
- Systemd: User service (no root needed), logs via journalctl
- OpenRC: System service (root needed), logs to `/var/log/vless-rust-serve.log`

## Development Guidelines

**Import Order:** Crate modules → External crates → std → tokio → tracing → Other

**Error Handling:**
- Return `anyhow::Result<T>` from functions
- Use `?` operator for propagation
- Include context: `anyhow::anyhow!("Failed to X: {}", e)`

**Security:**
- Never log UUIDs in success paths (acceptable in auth errors for debugging)
- Use `atomic_write.rs` for config file modifications
- Validate UUID format before protocol processing

## Release Profile

See `Cargo.toml` `[profile.release]`:
- `lto = "thin"`, `codegen-units = 1`, `opt-level = 3`
- `panic = "abort"`, `strip = true`

## References

- [VLESS Protocol Spec](https://xtls.github.io/en/development/protocols/vless.html)
- [xray-core](https://github.com/XTLS/Xray-core)
