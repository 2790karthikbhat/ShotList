#!/usr/bin/env bash
# Install the latest ShotList release on macOS or Linux. Non-interactive.
#
#   curl -fsSL https://raw.githubusercontent.com/2790karthikbhat/ShotList/main/scripts/install.sh | bash
#
# macOS: copies ShotList.app into /Applications (or ~/Applications if that isn't writable)
#        and clears the quarantine flag, since the builds are not code-signed yet.
# Linux: installs the AppImage as ~/.local/bin/shotlist.
# Set SHOTLIST_VERSION=v0.1.0 to pin a release instead of taking the latest.
set -eu

REPO="2790karthikbhat/ShotList"
VERSION="${SHOTLIST_VERSION:-latest}"

die() { echo "error: $*" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || die "curl is required"

if [ "$VERSION" = "latest" ]; then
  api="https://api.github.com/repos/$REPO/releases/latest"
else
  api="https://api.github.com/repos/$REPO/releases/tags/$VERSION"
fi

release_json="$(curl -fsSL -H 'Accept: application/vnd.github+json' "$api")" \
  || die "no published release found at $api. Build from source instead: see README.md, Build from source"

asset_url() {
  printf '%s\n' "$release_json" \
    | grep -o '"browser_download_url": *"[^"]*"' \
    | sed 's/.*"\(https[^"]*\)"/\1/' \
    | grep -E "$1" | head -n1
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

case "$(uname -s)" in
  Darwin)
    url="$(asset_url '_universal\.dmg$')"
    [ -n "$url" ] || die "this release has no macOS .dmg"
    echo "Downloading $url"
    curl -fL --progress-bar -o "$tmp/ShotList.dmg" "$url"
    mnt="$tmp/mnt"
    mkdir "$mnt"
    hdiutil attach -nobrowse -quiet -mountpoint "$mnt" "$tmp/ShotList.dmg"
    dest="/Applications"
    [ -w "$dest" ] || { dest="$HOME/Applications"; mkdir -p "$dest"; }
    rm -rf "$dest/ShotList.app"
    cp -R "$mnt/ShotList.app" "$dest/"
    hdiutil detach -quiet "$mnt"
    xattr -dr com.apple.quarantine "$dest/ShotList.app" 2>/dev/null || true
    echo "Installed $dest/ShotList.app"
    echo "Launch: open \"$dest/ShotList.app\""
    ;;
  Linux)
    [ "$(uname -m)" = "x86_64" ] || die "only x86_64 Linux builds are published; build from source on $(uname -m)"
    url="$(asset_url '_amd64\.AppImage$')"
    [ -n "$url" ] || die "this release has no Linux AppImage"
    echo "Downloading $url"
    mkdir -p "$HOME/.local/bin"
    curl -fL --progress-bar -o "$tmp/shotlist" "$url"
    chmod +x "$tmp/shotlist"
    mv "$tmp/shotlist" "$HOME/.local/bin/shotlist"
    echo "Installed $HOME/.local/bin/shotlist"
    case ":$PATH:" in *":$HOME/.local/bin:"*) ;; *) echo "Add ~/.local/bin to PATH to run it as 'shotlist'.";; esac
    echo "Launch: shotlist"
    ;;
  *)
    die "unsupported OS. On Windows, download the .msi from https://github.com/$REPO/releases/latest"
    ;;
esac
