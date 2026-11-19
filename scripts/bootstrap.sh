#!/usr/bin/env bash
# Set up a fresh clone of ShotList for development. Non-interactive and safe to re-run.
#
#   scripts/bootstrap.sh          check prerequisites, install deps, build the frontend, compile Rust
#   scripts/bootstrap.sh --check  only check prerequisites (exit 1 and print the fix if any are missing)
#
# Exit codes: 0 ready · 1 a prerequisite is missing · 2 a build step failed.
set -u

MIN_RUST="1.88.0"
MIN_NODE="18.0.0"
CHECK_ONLY=0
[ "${1:-}" = "--check" ] && CHECK_ONLY=1

cd "$(dirname "$0")/.." || exit 2

missing=0
ok()   { printf '  ok    %s\n' "$1"; }
fail() { printf '  MISS  %s\n        fix: %s\n' "$1" "$2"; missing=1; }

version_ge() {
  [ "$(printf '%s\n%s\n' "$2" "$1" | sort -t. -k1,1n -k2,2n -k3,3n | head -n1)" = "$2" ]
}

echo "Checking prerequisites"

if command -v rustc >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
  rv="$(rustc --version | awk '{print $2}')"
  if version_ge "$rv" "$MIN_RUST"; then ok "rustc $rv"; else fail "rustc $rv (need >= $MIN_RUST)" "rustup update stable"; fi
else
  fail "rustc / cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
fi

if command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
  nv="$(node --version | sed 's/^v//')"
  if version_ge "$nv" "$MIN_NODE"; then ok "node $nv"; else fail "node $nv (need >= $MIN_NODE)" "install Node.js LTS from https://nodejs.org"; fi
else
  fail "node / npm" "install Node.js LTS from https://nodejs.org"
fi

case "$(uname -s)" in
  Darwin)
    if xcode-select -p >/dev/null 2>&1; then ok "Xcode Command Line Tools"; else fail "Xcode Command Line Tools" "xcode-select --install"; fi
    ;;
  Linux)
    linux_fix="sudo apt-get install -y build-essential pkg-config libssl-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf"
    if command -v pkg-config >/dev/null 2>&1; then
      for lib in webkit2gtk-4.1 openssl librsvg-2.0; do
        if pkg-config --exists "$lib"; then ok "$lib"; else fail "$lib" "$linux_fix"; fi
      done
    else
      fail "pkg-config" "$linux_fix"
    fi
    ;;
  *)
    echo "  note  unrecognised OS; on Windows install the WebView2 runtime and the MSVC build tools (see https://v2.tauri.app/start/prerequisites/)"
    ;;
esac

if [ "$missing" -ne 0 ]; then
  echo "Prerequisites missing. Run the fix commands above, then re-run scripts/bootstrap.sh."
  exit 1
fi
[ "$CHECK_ONLY" -eq 1 ] && { echo "All prerequisites present."; exit 0; }

step() {
  echo "==> $1"
  shift
  "$@" || { echo "FAILED: $*"; exit 2; }
}

step "Installing frontend dependencies" npm --prefix frontend ci --no-audit --no-fund
step "Building frontend (Tauri embeds frontend/dist at compile time)" npm --prefix frontend run build
step "Compiling Rust (first run downloads and builds ~500 crates; allow several minutes)" cargo build

if [ -d .git ] && [ "$(git config core.hooksPath 2>/dev/null)" != ".githooks" ]; then
  git config core.hooksPath .githooks && echo "==> Enabled the fmt + clippy pre-commit hook"
fi

echo
echo "Ready. Next:"
echo "  npm run dev     launch the app with hot reload"
echo "  npm run check   run every check CI runs"
