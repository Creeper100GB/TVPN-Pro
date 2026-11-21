#!/usr/bin/env bash
# TVPN Pro installer for macOS.
#
# There is no public macOS release yet. This script finds a .dmg or .zip asset
# in the GitHub releases, verifies SHA256SUMS.txt when present and copies the
# app to ~/Applications. Signing and notarization are still missing.

set -euo pipefail

REPOSITORY="Creeper100GB/TVPN-Pro"
API="https://api.github.com/repos/${REPOSITORY}/releases?per_page=10"
TARGET="${HOME}/Applications"

if [ "$(uname -s)" != "Darwin" ]; then
    echo "Dieses Skript ist für macOS. Unter Windows: install.ps1" >&2
    exit 1
fi

json="$(curl -fsSL \
    -H 'User-Agent: TVPN-Pro-Installer' \
    -H 'Accept: application/vnd.github+json' \
    "$API")" || {
    echo "GitHub-Abfrage fehlgeschlagen." >&2
    exit 1
}

urls="$(printf '%s' "$json" \
    | grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*"' \
    | sed -E 's/.*"(https:[^"]+)".*/\1/' || true)"

download="$(printf '%s\n' "$urls" | grep -Ei '\.(dmg|zip)$' | head -n 1 || true)"
sums_url="$(printf '%s\n' "$urls" | grep -Ei '/SHA256SUMS\.txt$' | head -n 1 || true)"

if [ -z "$download" ]; then
    echo "Für macOS gibt es noch keinen Release." >&2
    echo "Der Build-Check läuft in der CI. Signierung und Notarisierung fehlen noch." >&2
    echo "https://github.com/${REPOSITORY}/actions/workflows/macos.yml" >&2
    exit 1
fi

echo "==> Lade $(basename "$download")"
tmp="$(mktemp -d)"
mountpoint=""

cleanup() {
    if [ -n "$mountpoint" ] && mount | grep -Fq "on ${mountpoint} "; then
        hdiutil detach "$mountpoint" -quiet || true
    fi
    rm -rf "$tmp"
}
trap cleanup EXIT

name="$(basename "$download" | sed 's/%20/ /g')"
file="${tmp}/${name}"
curl -fsSL "$download" -o "$file"

if [ -n "$sums_url" ]; then
    echo "==> Kontrolliere SHA-256"
    curl -fsSL "$sums_url" -o "$tmp/SHA256SUMS.txt"
    normalized="$(printf '%s' "$name" | tr -d ' .')"
    expected="$(awk -v want="$normalized" '{
        hash=$1;
        line=$0;
        sub(/^[^ ]+[ ]+/, "", line);
        gsub(/[ .]/, "", line);
        if (line == want) { print hash; exit }
    }' "$tmp/SHA256SUMS.txt")"
    if [ -z "$expected" ]; then
        echo "Keine SHA-256-Zeile zu ${name} gefunden." >&2
        exit 1
    fi
    actual="$(shasum -a 256 "$file" | awk '{print $1}')"
    if [ "$actual" != "$expected" ]; then
        echo "SHA-256 stimmt nicht. Erwartet: ${expected}. Erhalten: ${actual}." >&2
        exit 1
    fi
else
    echo "Warnung: keine SHA256SUMS.txt im Release, SHA-256-Kontrolle entfällt." >&2
fi

case "$name" in
    *.dmg)
        echo "==> Binde DMG ein"
        mountpoint="${tmp}/mnt"
        mkdir -p "$mountpoint"
        hdiutil attach "$file" -mountpoint "$mountpoint" -nobrowse -quiet

        app=""
        for candidate in "$mountpoint"/*.app; do
            if [ -d "$candidate" ]; then
                app="$candidate"
                break
            fi
        done
        if [ -z "$app" ]; then
            echo "Keine .app im DMG gefunden." >&2
            exit 1
        fi
        ;;
    *.zip)
        echo "==> Entpacke ZIP"
        mkdir -p "$tmp/unpack"
        ditto -x -k "$file" "$tmp/unpack"

        app=""
        for candidate in "$tmp/unpack"/*.app "$tmp/unpack"/*/*.app; do
            if [ -d "$candidate" ]; then
                app="$candidate"
                break
            fi
        done
        if [ -z "$app" ]; then
            echo "Keine .app im ZIP gefunden." >&2
            exit 1
        fi
        ;;
    *)
        echo "Unbekanntes Format: ${name}" >&2
        exit 1
        ;;
esac

mkdir -p "$TARGET"
echo "==> Kopiere $(basename "$app") nach ${TARGET}"
rm -rf "${TARGET}/$(basename "$app")"
cp -R "$app" "${TARGET}/"

if [ -n "$mountpoint" ]; then
    hdiutil detach "$mountpoint" -quiet
    mountpoint=""
fi

cat <<'NOTE'
Fertig. Der Build ist unsigniert und nicht notarisiert. macOS blockiert ihn
beim ersten Start. Erlaube ihn unter Systemeinstellungen > Datenschutz &
Sicherheit, oder starte ihn einmal per Rechtsklick > Öffnen.
NOTE
