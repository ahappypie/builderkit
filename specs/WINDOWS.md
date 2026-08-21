# Builder Kit Windows and WSL Specification

## 1. Purpose

This specification defines Builder Kit behavior on Windows, including the Windows host CLI, the downloadable Builder Kit WSL image, and the Linux-resident Builder Kit agent included in that image.

Students download only the native Windows `kit.exe` (likely via Scoop, the Windows package manager). The CLI detects WSL prerequisites, downloads and verifies the correct Builder Kit `.wsl` image, installs it, and coordinates first-time declarative configuration through the internal agent. The `.wsl` image remains a complete portable distribution artifact, but is not a separate required student download.

## 2. Architecture

```text
Windows host
└── kit.exe
    ├── verifies a signed Builder Kit release manifest
    ├── downloads and verifies Builder-Kit-<arch>.wsl
    ├── invokes wsl.exe for installation and lifecycle operations
    └── invokes a narrow local protocol through wsl.exe
        └── Builder Kit WSL distribution
            ├── /etc/nixos/flake.nix
            ├── /etc/nixos/flake.lock
            └── kit-agent
```

The host CLI is the Windows control plane. The agent is part of the verified WSL image and is never separately downloaded, copied into the distribution, or remotely exposed.

## 3. WSL image requirements

The Builder Kit WSL image is a NixOS-WSL tarball packaged as a `.wsl` distribution artifact. It is built from the generated WSL flake and includes:

```text
/etc/nixos/flake.nix
/etc/nixos/flake.lock
/etc/wsl.conf
/etc/wsl-distribution.conf        # when required for modern WSL distribution metadata
/usr/bin or profile path/kit-agent
```

### 3.1 Configuration inclusion

The NixOS-WSL build configuration must set:

```nix
wsl.tarball.configPath = ./.;
```

This is mandatory. The image must receive a writable copy of the generated flake and lock file at `/etc/nixos`, rather than only an opaque generated NixOS configuration.

### 3.2 WSL and systemd settings

The image enables NixOS-WSL and systemd as required by the selected NixOS-WSL release. Its distribution metadata may support direct Explorer installation, but Builder Kit’s supported setup path uses `wsl.exe` so the host CLI can verify and orchestrate installation.

### 3.3 Bootstrap account

The image contains a minimal internal bootstrap/recovery account. It is an implementation detail, not the student’s permanent account and not part of normal documentation.

The bootstrap account must be sufficient to perform the initial declarative rebuild. It must not rely on a shared plaintext password. Its exact access design is part of the NixOS image security review and must support the agent’s controlled first-run workflow.

The account is retained as a recovery path by default. Builder Kit does not claim that deleting its declaration automatically removes it unless the NixOS user-management policy explicitly makes that behavior true and the resulting recovery implications are accepted.

## 4. Host prerequisite detection

Before making changes, `kit setup` determines:

1. Windows version, architecture, and process elevation state.
2. Whether WSL is installed, enabled, current enough for the supported custom-distribution workflow, and configured for WSL 2.
3. Whether required Windows virtualization features are enabled.
4. Whether a Builder Kit distribution with the target name already exists.
5. Whether another installation state is incomplete, managed, detached, or ambiguous.
6. Available disk space for image download, WSL import, and Nix store growth.

The CLI must never unregister, overwrite, or modify another WSL distribution without an explicit future migration feature.

## 5. Windows setup flow

### 5.1 Plan

The CLI prints a plan before changing the system. Example:

```text
Builder Kit setup plan
────────────────────────────────────────
Platform: Windows 11, x86_64
WSL:      Not installed
Target:   Builder Kit for WSL

This setup will:
  1. Enable WSL 2 prerequisites.
  2. Ask Windows to restart. Run this command once more afterward.
  3. Download and verify Builder Kit WSL image <version>.
  4. Install the image as the Builder-Kit distribution.
  5. Ask for your name, Git email, and Linux username.
  6. Configure your NixOS and Home Manager environment declaratively.

Administrator approval is required for Windows system changes.
Proceed? [y/N]
```

When WSL is absent or needs a Windows restart, the CLI performs only the prerequisite step after confirmation, reports why setup cannot safely continue, and exits with a distinct “restart required” status. The student reruns `kit setup` after restart.

### 5.2 Image installation

When prerequisites are ready, the CLI:

1. Selects the architecture-matched `.wsl` image from the verified Builder Kit release manifest.
2. Downloads it into a temporary release-cache location adjacent to the CLI.
3. Verifies its digest against the signed manifest.
4. Installs the custom distribution through the supported `wsl.exe` custom-distribution installation path.
5. Uses a stable distribution name, such as `Builder Kit`, unless an existing managed installation requires a release-defined naming strategy.
6. Starts the new distribution only through known commands and waits for the agent handshake.

The CLI records the selected image release and agent protocol version only after the installed agent reports a compatible handshake.

## 6. Identity collection and declarative user creation

The Windows host CLI collects the minimum student values after the image is installed:

- proposed Linux username, defaulting to a validated Windows account name where available;
- full name;
- Git email;
- optional supported profile/tool choices.

The CLI validates a proposed Linux username against the release-defined conservative username policy. It must not blindly use a domain-qualified `whoami` value.

The host sends these values to the WSL agent through the versioned local protocol. The agent updates only the marked `builderKit` user settings region in `/etc/nixos/flake.nix` and lets NixOS/Home Manager create the account and home environment.

Forbidden first-run shortcuts include:

- `useradd` or `adduser`;
- `passwd` or an embedded shared password;
- imperative package installation;
- direct unmanaged dotfile creation;
- editing arbitrary configuration paths supplied by the host.

## 7. Default-user migration sequence

Changing the configured WSL launch user has NixOS-WSL-specific activation semantics. The agent must use a staged, tested migration rather than treating this as an ordinary `nixos-rebuild switch` change.

### 7.1 Stage 1: create the student account

The agent writes the student’s account and Home Manager configuration to the flake while retaining the bootstrap account and current bootstrap default user. It validates and applies this generation using the known NixOS rebuild path.

This stage creates the student account declaratively and allows the build to fail while the known bootstrap entry path remains valid.

### 7.2 Stage 2: change the WSL default user

After Stage 1 succeeds, the agent updates the same flake so `wsl.defaultUser` points to the new account. It applies this change using the NixOS-WSL-required boot activation path rather than relying on `switch` alone.

The host CLI then performs the required WSL lifecycle sequence for the selected NixOS-WSL release: terminate the distribution, perform the documented root launch/exit operation if required, terminate again, and relaunch normally. The exact commands are version-tested and owned by the agent/host compatibility contract.

Only after normal relaunch verifies the intended user does the CLI mark initial setup successful.

### 7.3 Recovery behavior

If either stage fails:

- the CLI does not remove the bootstrap path;
- the agent returns a structured failure with the generation/build details;
- the host prints the recovery distribution command and log locations;
- no further automatic mutation occurs.

## 8. Linux agent specification

### 8.1 Distribution and build

`kit-agent` is a Linux-only Rust binary in the same source workspace as the host CLI. The WSL image build compiles it from local repository source with Nix, conceptually:

```nix
pkgs.rustPlatform.buildRustPackage {
  pname = "kit-agent";
  src = ../.;
  cargoLock.lockFile = ../Cargo.lock;
  cargoBuildFlags = [ "-p" "kit-agent" ];
}
```

The exact source revision is part of the WSL image build input. The resulting agent is included in the NixOS system closure and therefore covered by the verified WSL image artifact digest. It has no independent network installation or update channel.

### 8.2 Protocol

The host invokes the agent as a one-shot process through `wsl.exe`. The agent does not listen on TCP, named pipes, Unix sockets exposed to the host, or any network interface.

Transport is newline-delimited JSON over standard input/output. The protocol is versioned and intentionally narrow.

Required operations:

```text
handshake
configure-initial-user
apply-managed-update
eject
```

A handshake returns at least:

```json
{
  "protocolVersion": 1,
  "agentVersion": "0.1.0",
  "imageRelease": "0.1.0",
  "flakeSchemaVersion": 1
}
```

The host refuses to proceed outside the supported protocol/schema range. The agent refuses unrecognized operations and invalid request fields.

The agent must not expose:

- arbitrary command execution;
- arbitrary file reads or writes;
- arbitrary Nix evaluation or build arguments;
- arbitrary user selection;
- any network service.

### 8.3 File-write boundary

The agent may modify only the generated configuration and state paths explicitly defined by the release contract, primarily:

```text
/etc/nixos/flake.nix
/etc/nixos/flake.lock
/etc/builderkit/      # if a small agent-local non-secret state directory is required
```

It validates the managed-region hash before feature replacement, writes backups, uses atomic file replacement, and validates candidate configuration before activation.

## 9. Windows updates

The Windows host CLI owns release retrieval and verification. The agent owns NixOS-local configuration edits and rebuild sequences.

For dependency updates, the host provides the verified lock artifact and the agent replaces only `/etc/nixos/flake.lock`, validates, and activates the known configuration.

For feature updates, the host provides the verified flake and lock artifacts plus schema/migration metadata. The agent extracts student settings, rejects unexpected managed-region changes, applies a supported migration if needed, writes backups, validates, and activates.

The host and agent must explicitly report whether the update is dependency-only or a feature update.

## 10. Windows eject

`kit eject` invokes the agent after host-side managed-state checks.

The agent:

1. backs up the current flake and lock file;
2. marks the distribution configuration detached;
3. disables future Builder Kit-managed modifications;
4. leaves the NixOS configuration, Home Manager environment, user accounts, distribution, and user data running.

The host records that this distribution is detached and refuses future managed updates. Eject does not invoke `wsl --unregister`, does not delete the WSL image cache automatically, and does not uninstall NixOS.

## 11. Acceptance criteria

1. On a clean supported Windows host, a student downloads only `kit.exe` (via Scoop) and can reach a configured Builder Kit WSL environment through `kit setup`.
2. Missing WSL prerequisites produce an accurate plan, request required elevation, and stop cleanly for restart/re-run when necessary.
3. The `.wsl` image, host CLI, release manifest, and baked agent have a verified release relationship.
4. `/etc/nixos/flake.nix` and `/etc/nixos/flake.lock` exist as editable, writable configuration artifacts in the installed distribution.
5. The real Linux user is created by NixOS configuration and configured by Home Manager, not imperatively.
6. Default-user migration follows the staged NixOS-WSL flow and preserves recovery access on failure.
7. The agent accepts only a narrow versioned local protocol and cannot serve as a general WSL remote-execution channel.
8. Managed updates and ejection preserve student data and never unregister the distribution.
