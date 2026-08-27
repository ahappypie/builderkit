#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: darwin-package-snapshot.sh <flake-dir> <output-dir>" >&2
  exit 2
fi

FLAKE_DIR=$1
OUTPUT_DIR=$2

flake_ref() {
  local dir=$1
  printf 'path:%s' "$(cd "$dir" && pwd)"
}

first_darwin_system() {
  local dir=$1
  nix eval --json "$(flake_ref "$dir")#darwinConfigurations" --apply 'builtins.attrNames' \
    | jq -r '.[0]'
}

package_names() {
  local dir=$1
  local attr=$2
  local system
  system=$(first_darwin_system "$dir")

  nix eval --json "$(flake_ref "$dir")#darwinConfigurations.$system.config.$attr" \
    --apply 'xs: map (x: x.name or null) xs' \
    | jq -r '.[]?' \
    | sort -u
}

home_manager_package_names() {
  local dir=$1
  local system user
  system=$(first_darwin_system "$dir")
  user=$(nix eval --json "$(flake_ref "$dir")#darwinConfigurations.$system.config.home-manager.users" --apply 'builtins.attrNames' \
    | jq -r '.[0]')

  nix eval --json "$(flake_ref "$dir")#darwinConfigurations.$system.config.home-manager.users.$user.home.packages" \
    --apply 'xs: map (x: x.name or null) xs' \
    | jq -r '.[]?' \
    | sort -u
}

flake_defined_package_names() {
  local dir=$1
  local system
  system=$(first_darwin_system "$dir")

  nix eval --json "$(flake_ref "$dir")#packages.$system" --apply 'builtins.attrNames' 2>/dev/null \
    | jq -r '.[]?' \
    | sort -u || true
}

lock_input_refs() {
  local lock=$1

  jq -r '
    def ref($l):
      if $l == null then "unlocked"
      elif $l.type == "github" then "github:\($l.owner)/\($l.repo)/\($l.rev[0:12])"
      elif $l.path != null then "path:\($l.path)"
      else ($l | tojson)
      end;

    .nodes
    | to_entries[]
    | select(.key != "root")
    | "\(.key)\t\(ref(.value.locked))"
  ' "$lock" | sort -u
}

mkdir -p "$OUTPUT_DIR"

SYSTEM=$(first_darwin_system "$FLAKE_DIR")
printf '%s\n' "$SYSTEM" > "$OUTPUT_DIR/system.txt"
lock_input_refs "$FLAKE_DIR/flake.lock" > "$OUTPUT_DIR/flake-inputs.txt"
package_names "$FLAKE_DIR" environment.systemPackages > "$OUTPUT_DIR/darwin-system-packages.txt"
home_manager_package_names "$FLAKE_DIR" > "$OUTPUT_DIR/home-manager-packages.txt"
flake_defined_package_names "$FLAKE_DIR" > "$OUTPUT_DIR/flake-defined-packages.txt"
