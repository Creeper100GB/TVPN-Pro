set shell := ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

# Default: show available targets
default:
    @just --list

# Format Rust code
fmt:
    cargo fmt --all
    cargo fmt --manifest-path ui/src-tauri/Cargo.toml

# Check formatting without writing
fmt-check:
    cargo fmt --all -- --check

# Desktop shell (separate workspace): formatting
shell-fmt-check:
    cargo fmt --manifest-path ui/src-tauri/Cargo.toml -- --check

# Lint with clippy; warnings are errors
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Unit tests (MSVC environment is loaded by the wrapper)
test:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/with-msvc.ps1 cargo test --workspace

# Desktop shell (separate workspace): lint
shell-lint:
    cargo clippy --locked --manifest-path ui/src-tauri/Cargo.toml --all-targets -- -D warnings

# Desktop shell: unit tests
shell-test:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/with-msvc.ps1 cargo test --locked --manifest-path ui/src-tauri/Cargo.toml

# Debug build of the service crates
build:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/with-msvc.ps1 cargo build --workspace

# Parse-check every PowerShell script
pscheck:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/check-powershell.ps1

# UI: type check (Svelte + TypeScript)
ui-check:
    bun --cwd=ui run check

# UI: production build
ui-build:
    bun --cwd=ui run build

# UI: unit tests (pure helpers such as the teardown rules)
ui-test:
    bun --cwd=ui run test

# UI: screenshots and layout assertions (needs: bun --cwd=ui run preview)
ui-shots:
    bun --cwd=ui run shots

# End-to-end: UI flow plus a real file download with hash check and cleanup
e2e:
    bun --cwd=ui run e2e

# Log redaction gate (self-test against the fixtures)
redact:
    powershell -NoProfile -ExecutionPolicy Bypass -File tests/log-redaction/check-log-redaction.ps1 -SelfTest

# Winget: validate the local 0.1.6 manifest (requires winget)
winget-validate:
    winget validate --manifest packaging/winget/manifests/c/Creeper100GB/TVPNPro/0.1.6

# Everything CI runs except the Playwright E2E job, in one go (`just e2e` runs that)
verify: fmt-check shell-fmt-check lint shell-lint test shell-test pscheck redact ui-test ui-check ui-build
    @Write-Host 'verify: all checks passed'

# Remove build output
clean:
    cargo clean
