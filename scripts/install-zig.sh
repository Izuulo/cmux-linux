#!/usr/bin/env bash
# Installs the exact Zig version Ghostty needs into .tools/zig (gitignored).
set -euo pipefail

VERSION=0.16.0
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    ARCH=x86_64-linux
    SHA=70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00
    ;;
  Darwin-arm64)
    ARCH=aarch64-macos
    SHA=b23d70deaa879b5c2d486ed3316f7eaa53e84acf6fc9cc747de152450d401489
    ;;
  Darwin-x86_64)
    ARCH=x86_64-macos
    SHA=0387557ed1877bc6a2e1802c8391953baddba76081876301c522f52977b52ba7
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
echo "$SHA  $TMP/zig.tar.xz" | shasum -a 256 -c -
tar -xJf "$TMP/zig.tar.xz" -C "$TMP"
mkdir -p "$ROOT/.tools"
rm -rf "$DEST"
mv "$TMP/zig-$ARCH-$VERSION" "$DEST"
"$DEST/zig" version
