# Development Guide

This guide explains how to set up and develop nono-py, including working with a local copy of the nono Rust library.

## Prerequisites

- **Rust toolchain**: Install via [rustup](https://rustup.rs/)
- **Python 3.9+**
- **uv**: Install via [docs.astral.sh/uv](https://docs.astral.sh/uv/getting-started/installation/)

## Project Structure

```
nono-py/
├── Cargo.toml              # Rust crate configuration
├── pyproject.toml          # Python package configuration
├── Makefile                # Build commands
├── src/
│   └── lib.rs              # PyO3 bindings (Rust)
├── python/
│   └── nono_py/
│       ├── __init__.py     # Python package entry point
│       ├── _nono_py.pyi    # Type stubs
│       └── py.typed        # PEP 561 marker
└── tests/
    └── test_*.py           # Python tests
```

## Working with Local nono Crate

The nono-py bindings depend on the `nono` Rust library. By default, `Cargo.toml` references the local nono workspace:

```toml
[dependencies]
nono = { path = "../nono/crates/nono" }
```

### Directory Layout

For local development, arrange your directories like this:

```
~/dev/
├── nono/                   # Main nono repository
│   ├── crates/
│   │   ├── nono/           # Core library
│   │   └── nono-cli/       # CLI binary
│   └── ...
└── nono-py/                # This repository
    └── ...
```

### Adjusting the Path

If your nono repository is in a different location, update `Cargo.toml`:

```toml
# Relative path (recommended for local dev)
nono = { path = "../nono/crates/nono" }

# Or absolute path
nono = { path = "/home/user/projects/nono/crates/nono" }

# Or git reference (for CI/release)
nono = { git = "https://github.com/always-further/nono", branch = "main" }

# Or crates.io (when published)
nono = "0.1"
```

## Setup

### 1. Clone Both Repositories

```bash
cd ~/dev
git clone https://github.com/always-further/nono.git
git clone https://github.com/always-further/nono-py.git
```

### 2. Install Dependencies

```bash
cd nono-py
uv sync
```

This creates a virtual environment and installs all dev dependencies (maturin, pytest, mypy, ruff).

### 3. Build and Install

```bash
# Development build (debug mode, faster compilation)
uv run maturin develop

# Or use make
make dev
```

## Development Workflow

### Making Changes to nono-py

1. Edit Rust code in `src/lib.rs`
2. Edit Python code in `python/nono_py/`
3. Rebuild: `uv run maturin develop`
4. Run tests: `uv run pytest tests/ -v`

### Making Changes to nono Library

When you need to modify the underlying nono library:

1. Make changes in `../nono/crates/nono/`
2. Rebuild nono-py: `uv run maturin develop`
   - maturin automatically picks up changes in the dependency
3. Test your changes

### Iterating on Both

For rapid iteration when changing both repositories:

```bash
# Terminal 1: Watch nono library
cd ~/dev/nono
cargo watch -x check

# Terminal 2: Rebuild and test nono-py
cd ~/dev/nono-py
uv run maturin develop && uv run pytest tests/ -v
```

## Build Commands

```bash
# Development build (debug, fast)
make dev

# Release build (optimized)
make install

# Run tests
make test

# Run linters
make lint

# Format code
make fmt

# Clean build artifacts
make clean
```

## Testing

### Run All Tests

```bash
uv run pytest tests/ -v
```

### Run Specific Test File

```bash
uv run pytest tests/test_capability_set.py -v
```

### Run with Coverage

```bash
uv add --dev pytest-cov
uv run pytest tests/ --cov=nono_py --cov-report=html
```

## Linting and Formatting

### Rust

```bash
# Check formatting
cargo fmt --check

# Auto-format
cargo fmt

# Run clippy
cargo clippy -- -D warnings
```

### Python

```bash
# Check formatting
uv run ruff format --check python/ tests/

# Auto-format
uv run ruff format python/ tests/

# Run linter
uv run ruff check python/ tests/

# Auto-fix linter issues
uv run ruff check --fix python/ tests/

# Type checking
uv run mypy python/nono_py
```

## Switching Between Local and Published nono

### For Local Development

```toml
# Cargo.toml
[dependencies]
nono = { path = "../nono/crates/nono" }
```

### For CI/GitHub Actions

The CI workflows checkout nono as a sibling directory:

```yaml
- name: Checkout nono library
  uses: actions/checkout@v6.0.2
  with:
    repository: always-further/nono
    path: nono
```

### For Release/Publishing

Before publishing to PyPI, update to use git or crates.io:

```toml
# Cargo.toml - Option 1: Git reference
[dependencies]
nono = { git = "https://github.com/always-further/nono", tag = "v0.1.0" }

# Cargo.toml - Option 2: crates.io (when nono is published)
[dependencies]
nono = "0.1"
```

## Troubleshooting

### "nono crate not found"

Ensure the path in `Cargo.toml` points to the correct location:

```bash
# Verify the path exists
ls ../nono/crates/nono/Cargo.toml
```

### Build Fails After nono Changes

If the nono library API changed:

1. Check `../nono/crates/nono/src/lib.rs` for the new API
2. Update `src/lib.rs` in nono-py to match
3. Update type stubs in `python/nono_py/_nono_py.pyi`
4. Rebuild: `uv run maturin develop`

### Import Errors in Python

If you get `ModuleNotFoundError: No module named 'nono_py._nono_py'`:

```bash
# Ensure you've built and installed
uv run maturin develop

# Verify installation
uv run python -c "from nono_py import CapabilitySet; print('OK')"
```

### Type Checking Errors

If mypy complains about missing stubs:

```bash
# The _nono_py.pyi file provides type information
# Ensure it's up to date with the Rust API
uv run mypy python/nono_py --ignore-missing-imports
```

## Release Process

1. Update version in `Cargo.toml` and `pyproject.toml`
2. Update `Cargo.toml` to use git tag or crates.io for nono dependency
3. Commit changes
4. Create and push tag: `git tag v0.1.0 && git push --tags`
5. GitHub Actions will build wheels and publish to PyPI

## Architecture Notes

### PyO3 Bindings

The Rust code in `src/lib.rs` uses PyO3 to expose the nono API to Python:

- `#[pyclass]` - Exposes a Rust struct as a Python class
- `#[pymethods]` - Exposes methods on a Python class
- `#[pyfunction]` - Exposes a standalone function
- `#[pymodule]` - Defines the module entry point

### Type Stubs

The `_nono_py.pyi` file provides type information for IDE autocompletion and mypy. Keep it in sync with the Rust API.

### Module Structure

```
nono_py/
├── __init__.py      # Re-exports from native module
├── _nono_py.pyi     # Type stubs for native module
├── _nono_py.so      # Compiled native module (generated)
└── py.typed         # PEP 561 marker
```

The native module is named `_nono_py` (with underscore) to indicate it's internal. The public API is exposed through `__init__.py`.
