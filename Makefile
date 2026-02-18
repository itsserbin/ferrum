# Ferrum Terminal - Build Commands
# Usage: make <target>

.PHONY: build build-gpu build-cpu release release-gpu release-cpu run run-gpu run-cpu test clean help

# Default: GPU build
build: build-gpu

# GPU renderer (default)
build-gpu:
	cargo build

# CPU-only renderer (no GPU dependencies)
build-cpu:
	cargo build --no-default-features

# Release builds
release: release-gpu

release-gpu:
	cargo build --release

release-cpu:
	cargo build --release --no-default-features

# Run
run: run-gpu

run-gpu:
	cargo run

run-cpu:
	cargo run --no-default-features

# Development
test:
	cargo test

check:
	cargo check

clippy:
	cargo clippy

fmt:
	cargo fmt

# Clean
clean:
	cargo clean

# Help
help:
	@echo "Ferrum Terminal - Build Targets:"
	@echo ""
	@echo "  Build:"
	@echo "    make build       - Debug build with GPU (default)"
	@echo "    make build-gpu   - Debug build with GPU renderer"
	@echo "    make build-cpu   - Debug build with CPU-only renderer"
	@echo ""
	@echo "  Release:"
	@echo "    make release     - Release build with GPU (default)"
	@echo "    make release-gpu - Release build with GPU renderer"
	@echo "    make release-cpu - Release build with CPU-only renderer"
	@echo ""
	@echo "  Run:"
	@echo "    make run         - Run with GPU (default)"
	@echo "    make run-gpu     - Run with GPU renderer"
	@echo "    make run-cpu     - Run with CPU-only renderer"
	@echo ""
	@echo "  Development:"
	@echo "    make test        - Run all tests"
	@echo "    make check       - Check compilation"
	@echo "    make clippy      - Run clippy lints"
	@echo "    make fmt         - Format code"
	@echo "    make clean       - Clean build artifacts"
