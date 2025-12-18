#!/usr/bin/env bash
set -euo pipefail

./node_modules/.bin/tailwindcss \
  -c tailwind.config.cjs \
  -i assets/css/tailwind.input.css \
  -o assets/css/tailwind.generated.css \
  --minify

