# Darwin fragments

These files are maintainer-authored Nix fragments for the generated macOS flake.
They are intentionally small snippets, not standalone flakes. The assembler embeds
them into known syntactic locations in the full-file template.

Expected template bindings:

- `inputs`: the generated flake input set
- `builderKit`: the student-owned settings attrset
- `system`: the current release-qualified Nix system from `forEachSystem`
- `pkgs`, `lib`, `config`: normal nix-darwin/Home Manager module arguments where applicable

Fragment scopes:

- `flake-inputs.nix`: entries inside the top-level `inputs = { ... };` attrset
- `darwin-configuration.nix`: entries inside the top-level `outputs` attrset
- `darwin-module.nix`: statements inside a nix-darwin module body; embeds shared Home Manager fragments from `nix/home-manager/` and Darwin-only Home Manager fragments from `nix/platforms/darwin/home-manager/`
- `home-manager/zsh-init-content.nix`: Darwin-only statements inside the shared Home Manager `programs.zsh` body
