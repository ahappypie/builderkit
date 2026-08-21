# Builder Kit Host CLI Specification

## 1. Purpose

`kit` is a small, native Rust command-line application that installs, configures, updates, and deliberately detaches Builder Kit environments. It is the only artifact a student downloads directly.

The CLI supports:

- Windows hosts, where it installs and controls a Builder Kit NixOS-WSL distribution.
- macOS hosts, where it installs upstream Nix when necessary and manages a nix-darwin/Home Manager configuration.

Builder Kit is an opinionated, managed onboarding product for technically interested but non-specialist users. It must make the supported path reliable and understandable without turning into a general-purpose Nix administration tool.

## 2. Product principles

1. **One user download.** Students download a platform-native `kit` executable, not a collection of scripts, WSL images, Nix files, or installers.
2. **Plan before action.** `setup` and mutating updates display a concise, concrete plan and require confirmation unless `--yes` is supplied.
3. **Declarative final state.** User accounts, packages, dotfiles, and OS configuration are applied by NixOS, nix-darwin, and Home Manager. The CLI collects data, safely updates the managed settings region, and invokes known activation paths; it does not imperatively create users or install packages.
4. **Small public surface.** The v1 command tree is `setup`, `update`, and `eject`. Advanced implementation details remain internal.
5. **Verified release payloads.** The CLI trusts a Builder Kit release signing key embedded at build time. It downloads only artifacts authorized by a signed release manifest and verifies their digests before use.
6. **Safe ownership boundary.** Personal settings belong to the student; generated Builder Kit implementation belongs to Builder Kit while the installation is managed. `eject` makes the handoff explicit.
7. **Conservative coexistence.** The CLI must not alter an existing, unrelated Nix installation or an unmanaged WSL distribution in v1.

## 3. Scope and non-goals

### In scope

- Host detection and prerequisite checks.
- Interactive installation planning and confirmation.
- Downloading, caching, and verifying Builder Kit release artifacts.
- macOS upstream Nix installation through a pinned, separately distributed upstream installer binary.
- WSL enablement checks and Builder Kit WSL installation.
- Managed configuration updates and activation.
- Roll-forward updates and explicit ejection.
- Clear logs, backups, diagnostics, and actionable failures.

### Out of scope

- Supporting arbitrary existing Nix installations.
- General-purpose Nix troubleshooting, repair, or migration.
- Managing secrets, API keys, SSH private keys, passwords, or cloud credentials.
- Arbitrary remote command execution.
- Merging user edits made to Builder Kit-managed flake implementation code.
- Uninstalling Nix, nix-darwin, WSL, or a user’s WSL distribution through `eject`.
- Guarantees of coding-agent performance, availability, suitability, or downstream package compatibility.

## 4. Supported command surface

```text
kit setup [--dry-run] [--yes] [--channel <stable|edge>]
kit update [--check] [--yes]
kit eject [--yes]
kit --version
kit --help
```

`--yes` bypasses Builder Kit’s own confirmation prompt but must not bypass an upstream installer’s confirmation unless a future explicit noninteractive deployment mode is designed and reviewed.

### 4.1 `kit setup`

`setup` identifies the host and current state, prints an installation plan, asks for confirmation, and runs the supported platform setup flow.

It must:

1. Determine Windows versus macOS and host architecture.
2. Find the executable’s canonical directory and derive the local Builder Kit data directory from it.
3. Detect existing Builder Kit state and reject ambiguous or unsupported states rather than guessing.
4. Fetch and verify the current selected-channel release manifest.
5. Select the platform- and architecture-matched payloads.
6. Print the exact intended changes, downloads, privilege prompts, and any required restart/re-run point.
7. Require confirmation.
8. Delegate to the platform implementation.
9. Record only non-secret installation state after successful setup.

### 4.2 `kit update`

`update` checks the installed environment against the selected release channel and applies an eligible tested release after confirmation.

It handles two release classes:

- **Dependency update:** replaces the tested `flake.lock` only, then evaluates/builds and activates the existing configuration.
- **Feature update:** replaces Builder Kit-managed `flake.nix` implementation while preserving or migrating the student settings block, updates `flake.lock`, validates, and activates.

The command must distinguish these classes in user output. A lock update is not described as a harmless chore: it can change package versions and behavior.

### 4.3 `kit eject`

`eject` permanently stops Builder Kit from managing the current environment. It preserves the working flake, lock file, and a timestamped backup; it disables managed updates; and it removes or neutralizes Builder Kit ownership metadata as defined by the platform specifications.

Before executing, it must explain that the student becomes responsible for Nix configuration, dependency updates, and recovery. Interactive use requires the literal confirmation text `EJECT`.

It must not delete a WSL distribution, Nix installation, user files, or coding environment.

## 5. Installation directories and state

The CLI data root is adjacent to the canonical executable path:

```text
<executable-directory>/data/
```

The current working directory must never determine this location.

Expected layout:

```text
data/
├── installers/       # retained, versioned upstream Nix installers on macOS
├── releases/         # immutable verified release payloads, by Builder Kit version
├── cache/            # resumable temporary downloads; safe to discard
├── backups/          # generated-flake and lock-file backups
├── logs/             # non-secret operation logs
├── licenses/         # third-party notices, including installer licensing
└── state/
    └── installation.json
```

`installation.json` contains only operational metadata: schema version, channel, platform, Builder Kit release, configuration location, managed/detached status, selected payload versions and hashes, and protocol compatibility information. It must never contain tokens, passwords, private keys, or other secrets.

Release directories are immutable after verification. Files are downloaded to a temporary name, verified, then atomically renamed into their final path.

## 6. Trust and release model

### 6.1 Embedded trust root

Each CLI binary embeds one or more Builder Kit public release-verification keys. The CLI verifies a versioned release manifest before trusting any payload URL, checksum, release version, or compatibility declaration.

### 6.2 Signed release manifest

At minimum, a manifest declares:

```text
- release version and channel
- publication timestamp
- supported host platform and architecture payloads
- SHA-256 digest and size of every payload
- WSL image release and agent protocol version
- macOS flake and lock artifact versions
- upstream Nix installer version, URL, architecture, and digest
- supported settings schema range
- minimum CLI version
- update classification: dependency or feature
- release notes and migration identifier, if applicable
```

The manifest signature is verified before payload download. Every payload digest is verified before use. The CLI must fail closed on signature, digest, schema, or protocol incompatibility.

### 6.3 Release channels

- `stable` is the default student channel. It receives only tested, promoted releases.
- `edge` is for maintainers and early testers.

Changing channels is out of scope for the initial student-facing command set; it may be an explicit documented advanced action later.

## 7. Managed configuration contract

Generated flakes contain a designated user-owned settings attribute set delimited by stable markers:

```nix
# BEGIN BUILDER-KIT USER SETTINGS
builderKit = {
  settingsSchemaVersion = 1;
  # student-owned values
};
# END BUILDER-KIT USER SETTINGS
```

The rest of the generated flake is Builder Kit-managed implementation code. The CLI may automatically update only a file whose managed region matches the expected installed-release hash.

If changes are detected outside the user settings region, a feature update stops without modifying anything. The CLI explains that only dependency updates remain safe until the user restores the managed implementation or ejects.

The current settings schema version is recorded in the flake and the local installation state. Feature releases may provide explicit, versioned settings migrations. A migration that cannot be safely applied must stop before writing changes.

## 8. Error handling, logging, and rollback

- All writes to flake, lock, and state files use temporary files plus atomic replacement.
- Before every replacement, the CLI saves a timestamped backup.
- The CLI validates candidate configuration before activation.
- Failed setup leaves retained downloads and logs for inspection but does not mark installation complete.
- Failed updates restore no files automatically after activation has begun; NixOS/nix-darwin generation rollback remains the platform-native recovery path.
- Output must identify the failed operation, artifact version, local log location, and the next supported action.

The CLI should offer concise student-facing output by default and retain detailed non-secret diagnostics in `logs/`.

## 9. Implementation constraints

- Language: Rust.
- License: MIT for Builder Kit-owned source and binaries.
- CLI architecture: platform adapters behind a common command and release-verification layer.
- The CLI uses no shell-string interpolation for privileged or cross-platform operations; it invokes commands with explicit argument vectors.
- Network downloads require HTTPS and verified release metadata.
- The host CLI must not include a generic “execute arbitrary command in WSL” capability.
- External dependencies that are separately distributed retain their own licenses and notices.

## 10. Acceptance criteria

1. A new supported Windows or macOS user can download one native Builder Kit binary and run `kit setup`.
2. Setup prints a plan before changing the machine.
3. Every downloaded executable/configuration payload is authorized by a verified signed manifest and matches its digest.
4. Successful setup leaves a readable, pinned generated flake and lock file in the platform-defined location.
5. `kit update --check` reports availability without changing state.
6. Dependency releases replace only the lock file; feature releases preserve/migrate user settings and reject managed-region modifications.
7. `kit eject` preserves the environment and permanently disables Builder Kit-managed configuration updates.
8. No normal user flow requires students to understand Nix module layout, manually use `useradd`, manually import WSL tarballs, or edit a lock file.
