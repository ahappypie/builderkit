# macOS preview runbook

This preview uses explicit release versions and fixed GitHub HTTPS URLs. It has
no manifests, channels, automatic latest selection, signing keys, signature
verification, or hash checks. It trusts GitHub/HTTPS and anyone able to replace
release assets. Use a test VM when evaluating the preview.

This runbook supersedes the signed-release requirements in the longer-term host
and platform specifications for this preview only.

## Build and publish

The CLI and configuration bundle have independent release lifecycles:

- `.github/workflows/kit-release.yml`: `kit/MAJOR.MINOR.PATCH` tags build and
  publish only `kit-darwin-aarch64`. The SemVer suffix must match the version in
  `rust/kit/Cargo.toml`. Tags such as `kit/0.0.3-preview.1` create GitHub
  prereleases.
- `.github/workflows/darwin-release.yml`: `bundle/YYYY.MM.PATCH` tags assemble, check,
  and publish the Darwin flake bundle, independently of the CLI.

The configuration archive remains at:

```text
https://github.com/ahappypie/builderkit/releases/download/bundle%2F<calver>/builderkit-darwin-<calver>.tar.gz
```

Build the standalone CLI locally with the same recipe used by CI:

```sh
just build-kit-macos-aarch64
```

The output is `target/macos-dist/aarch64-apple-darwin/release/kit`. The recipe
uses zigbuild. CI runs the workspace tests natively on Linux, copies the
cross-built binary to `dist/kit-darwin-aarch64`, checks that it is an ARM64
Mach-O executable without executing it, and publishes it under the SemVer tag.
Runtime testing belongs in the macOS VM. CI does not assemble or publish a
flake bundle.

There are no signing, 1Password, manifest, or binary checksum steps. The
`--version` argument to setup/update selects a **CalVer configuration release**,
not the CLI's SemVer version. For example, `kit setup --version 2026.09.0`
selects tag `bundle/2026.09.0`; the slash is URL-encoded in download URLs. Archive
filenames and local cache directories retain the bare CalVer value. Old unprefixed
release tags are not used as a fallback.

Apple Developer ID signing and notarization are not part of this preview. The
linker's normal ad-hoc ARM64 signature does not require a developer account.

## VM gate 1: launch

Start with a clean native Apple Silicon macOS VM, administrator access, internet
access, and no Nix installation. Snapshot it before making changes.

Download the binary from the chosen prerelease, then run:

```sh
chmod +x ./kit
./kit --version
./kit --help
./kit setup --help
```

The binary should launch without Nix installed. Setup help must require
`--version`. Do not install developer tools or Nix to work around launch failures.

## VM gate 2: dry-run

The examples below assume bundles `bundle/2026.08.0` and `bundle/2026.09.0`
have been published. Use the older bundle if testing the update path next:

```sh
./kit setup --version 2026.08.0 --dry-run
```

Expected: the plan names the exact release and download URLs. No downloads,
confirmation prompts, privilege prompts, or filesystem changes occur.

## VM gate 3: downloads only

```sh
./kit setup --version 2026.08.0 --download-only
```

Expected:

- Confirmation precedes downloads.
- The requested configuration archive is downloaded to
  `~/.kit/releases/2026.08.0/darwin.tar.gz`.
- When Nix is absent, the upstream installer is downloaded to `~/.kit/installer/`.
- The installer release is pinned in `rust/kit/src/release.rs` (currently
  `NixOS/nix-installer` version `2.35.3`), not selected through a latest endpoint.
- Configuration archives accept only bounded regular `flake.nix` and `flake.lock`
  files, rejecting extra entries, symlinks, duplicates, and truncated data.
- No installer runs, no identity or administrator prompt appears, and `/nix`,
  `~/.kit/nix`, `~/.kit/state/installation.json`, and `~/.local/bin/kit` remain absent.
- Repeating the command redownloads files rather than trusting unsigned cache
  contents. Failed transfers do not replace existing completed downloads.

Stop and review the output before testing installation.

## VM gate 4: installation

```sh
./kit setup --version 2026.08.0
```

Review the plan and the second confirmation after downloads. The upstream
installer retains its own confirmation even when Builder Kit's `--yes` is used.
After activation succeeds, open a new terminal and check:

```sh
kit --version
nix --version
git config --get user.name
git config --get user.email
```

`kit --version` identifies the CLI build, not the installed configuration release.
The selected configuration version is recorded in
`~/.kit/state/installation.json` only after activation succeeds.

## VM gate 5: explicit update

```sh
kit update --version 2026.09.0 --dry-run
kit update --version 2026.09.0 --check
kit update --version 2026.09.0
```

Dry-run changes nothing. Check tests availability without downloading the archive
or modifying files. Applying an update downloads that exact release, preserves
user settings, saves backups, validates, and activates. The installed version is
updated only after successful activation. Requesting the installed version is a
successful no-op; downgrades are not supported.

## Not yet qualified

Clean installation, installer compatibility, Gatekeeper behavior, conflicting
configurations, interrupted-activation recovery, and existing-Nix ownership still
need VM testing. Ejection is not implemented. Do not treat a passed download gate
as a passed installation or recovery gate.
