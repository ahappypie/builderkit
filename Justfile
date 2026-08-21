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

# Render the basic generated flake artifact.
assemble out="dist":
    cargo run -p kit-assembler -- release --out {{out}}

# Build the Windows host CLI for an MSVC target using cargo-xwin.
xwin-kit target:
    cargo xwin build -p kit --release --target {{target}}

# Build the Windows host CLI for x86_64 Windows using the native MSVC ABI.
xwin-kit-windows-x86_64:
    just xwin-kit x86_64-pc-windows-msvc

# Build all Windows host CLI targets.
xwin-kit-windows-all: xwin-kit-windows-x86_64

# Build a package for a target with zig as the linker.
zigbuild package target:
    cargo zigbuild -p {{package}} --release --target {{target}}

# Build the macOS host CLI for Apple Silicon with zig as the linker.
zigbuild-kit-macos-aarch64:
    just zigbuild kit aarch64-apple-darwin

# Build the Linux WSL agent with zig as the linker.
zigbuild-agent-linux-x86_64:
    just zigbuild kit-agent x86_64-unknown-linux-gnu

zigbuild-all: zigbuild-kit-macos-aarch64 zigbuild-agent-linux-x86_64

crossbuild-all: xwin-kit-windows-all zigbuild-all
