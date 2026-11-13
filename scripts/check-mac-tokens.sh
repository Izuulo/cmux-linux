#!/usr/bin/env bash
# M14 exit 5: surfaces built on the mac kit set no colour, radius, shadow or timing of their own.
set -euo pipefail
cd "$(dirname "$0")/.."
files=(
  crates/cmux-app/src/window_view/menu.rs
  crates/cmux-app/src/window_view/panel.rs
  crates/cmux-app/src/window_view/palette.rs
  crates/cmux-app/src/window_view/titlebar.rs
  crates/cmux-app/src/settings.rs
  crates/cmux-app/src/settings
  crates/cmux-app/src/browser.rs
)
# A line that draws content rather than chrome (a theme preview swatch) may carry `// mac-ok`.
if grep -rnE 'BoxShadow|Rgba \{|\.rounded\(px\([0-9]|Duration::from_millis' "${files[@]}" | grep -v 'mac-ok'; then
  echo "check-mac-tokens: the lines above set their own look; use crate::mac tokens" >&2
  exit 1
fi
echo "check-mac-tokens: ok"
