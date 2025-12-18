#!/usr/bin/env bash
set -euo pipefail

cmd="${1:-}"
shift || true

if [[ -z "${cmd}" ]]; then
  cmd="help"
fi

kill_port_3000() {
  if lsof -i :3000 -t >/dev/null 2>&1; then
    echo "⚠️  Killing existing process on port 3000..."
    kill -9 $(lsof -i :3000 -t)
  fi
}

list_bins() {
  if ! command -v python3 >/dev/null 2>&1; then
    echo "website"
    return 0
  fi

  cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c '
import json, sys
try:
    data = json.load(sys.stdin)
except Exception:
    print("website")
    sys.exit(0)
pkgs = data.get("packages") or []
pkg = pkgs[0] if pkgs else {}
targets = pkg.get("targets") or []
bins = [t.get("name") for t in targets if "bin" in (t.get("kind") or [])]
for b in bins:
    if b:
        print(b)
'
}

has_bin_flag() {
  for a in "$@"; do
    if [[ "$a" == "--bin" || "$a" == --bin=* ]]; then
      return 0
    fi
  done
  return 1
}

validate_bin_flag() {
  local prev=""
  for a in "$@"; do
    if [[ "$prev" == "--bin" ]]; then
      if [[ -z "${a:-}" || "$a" == -* || "$a" == "--" ]]; then
        echo "error: \"--bin\" takes one argument." >&2
        echo "Available binaries:" >&2
        list_bins | sed 's/^/    /' >&2 || true
        exit 2
      fi
      return 0
    fi
    if [[ "$a" == --bin=* ]]; then
      return 0
    fi
    prev="$a"
  done

  if [[ "$prev" == "--bin" ]]; then
    echo "error: \"--bin\" takes one argument." >&2
    echo "Available binaries:" >&2
    list_bins | sed 's/^/    /' >&2 || true
    exit 2
  fi
}

# Convenience: `./scripts/cargo-fast.sh dev` == `cargo run`
if [[ "$cmd" == "dev" ]]; then
  cmd="run"
fi

# If we have multiple binaries, default `run` to the main server binary.
if [[ "$cmd" == "run" ]]; then
  if [[ "${1:-}" != "" && "${1:-}" != -* && "${1:-}" != "--" ]] && ! has_bin_flag "$@"; then
    # Shorthand: `./scripts/cargo-fast.sh run backfill_activity_geo`
    bin="$1"
    shift
    set -- --bin "$bin" "$@"
  fi

  if has_bin_flag "$@"; then
    validate_bin_flag "$@"
  else
    set -- --bin website "$@"
  fi

  # Only kill the dev server when we're going to run it again.
  kill_port_3000
  set -- run "$@"
else
  set -- "$cmd" "$@"
fi

# Speedy Cargo wrapper:
# - Uses sccache if available.
# - Prefers mold, falls back to ld.lld if present.
# - Leaves env untouched if tools are missing (safe defaults).

export RUSTC_WRAPPER="${RUSTC_WRAPPER:-}"
export RUSTFLAGS="${RUSTFLAGS:-}"

if command -v sccache >/dev/null 2>&1; then
  export RUSTC_WRAPPER="sccache"
fi

if command -v mold >/dev/null 2>&1; then
  export RUSTFLAGS="$RUSTFLAGS -C link-arg=-fuse-ld=mold"
elif command -v ld.lld >/dev/null 2>&1; then
  export RUSTFLAGS="$RUSTFLAGS -C link-arg=-fuse-ld=lld"
fi

exec cargo "$@"
