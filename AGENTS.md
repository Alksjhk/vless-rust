# AGENTS.md

This file defines coding guidelines and development rules for AI agents working on this VLESS server project.

## Build/Test Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build (optimized)
make release             # Build + copy to root
make dev                 # Debug build + copy to root

# Test
cargo test                        # Run all tests
cargo test test_name              # Run specific test
cargo test --test test_file       # Run tests in specific file
cargo test -- --nocapture         # Show println! output

# Lint & Format
cargo fmt                          # Format code
cargo fmt --check                  # Check formatting (CI)
cargo clippy                       # Run linter
cargo clippy -- -D warnings        # Clippy with warnings as errors

# Check
cargo check                        # Fast compile check
cargo check --release              # Check release build

# Clean
cargo clean                        # Remove build artifacts
make clean                         # Clean + remove executables

# Run
cargo run                          # Run debug build
make run                           # Build and run release
```

## Code Style Guidelines

### Import Organization

Order imports in this sequence (separated by blank lines):

1. **Crate modules** (`use crate::`)
2. **External crates** (alphabetically: `anyhow`, `bytes`, `serde`, etc.)
3. **Standard library** (`use std::`)
4. **Tokio** (`use tokio::`)
5. **Tracing** (`use tracing::`)
6. **Other external** (uuid, chrono, etc.)

```rust
// Example from src/server.rs
use crate::api::{self, ApiConfig};
use crate::config::{PerformanceConfig, ProtocolType};
use crate::http::is_http_request;
use crate::tcp;
use crate::ws::{self, WsConnectionResult};

use anyhow::Result;
use bytes::Bytes;
use std::collections::{HashSet, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tracing::{info, error, debug};
use uuid::Uuid;
```

### Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Functions | `snake_case` | `handle_tcp_connection`, `parse_http_request` |
| Variables | `snake_case` | `client_addr`, `target_stream` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_LOG_ENTRIES`, `WEBSOCKET_GUID` |
| Types/Structs | `PascalCase` | `ServerConfig`, `VlessRequest` |
| Enums | `PascalCase` | `ProtocolType`, `Command` |
| Modules | `snake_case` | `tcp`, `ws`, `protocol` |
| Private fields | `snake_case` | `config`, `performance_config` |

### Error Handling

- Use `anyhow::Result<T>` for function return types
- Use `anyhow::anyhow!("message")` for error creation
- Use `?` operator for error propagation
- Include context in error messages

```rust
// Correct: Return anyhow::Result
pub async fn run(&self) -> Result<()> {
    let listener = TcpListener::bind(self.config.bind_addr).await?;
    // ...
}

// Correct: Contextual error messages
return Err(anyhow::anyhow!(
    "Connection closed by client (addr: {})", 
    client_addr
));

// Correct: Use map_err for custom error context
.map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?
```

### Type Definitions

```rust
// Public structs with Debug, Clone
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub protocol: ProtocolType,
    // ...
}

// Enums with derive macros
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtocolType {
    #[default]
    Tcp,
    #[serde(rename = "ws")]
    WebSocket,
}
```

### Async Functions

- Use `async fn` for async functions
- Use `tokio::spawn` for concurrent tasks
- Use `tokio::select!` for waiting on multiple futures

```rust
// Spawn concurrent task
tokio::spawn(async move {
    if let Err(e) = Self::handle_connection(...).await {
        error!("Error: {}", e);
    }
});

// Select pattern
tokio::select! {
    result = server.run() => { ... }
    _ = shutdown => { ... }
}
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_request_root() {
        let data = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let query = parse_http_request(data).unwrap();
        assert_eq!(query.path, "/");
    }

    #[tokio::test]
    async fn test_async_function() {
        // Async test with tokio runtime
    }
}
```

### Comments and Documentation

- Use `//!` for module-level documentation
- Use `///` for public item documentation
- Include examples in doc comments for public APIs

```rust
//! TCP Socket configuration module
//!
//! Provides TCP socket parameter configuration

/// Configure TCP socket options
///
/// # Arguments
/// * `stream` - TCP connection stream
/// * `recv_buf` - Receive buffer size (0 = system default)
pub fn configure_tcp_socket(...) -> Result<()> {
```

## Project Structure

```
src/
├── main.rs         # Entry point, TUI, signal handling
├── config.rs       # Configuration structures and parsing
├── server.rs       # Server core and connection dispatcher
├── protocol.rs     # VLESS protocol encoding/decoding
├── tcp.rs          # TCP protocol handler
├── ws.rs           # WebSocket protocol handler
├── http.rs         # HTTP request detection and response
├── api.rs          # HTTP API endpoints
├── wizard.rs       # Interactive configuration wizard
├── socket.rs       # TCP socket configuration
├── vless_link.rs   # VLESS link generation
├── public_ip.rs    # Public IP detection
├── service.rs      # Systemd/OpenRC service management
├── atomic_write.rs # Atomic file writing utilities
├── tui.rs          # Terminal UI components
├── version.rs      # Version display utilities
├── version_info.rs # Generated version constants (auto)
├── time.rs         # Time utilities (RFC3339)
└── build.rs        # Build script (version embedding)
```

## Critical Rules

1. **No hardcoding**: Never hardcode absolute paths, credentials, or magic numbers
2. **Error context**: Always provide context in error messages
3. **No `unwrap()` in production**: Use `expect()` with message or proper error handling
4. **No `as any` or `@ts-ignore`**: Maintain type safety
5. **No suppression of linter warnings without reason**
6. **Atomic operations**: Use atomic writes for configuration files
7. **Security**: Never log sensitive information (UUIDs in auth errors are acceptable for debugging)

## Technology Stack

- **Edition**: Rust 2021
- **Async Runtime**: Tokio (rt-multi-thread)
- **Error Handling**: anyhow
- **Logging**: tracing + tracing-subscriber
- **Serialization**: serde + serde_json
- **Memory Allocator**: mimalloc (not on musl targets)

## Commit Message Format

```
<type>: <short description>

<optional body>

<optional footer>
```

Types: `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `chore`

## Pre-commit Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy` reports no errors
- [ ] `cargo test` passes
- [ ] No `unwrap()` in non-test code without justification
- [ ] Documentation updated for public API changes