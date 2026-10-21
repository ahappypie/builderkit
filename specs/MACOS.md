# Builder Kit macOS Setup Specification

> Preview override: see [MACOS_PREVIEW.md](MACOS_PREVIEW.md). The current preview
> requires an explicit release version, downloads over HTTPS without signatures
> or hash checks, and uses `~/.kit/nix/`. The remainder is a longer-term design.

## 1. Purpose

This specification defines the macOS implementation of `kit setup`, `kit update`, and `kit eject`.

Builder Kit configures the student’s existing macOS account through nix-darwin and Home Manager. It does not create a separate macOS Unix user. The student downloads only the native `kit` binary; the CLI downloads the signed Builder Kit macOS flake payload and, when needed, a pinned upstream Nix installer binary.

## 2. Supported system model

The active Builder Kit configuration is stored at:

```text
~/builderkit/
├── flake.nix
└── flake.lock
```

`flake.nix` is the only configuration file a student is expected to read or edit. `flake.lock` is retained beside it because a reproducible flake needs its complete locked dependency graph; it is Builder Kit-managed and is not a student-facing editing surface.

The local CLI data root is adjacent to the `kit` executable as defined in the host CLI specification.

Builder Kit targets the current macOS login account. Home Manager configuration is keyed to the current local username after validation. Nix-darwin performs system-level macOS configuration and therefore may request administrator authentication during activation.

## 3. Nix implementation policy

Builder Kit installs **upstream Nix**, not Determinate Nix, in v1.

The Nix installer is obtained as a separately distributed, pinned executable from the NixOS-maintained `nix-installer` project. Builder Kit does not link the installer as a Rust library in v1 and does not fetch-and-execute a shell script.

Reasons for this boundary:

- Builder Kit remains MIT-licensed and operationally independent of installer internals.
- The upstream installer remains visible, versioned, locally retained, and usable as a fallback.
- Builder Kit can verify a known installer version and digest through its own signed release manifest.
- Changes to the installer’s Rust API do not become Builder Kit compatibility obligations.

The initial supported macOS architecture set is declared per Builder Kit release manifest. No architecture is advertised as supported merely because the flake evaluates; qualification requires installation and activation testing for that architecture.

## 4. Setup state detection

Before making changes, `kit setup` determines:

1. macOS version and host architecture.
2. Current local username and home directory.
3. Whether the configuration directory already contains a managed, detached, or unrelated Builder Kit configuration.
4. Whether `nix` is absent, installed by the compatible upstream installer, installed by another method, or present but unhealthy.
5. Whether `darwin-rebuild` is available and whether it belongs to the managed Builder Kit configuration.
6. Whether sufficient disk space and network access are available for declared artifacts.

### 4.1 Existing Nix policy

| Detected state | Required behavior |
|---|---|
| Nix absent | Install through the pinned upstream installer. |
| Compatible upstream installer receipt found and Nix is healthy | Do not reinstall Nix; continue with Builder Kit configuration setup. |
| Nix appears to have been installed by another method | Stop without changing Nix. Explain that Builder Kit v1 does not modify pre-existing Nix installations. |
| Nix is present but unhealthy | Stop without automatic repair. Show the retained installer location and the installer’s supported repair/uninstall documentation path. |

Builder Kit must not infer that every Nix installation has the same daemon, configuration files, receipt, or lifecycle.

## 5. Setup flow

### 5.1 Plan

The CLI must show a plan similar to:

```text
Builder Kit setup plan
────────────────────────────────────────
Platform: macOS, Apple Silicon
Nix:      Not installed
Target:   Builder Kit for macOS

This setup will:
  1. Download and verify upstream Nix installer <version>.
  2. Install upstream Nix with flakes enabled.
  3. Download and verify Builder Kit release <version>.
  4. Create ~/builderkit/flake.nix and flake.lock.
  5. Ask for Git identity and Builder Kit preferences.
  6. Install and activate nix-darwin and Home Manager.

macOS may request your administrator password for system changes.
Proceed? [y/N]
```

The exact plan changes based on detected state. The plan must include actual payload sizes where known, expected privilege prompts, and any stop/re-run requirement.

### 5.2 Installer acquisition and retention

The CLI resolves the exact installer version from the verified Builder Kit release manifest. It retains the installer under:

```text
<data>/installers/
nix-installer-<version>-<architecture>-darwin
```

Acquisition procedure:

1. Reuse a file only when its digest matches the manifest.
2. Otherwise download to a temporary file in the installers directory.
3. Verify the SHA-256 declared by the signed Builder Kit manifest.
4. Atomically rename it to the versioned final name.
5. Set executable permissions.
6. Retain the installer after setup. Do not garbage-collect the installer that created the active Nix installation.
7. Store third-party license notices in `<data>/licenses/`.

The CLI invokes the local binary with explicit arguments, equivalent to:

```text
nix-installer install --enable-flakes
```

The final argument list is pinned and tested by Builder Kit release CI. Builder Kit’s confirmation does not suppress the upstream installer’s own confirmation in normal interactive setup.

### 5.3 Builder Kit configuration creation

After Nix is available, the CLI:

1. Downloads and verifies the macOS `flake.nix` and `flake.lock` for the selected release.
2. Creates `~/builderkit` with conservative user-only permissions if it does not exist.
3. Collects the minimal student values:
    - full name;
    - Git email;
    - optional GitHub username, if supported by the release;
    - selected Builder Kit tool preferences.
4. Determines the existing macOS username; the student does not choose a separate Linux account on macOS.
5. Writes those values only into the generated flake’s marked Builder Kit user settings block.
6. Validates the resulting flake.
7. Activates the configured Darwin target using the release-defined `darwin-rebuild switch --flake ~/builderkit#<target>` invocation.
8. Records managed installation state only after activation succeeds.

The CLI must never place tokens, passwords, private keys, API keys, or recovery codes in `flake.nix`, `flake.lock`, logs, release state, or command arguments.

## 6. Generated macOS flake requirements

The generated macOS flake is a complete, readable, single-file configuration with a student-owned settings block near the top.

It must:

- pin all inputs through an adjacent `flake.lock`;
- expose one Darwin configuration target with a stable documented name;
- import nix-darwin and Home Manager;
- configure Home Manager for the existing macOS user;
- keep shared Builder Kit choices and macOS-specific behavior visibly separated;
- contain no WSL/NixOS-only module imports or settings;
- contain stable release metadata, settings schema metadata, and managed-region integrity metadata;
- describe secrets as out of scope for the flake.

The flake may install and configure coding-agent tools, but authentication must happen through the tool’s supported interactive flow or an OS-appropriate secret mechanism outside the flake.

## 7. Update flow

`kit update` operates on `~/builderkit`.

### 7.1 Dependency release

For a dependency release, the CLI:

1. Downloads and verifies the new tested lock file.
2. Backs up the current lock file.
3. Atomically replaces only `flake.lock`.
4. Evaluates/builds the known Darwin configuration.
5. Requests confirmation if activation was not already approved.
6. Runs the known `darwin-rebuild switch` invocation.

If evaluation fails, it restores the prior lock file before returning failure, provided activation has not begun.

### 7.2 Feature release

For a feature release, the CLI:

1. Verifies the new flake, lock, release metadata, managed-region hash, and settings schema transition.
2. Parses the installed generated flake and extracts only the Builder Kit user settings attribute set.
3. Refuses to continue when the existing managed implementation region differs from the expected installed-release hash.
4. Applies an explicit settings migration when required and supported.
5. Inserts the preserved/migrated settings into the new flake.
6. Writes timestamped backups of the previous flake and lock.
7. Validates and activates the candidate configuration.

No arbitrary three-way merge is attempted.

## 8. Eject flow

`kit eject` on macOS:

1. Identifies the managed configuration at `~/builderkit`.
2. Explains that Builder Kit feature and dependency automation will stop.
3. Requires `EJECT` confirmation unless `--yes` is supported for a documented automation context.
4. Creates a timestamped backup of the current flake, lock file, and Builder Kit state.
5. Marks the installation detached in local state and removes/neutralizes update-management metadata according to the generated flake contract.
6. Leaves the active flake, lock file, Nix installation, nix-darwin state, Home Manager configuration, and user data intact.

It does not run a Nix uninstaller and does not remove nix-darwin.

## 9. Failure behavior and recovery posture

- Builder Kit does not automatically repair Nix, remove Nix, or migrate an unrelated installation.
- Setup failures retain verified installer artifacts, release payloads, and logs.
- The CLI prints the local upstream installer path and the configuration path after successful setup.
- Nix-darwin generations remain the primary activation rollback mechanism after a successful system switch.
- The product provides deterministic automation for a clean supported path, not a support commitment for arbitrary machine states.

## 10. Acceptance criteria

1. A clean supported macOS host can reach an activated nix-darwin/Home Manager Builder Kit environment after downloading only `kit` and running `kit setup`.
2. The setup plan accurately states whether Nix installation is required.
3. The precise upstream installer used is retained beside Builder Kit and verified against a signed release manifest.
4. The student’s working configuration is `~/builderkit/flake.nix`, with an adjacent lock file.
5. The macOS setup path never creates a second user account or stores credentials in the flake.
6. Existing non-Builder-Kit Nix installations cause a safe stop, not modification.
7. Dependency and feature updates follow the managed settings/managed implementation contract.
8. Ejection leaves the active developer environment intact and disables future managed updates.
