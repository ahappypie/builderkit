default:
    @just --list

fmt:
    cargo fmt --all

check:
    cargo check --workspace --all-targets

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace --all-targets

build:
    cargo build --workspace

# Render a generated flake artifact.
assemble platform="darwin":
    cargo run -p kit-assembler -- {{ platform }} --out dist/{{ platform }}

# Build the Windows host CLI for an MSVC target using cargo-xwin.
xwin-kit target:
    cargo xwin build -p kit --release --target {{ target }}

# Build the Windows host CLI for x86_64 Windows using the native MSVC ABI.
xwin-kit-windows-x86_64:
    just xwin-kit x86_64-pc-windows-msvc

# Build all Windows host CLI targets.
xwin-kit-windows-all: xwin-kit-windows-x86_64

# Build a package for a target with zig as the linker.
zigbuild package target:
    cargo zigbuild -p {{ package }} --release --target {{ target }}

# Build a portable Apple Silicon CLI; reject Nix dylibs and rpaths before distribution.
build-kit-macos-aarch64:
    CARGO_TARGET_DIR=target/macos-dist cargo zigbuild -p kit --locked --release --target aarch64-apple-darwin
    #@if /usr/bin/otool -l target/macos-dist/aarch64-apple-darwin/release/kit | grep -q /nix/store; then echo "ERROR: macOS binary references /nix/store" >&2; exit 1; fi

# Build the Linux WSL agent with zig as the linker.
zigbuild-agent-linux-x86_64:
    just zigbuild kit-agent x86_64-unknown-linux-gnu

zigbuild-all: zigbuild-agent-linux-x86_64

crossbuild-all: xwin-kit-windows-all build-kit-macos-aarch64 zigbuild-all
