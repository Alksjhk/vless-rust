# AGENTS.md

Coding guidelines for AI agents working on this VLESS server project.

## Build/Test Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build (optimized)
make release                   # Build + copy to root
make dev                       # Debug build + copy to root

# Test
cargo test                     # Run all tests
cargo test test_name           # Run specific test by name
cargo test --lib               # Unit tests only (src/)
cargo test --test protocol_test # Specific integration test file
cargo test -- --nocapture      # Show println! output

# Lint & Format
cargo fmt                      # Format code
cargo fmt --check              # Check formatting (CI)
cargo clippy                   # Run linter
cargo clippy -- -D warnings    # Warnings as errors

# Check & Clean
cargo check                    # Fast compile check
cargo clean                    # Remove build artifacts
make clean                     # Clean + remove executables

# Run
cargo run                      # Run debug build
make run                       # Build and run release
```

## Code Style

### Import Order

1. Crate modules (`use crate::`)
2. External crates (alphabetically)
3. Standard library (`use std::`)
4. Tokio (`use tokio::`)
5. Tracing (`use tracing::`)
6. Other external (uuid, chrono, etc.)

```rust
use crate::config::ServerConfig;
use crate::protocol::VlessRequest;

use anyhow::Result;
use bytes::Bytes;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tracing::{debug, error, info};
use uuid::Uuid;
```

### Naming

| Type | Convention | Example |
|------|------------|---------|
| Functions/Variables | `snake_case` | `handle_connection`, `client_addr` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_BUFFER_SIZE` |
| Types/Structs/Enums | `PascalCase` | `ServerConfig`, `ProtocolType` |
| Modules | `snake_case` | `tcp`, `ws`, `protocol` |

### Error Handling

- Use `anyhow::Result<T>` for returns
- Use `anyhow::anyhow!("message")` for errors
- Use `?` operator for propagation
- Include context in messages

```rust
pub async fn run(&self) -> Result<()> {
    let listener = TcpListener::bind(self.config.bind_addr).await?;
    // ...
}

return Err(anyhow!("Connection closed by {}", addr));
```

### Types

```rust
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtocolType {
    #[default]
    Tcp,
    #[serde(rename = "ws")]
    WebSocket,
}
```

### Async Patterns

```rust
// Spawn task
tokio::spawn(async move {
    if let Err(e) = handle_client(stream, addr).await {
        error!("Client error: {}", e);
    }
});

// Select with shutdown
tokio::select! {
    result = server.run() => { ... }
    _ = shutdown_signal() => { info!("Shutting down"); }
}
```

### Testing

- Unit tests: Inline `#[cfg(test)]` modules in source files
- Integration tests: `tests/` directory (protocol_test.rs, tcp_test.rs, etc.)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let data = b"GET / HTTP/1.1\r\n";
        assert!(parse_request(data).is_ok());
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = async_fn().await;
        assert!(result.is_ok());
    }
}
```

## Project Structure

```
src/
├── main.rs           # Entry point, TUI, signals
├── lib.rs            # Library exports
├── server.rs         # Connection dispatcher
├── protocol.rs       # VLESS protocol codec
├── tcp.rs            # TCP handler
├── ws.rs             # WebSocket handler
├── http.rs           # HTTP detection
├── config.rs         # Configuration parsing
├── api.rs            # HTTP API endpoints
├── address.rs        # DNS resolution
├── socket.rs         # TCP socket config
├── wizard.rs         # Interactive config
├── vless_link.rs     # VLESS link generation
├── public_ip.rs      # IP detection
├── service.rs        # Linux service mgmt
├── atomic_write.rs   # Atomic file writes
├── tui.rs            # Terminal UI
├── version.rs        # Version display
└── version_info.rs   # Generated constants

tests/                # Integration tests
├── protocol_test.rs
├── tcp_test.rs
├── ws_test.rs
├── server_test.rs
├── vless_link_test.rs
├── public_ip_test.rs
└── atomic_write_test.rs

build.rs              # Build script (version, Windows resources)
Cargo.toml            # Package manifest
Makefile              # Build targets
```

## Critical Rules

1. **No hardcoding** - Never hardcode paths, credentials, or magic numbers
2. **Error context** - Always provide context in error messages
3. **No `unwrap()` in production** - Use `expect()` with message or proper handling
4. **Type safety** - No `as any` or type suppression
5. **No lint suppression** without documented reason
6. **Atomic writes** - Use atomic writes for config files
7. **Security** - Never log sensitive info (UUIDs in auth errors OK for debug)

## Tech Stack

- **Edition**: Rust 2021
- **Async**: Tokio (rt-multi-thread)
- **Errors**: anyhow
- **Logging**: tracing + tracing-subscriber
- **Serialization**: serde + serde_json
- **Allocator**: mimalloc (not on musl)

## Platform-Specific Code

```rust
// Memory allocator (musl incompatible)
#[cfg(not(target_env = "musl"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// Platform-specific imports
#[cfg(windows)]
use windows::Win32::{...};

#[cfg(unix)]
use libc::{...};

// Signal handling
#[cfg(unix)] {
    use tokio::signal::unix::{signal, SignalKind};
    // Handle SIGINT, SIGTERM
}
#[cfg(not(unix))] {
    // Windows: Ctrl+C only
    let _ = signal::ctrl_c().await;
}
```

## Release Profile

```toml
[profile.release]
lto = "thin"           # Link-time optimization
codegen-units = 1      # Maximum optimization
opt-level = 3          # Speed priority
panic = "abort"        # Smaller binary
strip = true           # Remove symbols
```

## Pre-commit Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy` reports no errors
- [ ] `cargo test` passes
- [ ] No `unwrap()` in non-test code without justification
- [ ] Documentation updated for public API changes
