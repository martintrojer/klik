# CI/CD Pipeline Documentation

This document describes the modern Rust CI/CD pipeline for thokr.

## Workflows

### `ci.yml` - Continuous Integration
**Triggers**: Push and PR to main branch

**Jobs**:
- **test**: Multi-platform testing (Ubuntu, Windows, macOS) with stable and beta Rust
- **format**: Code formatting verification with `cargo fmt`
- **lint**: Linting with `cargo clippy` (warnings as errors)
- **coverage**: Code coverage analysis with cargo-tarpaulin and Codecov reporting
- **security**: Security audit with `cargo audit`
- **minimal-versions**: Testing with minimal dependency versions
- **msrv**: Minimum Supported Rust Version compatibility check

**Features**:
- Efficient Rust caching with `Swatinem/rust-cache`
- Modern actions (checkout@v4, dtolnay/rust-toolchain)
- Cross-platform compatibility validation
- Comprehensive code quality enforcement

### `release.yml` - Release Automation
**Triggers**: Release published

**Jobs**:
- **publish**: Automated publishing to crates.io
- **build-binaries**: Cross-platform binary generation for multiple targets:
  - Linux x86_64 (glibc and musl)
  - Windows x86_64
  - macOS x86_64 and aarch64 (Apple Silicon)

**Features**:
- Automatic binary stripping for size optimization
- Release asset upload for all supported platforms
- Pre-publish verification with full test suite

## Security

- No deprecated actions (migrated from actions-rs/*)
- Secure secret management for crates.io and GitHub tokens
- Security audit integration with cargo-audit
- Modern action versions with latest security patches

## Performance

- Rust caching reduces build times significantly
- Matrix builds for efficient parallel execution
- Optimized job dependencies and resource usage
- Smart exclusions to reduce CI load while maintaining coverage