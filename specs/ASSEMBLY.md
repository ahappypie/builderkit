# Builder Kit Flake Assembly Specification

## 1. Purpose

Builder Kit maintainers need normal modular Nix source organization while students need one readable configuration file. This specification defines a native, typed release assembler that compiles the maintainer source tree into two release-ready generated flakes:

- a WSL/NixOS flake embedded in the Builder Kit WSL image;
- a macOS/nix-darwin flake downloaded by the Builder Kit CLI.

The assembler is a Rust tool. It does not attempt to implement the Nix module system or perform general semantic Nix-module merging.

## 2. Product contract

### 2.1 Maintainer experience

Maintainers author Builder Kit in separated, reviewable source fragments with clear platform boundaries:

```text
common/                 # shared Home Manager and product behavior
platforms/wsl/          # NixOS-WSL and WSL agent/image settings
platforms/darwin/       # nix-darwin and macOS settings
fragments/              # deliberate embed-ready Nix sections
release.toml            # typed release contract
```

### 2.2 Student experience

Students receive a generated complete flake with one obvious personalization location:

```nix
# BEGIN BUILDER-KIT USER SETTINGS
builderKit = {
  settingsSchemaVersion = 1;
  # student-owned values
};
# END BUILDER-KIT USER SETTINGS
```

The generated output is deliberately formatted, commented, and readable. It must never be minified, obfuscated, or a custom Builder Kit DSL.

The user settings block is the only region Builder Kit’s setup and update automation preserves/migrates. The remaining flake content is versioned managed implementation.

## 3. Technology choice

The assembler is a Rust CLI, for example `kit-assembler`.

Recommended components:

- `serde` and `toml` for a typed release manifest;
- Askama or an equivalent compile-time checked Rust template engine for full-file rendering;
- `rnix-parser` for post-render Nix syntax parsing, source-aware validation, and future settings-block transformations;
- normal Rust error types and structured diagnostics;
- external `nix` invocations for actual Nix evaluation and builds.

`rnix-parser` is not the semantic composition engine. Nix modules can contain functions, dynamic imports, priorities, conditionals, and module-system merge operators; a generic AST merger would need to reproduce Nix module semantics and would make output behavior difficult to reason about.

Instead, the assembler places selected intentionally authored Nix text fragments into known syntactic locations in a complete flake template. Nix remains responsible for evaluating Nix.

## 4. Source layout

A representative workspace layout:

```text
builderkit/
├── crates/
│   └── assemble/
│       ├── src/
│       │   ├── main.rs
│       │   ├── manifest.rs
│       │   ├── render.rs
│       │   ├── validate.rs
│       │   └── release.rs
│       └── templates/
│           ├── wsl-flake.nix.j2
│           └── darwin-flake.nix.j2
├── nix/
│   ├── common/
│   ├── fragments/
│   │   ├── git.nix
│   │   ├── agents.nix
│   │   ├── terminal.nix
│   │   └── packages.nix
│   └── platforms/
│       ├── wsl/
│       │   ├── system.nix
│       │   └── image.nix
│       └── darwin/
│           └── system.nix
├── release.toml
├── flake.lock.template-or-source
└── dist/
    ├── wsl/
    │   ├── flake.nix
    │   └── flake.lock
    └── darwin/
        ├── flake.nix
        └── flake.lock
```

The precise directory names may evolve; the separation of shared, WSL-only, Darwin-only, templates, typed release data, and generated outputs must remain explicit.

## 5. Typed release manifest

`release.toml` is a maintainer-controlled contract, not a second end-user configuration language.

It declares at least:

```toml
[release]
version = "0.1.0"
channel = "stable"
settings_schema_version = 1

[inputs]
nixpkgs_ref = "nixpkgs-unstable"
nixos_wsl_ref = "..."
nix_darwin_ref = "..."
home_manager_ref = "..."

[defaults]
hostname = "builderkit"
git_default_branch = "main"
profile = "standard"

[tools]
github_cli = true
claude_code = true
codex = true
vscode = true
```

The Rust manifest model uses enums where values are closed, validates version formats and supported release combinations, and rejects unknown or invalid values unless forward-compatible parsing is deliberately required.

The manifest must be sufficient to render deterministic platform release artifacts from a fixed source tree and lock input.

## 6. Rendering model

### 6.1 Full-file templates

The assembler renders a complete WSL flake and a complete Darwin flake from distinct full-file templates. The two files share product choices but are not required to be byte-identical.

The templates explicitly place source fragments in syntactically known scopes, such as a Home Manager user configuration body or a platform module body. A fragment’s expected scope is part of its maintained contract.

Example source fragment:

```nix
programs.git = {
  enable = true;
  userName = builderKit.owner.fullName;
  userEmail = builderKit.owner.email;
  extraConfig.init.defaultBranch = builderKit.git.defaultBranch;
};
```

The renderer may embed that fragment only in a scope where these statements are valid. The fragment does not independently import modules or dynamically discover its placement.

### 6.2 Output structure

Each generated flake contains, in a stable readable order:

1. Builder Kit release and settings-schema metadata.
2. The student-owned settings block.
3. Pinned input declarations.
4. Local variables and shared configuration definitions.
5. Platform-specific configuration implementation.
6. Managed implementation markers and integrity metadata.
7. Comments explaining only the configuration concepts a student may reasonably edit.

Generated output must include markers exactly once:

```text
BEGIN BUILDER-KIT USER SETTINGS
END BUILDER-KIT USER SETTINGS
BEGIN BUILDER-KIT MANAGED IMPLEMENTATION
END BUILDER-KIT MANAGED IMPLEMENTATION
```

The managed implementation hash is computed over a canonical defined range and recorded in release metadata and/or a stable flake comment. The updater uses it to detect manual changes outside student settings.

## 7. Platform artifact requirements

### 7.1 WSL artifact

The WSL generated flake must:

- define the NixOS configuration target used to build Builder Kit’s image;
- import NixOS-WSL and Home Manager NixOS modules;
- enable WSL integration;
- configure a bootstrap/recovery path and staged student-user migration model;
- set `wsl.tarball.configPath = ./.;` so the completed generated flake and lock file are copied into `/etc/nixos` in the image;
- include the source-built `kit-agent` in the image closure;
- define only release-owned WSL metadata and first-run behavior;
- use the release-pinned inputs represented by its adjacent lock file.

The resulting `dist/wsl` directory is the source used by release CI to build the `.wsl` image.

### 7.2 Darwin artifact

The Darwin generated flake must:

- define the stable nix-darwin configuration target used by the macOS CLI;
- import nix-darwin and Home Manager modules;
- configure Home Manager for the existing macOS user through the Builder Kit settings block;
- contain no NixOS-WSL modules or WSL-specific options;
- use the release-pinned inputs represented by its adjacent lock file;
- remain complete and usable at `~/builderkit/flake.nix` with no internal source tree required.

## 8. Lock file policy and release classes

Every generated flake ships with an adjacent `flake.lock`. The lock file is required release output, not optional metadata.

### 8.1 Dependency releases

A dependency release modifies the lock graph while leaving the generated `flake.nix` content unchanged. The release pipeline advances a selected upstream input cadence, validates the locked graph, and promotes only tested revisions.

Dependency release requirements:

- generated flake source hash matches the previous release;
- only `flake.lock` changes for each platform payload;
- WSL image is rebuilt and tested from the new WSL lock graph;
- release metadata declares the update class as `dependency`.

### 8.2 Feature releases

A feature release changes generated `flake.nix` implementation and may also change the lock graph.

Feature release requirements:

- increment or retain settings schema according to whether the student settings shape changed;
- provide explicit migrations for supported schema transitions;
- calculate and publish new managed-region integrity metadata;
- validate both platform outputs;
- release metadata declares the update class as `feature`.

## 9. Validation pipeline

The assembler must validate the artifact, not only the modular maintainer inputs.

For every render:

1. Validate `release.toml` into typed Rust data.
2. Load all selected Nix fragments and reject missing or invalid selections.
3. Render the full WSL and Darwin flakes.
4. Parse each rendered flake with `rnix-parser` and fail on parse errors.
5. Verify markers occur exactly once and in correct order.
6. Assert required platform structure through parser-aware checks and/or targeted Nix evaluation:
    - WSL output declares the expected NixOS target;
    - WSL output includes `wsl.tarball.configPath = ./.;`;
    - WSL output has NixOS-WSL and Home Manager integration;
    - Darwin output declares the expected Darwin target;
    - Darwin output has nix-darwin and Home Manager integration;
    - Darwin output contains no WSL-only configuration.
7. Format output consistently using the project-approved Nix formatter, then re-parse it.
8. Write generated files atomically.
9. Run `nix flake check` and target-specific evaluation where the release build environment supports it.

The release CI additionally builds the WSL image from `dist/wsl`, verifies that `/etc/nixos/flake.nix` and `/etc/nixos/flake.lock` are present in the result, and runs platform-qualified checks for macOS targets.

## 10. Settings parsing and migrations

The assembler and future updater share a small settings-management library.

Its responsibilities are:

- parse generated Nix with `rnix-parser`;
- locate the precisely delimited Builder Kit settings attribute set;
- extract it without rewriting unrelated comments or formatting;
- validate the declared schema version;
- migrate known schema versions in a deliberate, testable transformation;
- insert settings into a newly rendered feature-release flake;
- preserve all content outside the intended replacement span;
- reject malformed markers, duplicate markers, parser errors, and unexpected structure.

The library must not claim to interpret arbitrary Nix semantics. It works only with Builder Kit’s known generated-file layout and settings schema.

## 11. CLI contract

The assembler has a narrow maintainer interface, for example:

```text
kit-assembler release --manifest release.toml --out dist/
kit-assembler check --manifest release.toml --out dist/
```

`release` renders output and writes the artifact set. `check` validates existing or freshly rendered output and is suitable for CI.

It does not provision student machines, upload releases, create users, invoke WSL, or act as the student-facing Builder Kit CLI.

## 12. Acceptance criteria

1. Maintainers can evolve shared, WSL-only, and Darwin-only configuration in separated source files.
2. The assembler produces exactly two complete, readable, platform-specific generated flakes with adjacent lock files.
3. A student can inspect and edit a single flake file without requiring the maintainer source tree.
4. WSL output reliably copies its flake and lock file into `/etc/nixos` when built into an image.
5. Render failures, Nix syntax errors, missing markers, wrong platform imports, and invalid manifest data fail CI before publication.
6. The generated artifact itself is evaluated and tested, including WSL image construction.
7. Lock-only releases do not modify generated flake text; feature releases preserve or explicitly migrate the user settings block.
8. No part of the assembler attempts general semantic Nix module merging or reimplements Nix evaluation.
