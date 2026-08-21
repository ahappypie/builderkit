# WSL fragments

These files are maintainer-authored Nix fragments for the generated WSL/NixOS
flake. They are snippets for known template scopes, not standalone flakes.

Expected template bindings:

- `inputs`: the generated flake input set
- `builderKit`: the student-owned settings attrset
- `system`: the current release-qualified Nix system from `forEachSystem`
- `pkgs`, `lib`, `config`: normal NixOS/Home Manager module arguments where applicable

Fragment scopes:

- `flake-inputs.nix`: entries inside the top-level `inputs = { ... };` attrset
- `wsl-configuration.nix`: a NixOS system expression inside `nixosConfigurations = forEachSystem (system: ...)`
- `wsl-module.nix`: a complete NixOS module embedded in the WSL system `modules` list
- `home-manager/zsh-init-content.nix`: WSL-only statements inside the shared Home Manager `programs.zsh` body
