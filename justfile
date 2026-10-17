# Mesh - Modern 3D Mesh & Model Viewer
# Task runner configuration (just)

# Display available tasks and usage
default:
    @just --list

# ==============================================================================
# Development
# ==============================================================================

# Run application in development mode
dev *args="":
    cargo run -- {{args}}

# Run application with demo asset (Suzanne)
dev-demo:
    cargo run -- assets/models/Suzanne.glb

# ==============================================================================
# Quality, Verification & Testing
# ==============================================================================

# Run compiler checks
check:
    cargo check

# Run unit tests
test:
    cargo test

# Validate code formatting and lint rules (clippy)
lint:
    cargo fmt --check
    cargo clippy -- -D warnings

# Auto-format code across the project (rustfmt)
format:
    cargo fmt

# ==============================================================================
# Build & Distribution
# ==============================================================================

# Build release binary into dist/linux/
build-linux:
    cargo build --release
    @mkdir -p dist/linux
    @cp -f target/release/mesh dist/linux/
    @echo "=========================================="
    @echo "Linux build complete! Output in dist/linux/"
    @echo " - dist/linux/mesh"
    @echo "=========================================="

# Cross-compile Windows binary into dist/windows/ (x86_64-pc-windows-gnu)
build-windows:
    cargo build --release --target x86_64-pc-windows-gnu
    @mkdir -p dist/windows
    @cp -f target/x86_64-pc-windows-gnu/release/mesh.exe dist/windows/
    @echo "=========================================="
    @echo "Windows build complete! Output in dist/windows/"
    @echo " - dist/windows/mesh.exe"
    @echo "=========================================="

# Build release binary and macOS application bundle (requires macOS host)
build-macos:
    #!/usr/bin/env bash
    if [ "$(uname -s)" = "Darwin" ]; then \
        cargo build --release && \
        mkdir -p dist/macos/Mesh.app/Contents/MacOS dist/macos/Mesh.app/Contents/Resources && \
        cp -f target/release/mesh dist/macos/ && \
        cp -f target/release/mesh dist/macos/Mesh.app/Contents/MacOS/ && \
        sed "s/VERSION_PLACEHOLDER/$$(grep -m1 '^version' Cargo.toml | cut -d '\"' -f2)/g" packaging/macos/Info.plist > dist/macos/Mesh.app/Contents/Info.plist && \
        echo -n "APPL????" > dist/macos/Mesh.app/Contents/PkgInfo && \
        (cp -f packaging/macos/assets/mesh.icns dist/macos/Mesh.app/Contents/Resources/ 2>/dev/null || true) && \
        echo "==========================================" && \
        echo "macOS build complete! Output in dist/macos/" && \
        echo " - dist/macos/mesh" && \
        echo " - dist/macos/Mesh.app" && \
        echo "=========================================="; \
    else \
        echo "Notice: Compiling for macOS requires running on macOS or configuring an osxcross SDK." >&2; \
        exit 1; \
    fi

# Alias for build-macos
build-mac: build-macos

# Default release build for current host platform (Linux)
build: build-linux

# Build all supported multi-platform release binaries
build-all: build-linux build-windows

# ==============================================================================
# Maintenance & Utilities
# ==============================================================================

# Generate application icon assets (target: all, linux, macos, or windows)
icons target="all":
    ./packaging/scripts/generate-icons.sh --{{target}}

# Remove build artifacts and temporary compilation directories
clean:
    rm -rf dist dist-windows mesh.iconset
    cargo clean

# Synchronize project version across manifests and packaging scripts (e.g., just version 0.2.0)
version new_version="":
    #!/usr/bin/env bash
    NEW_V="{{new_version}}"
    NEW_V="${NEW_V#v}"
    if [ -z "$NEW_V" ]; then
        echo "Error: Missing version. Ex: just version 1.2.3"
        exit 1
    fi
    awk -v new_ver="$NEW_V" '/^\[package\]/ {p=1} /^version =/ && p {sub(/".*"/, "\"" new_ver "\""); p=0} 1' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml
    if [ -f packaging/windows/mesh.nsi ]; then
        sed -i -E 's/(!define VERSION ")[^"]*(")/\1'"$NEW_V"'\2/' packaging/windows/mesh.nsi
    fi
    cargo check --quiet
    printf "Version synchronized to %s\n" "$NEW_V"

# ==============================================================================
# Release Automation (Local testing via act)
# ==============================================================================

# Run GitHub Actions release workflow locally via act (target: linux, windows, mac, or all)
release target="":
    #!/usr/bin/env bash
    if ! command -v act &> /dev/null; then
        echo "Error: 'act' is not installed. Please install it to run the release workflow locally."
        exit 1
    fi
    TARGET="{{target}}"
    mkdir -p ./dist
    if [ -z "$TARGET" ]; then
        echo "Running Linux and Windows release workflows locally via act..."
        act release --artifact-server-path ./dist \
            -W .github/workflows/release-linux.yml \
            -W .github/workflows/release-windows.yml \
            -P windows-latest=catthehacker/ubuntu:act-latest \
            -P ubuntu-24.04-arm=catthehacker/ubuntu:act-latest
    elif [ "$TARGET" = "linux" ]; then
        echo "Running Linux release workflow..."
        act release --artifact-server-path ./dist \
            -W .github/workflows/release-linux.yml \
            -P ubuntu-24.04-arm=catthehacker/ubuntu:act-latest
    elif [ "$TARGET" = "windows" ]; then
        echo "Running Windows release workflow..."
        act release --artifact-server-path ./dist \
            -W .github/workflows/release-windows.yml \
            -P windows-latest=catthehacker/ubuntu:act-latest
    elif [ "$TARGET" = "mac" ] || [ "$TARGET" = "macos" ]; then
        echo "Note: act on Linux cannot emulate macOS runners (no macOS Docker images)."
        echo "Running macOS release workflow test if macos runner is available..."
        act release --artifact-server-path ./dist \
            -W .github/workflows/release-macos.yml || true
    else
        echo "Error: Unknown target '$TARGET'. Use 'linux', 'windows', 'mac', or leave empty for all."
        exit 1
    fi
