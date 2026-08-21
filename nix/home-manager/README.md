# Home Manager fragments

These files are shared Home Manager module fragments. The assembler reads every
top-level `*.nix` file in this directory, sorts them by path for deterministic
output, and places them in the generated Home Manager user's `imports` list.

Each fragment is a complete Home Manager module expression. Declare only the
module arguments it needs, for example `{ pkgs, ... }: { ... }`; fragments that
need no bindings may be plain attrsets.

`base.nix` also contains a stable insertion marker inside `programs.zsh` so
platform-specific shell initialization can be embedded in the shared zsh module.
