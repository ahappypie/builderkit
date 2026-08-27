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
assemble platform="darwin" out="dist/{{platform}}":
    cargo run -p kit-assembler -- {{platform}} --out {{out}}

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

# Build and ad-hoc sign the macOS host CLI for Apple Silicon with Apple's native linker.
build-kit-macos-aarch64:
    cargo build -p kit --release --target aarch64-apple-darwin

# Build the Linux WSL agent with zig as the linker.
zigbuild-agent-linux-x86_64:
    just zigbuild kit-agent x86_64-unknown-linux-gnu

zigbuild-all: zigbuild-agent-linux-x86_64

crossbuild-all: xwin-kit-windows-all build-kit-macos-aarch64 zigbuild-all
