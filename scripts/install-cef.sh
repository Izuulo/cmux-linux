#!/usr/bin/env bash
# Fetches CEF (the in-app browser engine, feature `browser`) into .tools/cef (gitignored).
# cef-dll-sys's build script downloads (once, ~330 MB) and checks the CEF version matching the
# pinned `cef` crate into $CEF_PATH, which .cargo/config.toml points at .tools/cef, so building
# the app once with the feature is the install.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export PATH="$ROOT/.tools/zig:$PATH"
cargo build --locked -p cmux-app --features browser
ls -d .tools/cef/*/cef_*
