# Makefile for vless-rust
# Pure VLESS Server Build

.PHONY: all release dev clean build-release build-dev help

# Default target: build release
all: release

# Build release version (optimized) and copy to root
release:
	@echo "Building Release version..."
	cargo build --release
	@echo "Copying executable to project root..."
	cp target/release/vless.exe ./vless.exe 2>/dev/null || cp target/release/vless ./vless
	@echo "Done! Executable: ./vless"

# Build debug version and copy to root
dev:
	@echo "Building Debug version..."
	cargo build
	@echo "Copying executable to project root..."
	cp target/debug/vless.exe ./vless-debug.exe 2>/dev/null || cp target/debug/vless ./vless-debug
	@echo "Done! Executable: ./vless-debug"

# Build only (no copy)
build-release:
	cargo build --release

build-dev:
	cargo build

# Clean build artifacts
clean:
	cargo clean
	@echo "Cleaned target directory"
	@echo "Deleting executables from root..."
	rm -f ./vless ./vless.exe ./vless-debug ./vless-debug.exe
	@echo "All cleaned"

# Run server
run: release
	./vless.exe 2>/dev/null || ./vless

# Run debug version
run-dev: dev
	./vless-debug.exe 2>/dev/null || ./vless-debug

# Show help
help:
	@echo "vless-rust build commands:"
	@echo "Build:"
	@echo "  make all         - Build release version (default)"
	@echo "  make release     - Build release and copy to root"
	@echo "  make dev         - Build debug and copy to root"
	@echo "Run:"
	@echo "  make run         - Build and run release"
	@echo "  make run-dev     - Build and run debug"
	@echo "Clean:"
	@echo "  make clean       - Clean all build artifacts"
	@echo "Other:"
	@echo "  make help        - Show this help"
