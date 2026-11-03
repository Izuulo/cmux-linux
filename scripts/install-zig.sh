#!/usr/bin/env bash
# Installs the exact Zig version Ghostty needs into .tools/zig (gitignored).
set -euo pipefail

VERSION=0.16.0
# shortcut: Linux x86_64 only, add macOS/aarch64 rows from ziglang.org/download/index.json in M6.
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    ARCH=x86_64-linux
    SHA=70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00
    ;;
  *)
    echo "unsupported platform $(uname -s)-$(uname -m): add its tarball and sha256 from https://ziglang.org/download/index.json" >&2
    exit 1
    ;;
esac

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/.tools/zig"

if [[ -x "$DEST/zig" && "$("$DEST/zig" version)" == "$VERSION" ]]; then
  echo "zig $VERSION already installed at .tools/zig"
  exit 0
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
curl -fsSL "https://ziglang.org/download/$VERSION/zig-$ARCH-$VERSION.tar.xz" -o "$TMP/zig.tar.xz"
echo "$SHA  $TMP/zig.tar.xz" | sha256sum -c -
tar -xJf "$TMP/zig.tar.xz" -C "$TMP"
mkdir -p "$ROOT/.tools"
rm -rf "$DEST"
mv "$TMP/zig-$ARCH-$VERSION" "$DEST"
"$DEST/zig" version
