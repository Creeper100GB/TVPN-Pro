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

# Unit tests (MSVC environment is loaded by the wrapper)
test:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/with-msvc.ps1 cargo test --workspace

# Debug build of the service crates
build:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/with-msvc.ps1 cargo build --workspace

# Parse-check every PowerShell script
pscheck:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/check-powershell.ps1

# UI: type check (Svelte + TypeScript)
ui-check:
    bun --cwd ui run check

# UI: production build
ui-build:
    bun --cwd ui run build

# UI: screenshots and layout assertions (needs: bun --cwd ui run preview)
ui-shots:
    bun --cwd ui run shots

# End-to-end: UI flow plus a real file download with hash check and cleanup
e2e:
    bun --cwd ui run e2e

# Log redaction gate (self-test against the fixtures)
redact:
    powershell -NoProfile -ExecutionPolicy Bypass -File tests/log-redaction/check-log-redaction.ps1 -SelfTest

# Everything CI runs, in one go
verify: fmt-check lint test pscheck redact
    @Write-Host 'verify: all checks passed'

# Remove build output
clean:
    cargo clean
