#!/usr/bin/env bash
# Builds the checked-out tree (release, with the browser) and installs it for this user only:
#   ~/.local/lib/cmux-linux/   app, `cmux` CLI, CEF
#   ~/.local/bin/              cmux-linux, cmux (symlinks)
#   ~/.local/share/applications, ~/.local/share/icons   launcher entry and icon
# No root, so no sudo password. Re-run after `git pull` to update; a running app keeps working
# (the new files go to a fresh directory that replaces the old one in one rename).
# `scripts/install-local.sh --uninstall` removes it again.
set -euo pipefail
cd "$(dirname "$0")/.."

prefix="${CMUX_LINUX_PREFIX:-$HOME/.local}"
lib="$prefix/lib/cmux-linux"
bin="$prefix/bin"
apps="$prefix/share/applications"
icons="$prefix/share/icons/hicolor/512x512/apps"

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -rf "$lib"
  rm -f "$bin/cmux-linux" "$bin/cmux" "$apps/cmux-linux.desktop" "$icons/cmux-linux.png"
  echo "cmux-linux removed from $prefix"
  exit 0
fi

[[ -x .tools/zig/zig ]] || scripts/install-zig.sh
export PATH="$PWD/.tools/zig:$PATH"
# The browser feature fetches CEF into .tools/cef (.cargo/config.toml) on its first build.
cargo build --release --locked -p cmux-app -p cmux-ipc --features cmux-app/browser

stage="$(mktemp -d "$prefix/lib/.cmux-linux-new.XXXXXX" 2>/dev/null || { mkdir -p "$prefix/lib"; mktemp -d "$prefix/lib/.cmux-linux-new.XXXXXX"; })"
trap 'rm -rf "$stage"' EXIT
r=target/release
install -m755 "$r/cmux-app" "$stage/cmux-linux"
install -m755 "$r/cmux" "$stage/cmux"
# CEF next to the binary (its rpath is $ORIGIN), as the Arch package does (packaging/arch/PKGBUILD).
install -m755 "$r/libcef.so" "$stage/libcef.so"
strip --strip-unneeded "$stage/libcef.so"
# Not CEF's libvulkan.so.1: with the $ORIGIN rpath gpui would load it over the system loader.
install -m755 "$r/libvk_swiftshader.so" "$stage/"
install -m644 "$r"/*.pak "$r/icudtl.dat" "$r/v8_context_snapshot.bin" "$r/vk_swiftshader_icd.json" "$stage/"
cp -r "$r/locales" "$stage/"
install -m644 NOTICE LICENSE "$stage/"
cp .tools/cef/*/cef_linux_x86_64/CREDITS.html "$stage/CREDITS.html" 2>/dev/null || true

# Swap in: the old directory is renamed away first, so a running app keeps its open files.
old=""
if [[ -e "$lib" ]]; then old="$lib.old.$$"; mv "$lib" "$old"; fi
mv "$stage" "$lib"
trap - EXIT
[[ -n "$old" ]] && rm -rf "$old"

mkdir -p "$bin" "$apps" "$icons"
ln -sfn "$lib/cmux-linux" "$bin/cmux-linux"
ln -sfn "$lib/cmux" "$bin/cmux"
# Absolute Exec: a launcher's PATH may not include ~/.local/bin.
sed "s|^Exec=cmux-linux|Exec=$lib/cmux-linux|" packaging/cmux-linux.desktop > "$apps/cmux-linux.desktop"
install -m644 packaging/cmux-linux.png "$icons/cmux-linux.png"
command -v update-desktop-database >/dev/null && update-desktop-database -q "$apps" || true

echo "Installed $(git describe --always --dirty) to $lib"
if pacman -Q cmux-linux-git >/dev/null 2>&1; then
  echo "Note: the cmux-linux-git package is also installed (/usr). Remove it once with:"
  echo "  sudo pacman -R cmux-linux-git"
fi
