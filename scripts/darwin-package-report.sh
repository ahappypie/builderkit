#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: darwin-package-report.sh <old-snapshot-dir> <new-snapshot-dir> <output-md>" >&2
  exit 2
fi

OLD_DIR=$1
NEW_DIR=$2
OUTPUT=$3

normalize_inputs() {
  local input=$1
  local output=$2
  cp "$input" "$output"
}

normalize_packages() {
  local input=$1
  local output=$2

  awk '
    /^[[:space:]]*$/ { next }
    {
      # Nix package names conventionally end in -<version>. Split at the last
      # hyphen whose following component starts with a digit. Unversioned
      # implementation artifacts are omitted from the summary table and remain
      # visible in the full raw diff.
      if (match($0, /-[0-9][^-]*$/)) {
        name = substr($0, 1, RSTART - 1)
        version = substr($0, RSTART + 1)
        print name "\t" version
      }
    }
  ' "$input" | sort -u > "$output"
}

write_comparison_table() {
  local title=$1
  local old_file=$2
  local new_file=$3
  local old_normalized=$4
  local new_normalized=$5
  local normalizer=$6
  local tmp

  tmp=$(mktemp -d)
  "$normalizer" "$old_file" "$old_normalized"
  "$normalizer" "$new_file" "$new_normalized"

  echo "## $title"
  echo

  awk -F '\t' '
    FNR == NR {
      old[$1] = $2
      names[$1] = 1
      next
    }
    {
      new[$1] = $2
      names[$1] = 1
    }
    END {
      changed = 0
      for (name in names) {
        old_value = (name in old) ? old[name] : "—"
        new_value = (name in new) ? new[name] : "—"
        if (old_value != new_value) {
          changed = 1
          print name "\t" old_value "\t" new_value
        }
      }
      if (!changed) {
        print "__NO_CHANGES__\t\t"
      }
    }
  ' "$old_normalized" "$new_normalized" | sort > "$tmp/changes.tsv"

  if grep -q '^__NO_CHANGES__' "$tmp/changes.tsv"; then
    echo "- No changes detected."
    echo
    rm -rf "$tmp"
    return
  fi

  echo "| Name | Previous | Updated |"
  echo "| --- | --- | --- |"
  while IFS=$'\t' read -r name old_value new_value; do
    echo "| \`$name\` | \`$old_value\` | \`$new_value\` |"
  done < "$tmp/changes.tsv"
  echo

  rm -rf "$tmp"
}

write_full_diff() {
  local title=$1
  local old_file=$2
  local new_file=$3

  echo "### $title"
  echo
  echo '```diff'
  diff -u "$old_file" "$new_file" || true
  echo '```'
  echo
}

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

{
  echo "# Darwin flake update summary"
  echo
  write_comparison_table "Flake input changes" "$OLD_DIR/flake-inputs.txt" "$NEW_DIR/flake-inputs.txt" "$TMP/old-inputs.tsv" "$TMP/new-inputs.tsv" normalize_inputs
  write_comparison_table "Darwin system package changes" "$OLD_DIR/darwin-system-packages.txt" "$NEW_DIR/darwin-system-packages.txt" "$TMP/old-system.tsv" "$TMP/new-system.tsv" normalize_packages
  write_comparison_table "Home Manager package changes" "$OLD_DIR/home-manager-packages.txt" "$NEW_DIR/home-manager-packages.txt" "$TMP/old-home.tsv" "$TMP/new-home.tsv" normalize_packages
  write_comparison_table "Flake-defined package changes" "$OLD_DIR/flake-defined-packages.txt" "$NEW_DIR/flake-defined-packages.txt" "$TMP/old-flake.tsv" "$TMP/new-flake.tsv" normalize_packages

  echo "## Full diffs"
  echo
  write_full_diff "Flake inputs" "$OLD_DIR/flake-inputs.txt" "$NEW_DIR/flake-inputs.txt"
  write_full_diff "Darwin system packages" "$OLD_DIR/darwin-system-packages.txt" "$NEW_DIR/darwin-system-packages.txt"
  write_full_diff "Home Manager packages" "$OLD_DIR/home-manager-packages.txt" "$NEW_DIR/home-manager-packages.txt"
  write_full_diff "Flake-defined packages" "$OLD_DIR/flake-defined-packages.txt" "$NEW_DIR/flake-defined-packages.txt"
} > "$OUTPUT"

cat "$OUTPUT"
