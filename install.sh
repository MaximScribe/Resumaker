#!/bin/sh
# Installs Resumaker on Linux and macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/MaximScribe/Resumaker/main/install.sh | sh
#
# Downloads the latest release, verifies its SHA-256 checksum, puts `resumaker` and
# `resumake` in ~/.local/bin and, if needed, adds that folder to PATH in your shell's
# startup file. Run it again to update.
#
# Optional environment variables:
#   RESUMAKER_VERSION         a release tag such as v1.0.1 (default: the latest release)
#   RESUMAKER_INSTALL_DIR     where the binaries go
#   RESUMAKER_NO_MODIFY_PATH  set to 1 to leave your shell startup files alone

set -eu

repo="MaximScribe/Resumaker"

err() {
    echo "error: $*" >&2
    exit 1
}

# Download $1 to $2 with whichever tool is available.
fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$2" "$1"
    else
        err "curl or wget is required"
    fi
}

case "$(uname -s)" in
    Linux) os="unknown-linux-gnu" ;;
    Darwin) os="apple-darwin" ;;
    *) err "unsupported OS $(uname -s); on Windows use install.ps1" ;;
esac
case "$(uname -m)" in
    x86_64 | amd64) arch="x86_64" ;;
    arm64 | aarch64) arch="aarch64" ;;
    *) err "unsupported CPU $(uname -m)" ;;
esac
target="$arch-$os"
if [ "$target" = "aarch64-unknown-linux-gnu" ]; then
    err "no prebuilt binary for ARM Linux yet; install from source with: cargo install --git https://github.com/$repo resumaker-cli"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

version="${RESUMAKER_VERSION:-}"
if [ -z "$version" ]; then
    fetch "https://api.github.com/repos/$repo/releases/latest" "$tmp/latest.json"
    version="$(sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' "$tmp/latest.json" | head -n 1)"
    [ -n "$version" ] || err "could not determine the latest release"
fi
dir="${RESUMAKER_INSTALL_DIR:-$HOME/.local/bin}"

name="resumaker-$version-$target"
base="https://github.com/$repo/releases/download/$version"

echo "Downloading Resumaker $version for $target ..."
fetch "$base/$name.tar.gz" "$tmp/$name.tar.gz"
fetch "$base/$name.tar.gz.sha256" "$tmp/$name.tar.gz.sha256"

if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$tmp/$name.tar.gz" | cut -d ' ' -f 1)"
elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$tmp/$name.tar.gz" | cut -d ' ' -f 1)"
else
    err "sha256sum or shasum is required to verify the download"
fi
expected="$(cut -d ' ' -f 1 "$tmp/$name.tar.gz.sha256")"
[ "$actual" = "$expected" ] || err "checksum mismatch for $name.tar.gz (expected $expected, got $actual); nothing was installed"

tar xzf "$tmp/$name.tar.gz" -C "$tmp"
mkdir -p "$dir"
for bin in resumaker resumake; do
    cp "$tmp/$name/$bin" "$dir/$bin"
    chmod 755 "$dir/$bin"
done
echo "Installed resumaker and resumake to $dir"

case ":$PATH:" in
    *":$dir:"*) on_path=1 ;;
    *) on_path=0 ;;
esac

if [ "$on_path" = 0 ] && [ "${RESUMAKER_NO_MODIFY_PATH:-0}" != 1 ]; then
    case "${SHELL:-}" in
        */zsh) rc="$HOME/.zshrc" ;;
        */bash) if [ "$(uname -s)" = Darwin ]; then rc="$HOME/.bash_profile"; else rc="$HOME/.bashrc"; fi ;;
        *) rc="$HOME/.profile" ;;
    esac
    line="export PATH=\"$dir:\$PATH\""
    if ! grep -qsF "$line" "$rc"; then
        printf '\n# Added by the Resumaker installer\n%s\n' "$line" >>"$rc"
        echo "Added $dir to PATH in $rc"
    fi
    echo ""
    echo "Open a new terminal, or run:  $line"
fi

echo ""
echo "Done. Run 'resumaker' to set up, then 'resumake' to make a resume."
