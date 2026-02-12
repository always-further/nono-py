.PHONY: build build-release dev install test lint fmt fmt-check clean help

# Default target
help:
	@echo "nono-py - Python bindings for nono sandboxing library"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Build targets:"
	@echo "  build        Build the package in debug mode"
	@echo "  build-release Build the package in release mode"
	@echo "  dev          Build and install in development mode"
	@echo "  install      Install the package"
	@echo ""
	@echo "Test targets:"
	@echo "  test         Run all tests"
	@echo "  test-quick   Run tests without rebuilding"
	@echo ""
	@echo "Quality targets:"
	@echo "  lint         Run linters (clippy + mypy)"
	@echo "  fmt          Format code (rustfmt + ruff)"
	@echo "  fmt-check    Check formatting"
	@echo ""
	@echo "Other targets:"
	@echo "  clean        Remove build artifacts"

# Build in debug mode
build:
	maturin build

# Build in release mode
build-release:
	maturin build --release

# Development install (editable)
dev:
	maturin develop

# Install the package
install:
	maturin develop --release

# Run tests (rebuilds first)
test: dev
	pytest tests/ -v

# Run tests without rebuilding
test-quick:
	pytest tests/ -v

# Run Rust linter
lint-rust:
	cargo clippy -- -D warnings

# Run Python type checker
lint-python:
	mypy python/nono_py

# Run all linters
lint: lint-rust lint-python

# Format Rust code
fmt-rust:
	cargo fmt

# Format Python code (requires ruff)
fmt-python:
	-ruff format python/ tests/
	-ruff check --fix python/ tests/

# Format all code
fmt: fmt-rust fmt-python

# Check Rust formatting
fmt-check-rust:
	cargo fmt --check

# Check Python formatting (requires ruff)
fmt-check-python:
	-ruff format --check python/ tests/
	-ruff check python/ tests/

# Check all formatting
fmt-check: fmt-check-rust fmt-check-python

# Remove build artifacts
clean:
	cargo clean
	rm -rf target/
	rm -rf dist/
	rm -rf *.egg-info/
	rm -rf .pytest_cache/
	rm -rf .mypy_cache/
	rm -rf .ruff_cache/
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true
	find . -type f -name "*.pyo" -delete 2>/dev/null || true
	find . -type f -name "*.so" -delete 2>/dev/null || true

# CI target - run all checks
ci: fmt-check lint test
