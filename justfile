set shell := ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

# Default: show available targets
default:
    @just --list

# Format Rust code
fmt:
    cargo fmt --all

# Check formatting without writing
fmt-check:
    cargo fmt --all -- --check

# Lint with clippy; warnings are errors
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Unit tests
test:
    cargo test --workspace

# Parse-check every PowerShell script
pscheck:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/check-powershell.ps1

# Log redaction gate (self-test against the fixtures)
redact:
    powershell -NoProfile -ExecutionPolicy Bypass -File tests/log-redaction/check-log-redaction.ps1 -SelfTest

# Everything CI runs, in one go
verify: fmt-check lint test pscheck redact
    @Write-Host 'verify: all checks passed'

# Remove build output
clean:
    cargo clean
