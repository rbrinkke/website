#!/usr/bin/env bash
set -euo pipefail

url="${1:-http://127.0.0.1:3000/login}"

open_chrome_incognito() {
  local target="$1"

  if [[ "${OSTYPE:-}" == "darwin"* ]]; then
    if open -Ra "Google Chrome" >/dev/null 2>&1; then
      open -na "Google Chrome" --args --incognito "$target" >/dev/null 2>&1 || true
      return 0
    fi
    echo "Could not find Google Chrome (macOS). Open this manually in an incognito window:"
    echo "  $target"
    return 0
  fi

  if command -v google-chrome-stable >/dev/null 2>&1; then
    nohup google-chrome-stable --incognito --new-window --no-first-run --no-default-browser-check "$target" >/dev/null 2>&1 &
    return 0
  fi

  if command -v google-chrome >/dev/null 2>&1; then
    nohup google-chrome --incognito --new-window --no-first-run --no-default-browser-check "$target" >/dev/null 2>&1 &
    return 0
  fi

  if command -v chromium >/dev/null 2>&1; then
    nohup chromium --incognito --new-window --no-first-run --no-default-browser-check "$target" >/dev/null 2>&1 &
    return 0
  fi

  if command -v chromium-browser >/dev/null 2>&1; then
    nohup chromium-browser --incognito --new-window --no-first-run --no-default-browser-check "$target" >/dev/null 2>&1 &
    return 0
  fi

  echo "Could not find Chrome/Chromium binary. Open this manually in an incognito window:"
  echo "  $target"
}

./scripts/cargo-fast.sh build --bin website
./scripts/build-css.sh

./scripts/cargo-fast.sh run &
server_pid="$!"

cleanup() {
  if kill -0 "$server_pid" >/dev/null 2>&1; then
    kill "$server_pid" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

sleep 1
open_chrome_incognito "$url"

wait "$server_pid"
