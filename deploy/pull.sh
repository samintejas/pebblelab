#!/usr/bin/env bash
# Installs a release from GitHub on this server, by hand: downloads its pebblelab.tar.gz, checks it against the
# release's checksum and hands it to release.sh, which swaps it in, restarts and checks /health.
# usage: pull.sh [prod-<date>-<commit>]   (the latest release when no tag is given)
# Run it in your own terminal: it asks for your sudo password once, for the restart.
set -euo pipefail
repo=https://github.com/samintejas/pebblelab
app=${PEBBLELAB_APP_DIR:-$HOME/apps/pebblelab}
if [ $# -ge 1 ]; then
  [[ $1 =~ ^[A-Za-z0-9._-]+$ ]] || { echo "error: bad tag '$1'" >&2; exit 2; }
  url=$repo/releases/download/$1
else
  url=$repo/releases/latest/download
fi
work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
curl -fsSL -o "$work/pebblelab.tar.gz" "$url/pebblelab.tar.gz"
curl -fsSL -o "$work/pebblelab.tar.gz.sha256" "$url/pebblelab.tar.gz.sha256"
(cd "$work" && sha256sum -c --quiet pebblelab.tar.gz.sha256)
mkdir "$work/peek" && tar xzf "$work/pebblelab.tar.gz" -C "$work/peek" ./REVISION
rev=$(head -c 12 "$work/peek/REVISION")
echo "installing pebblelab $rev"
sudo -v
"$app/release.sh" "$work/pebblelab.tar.gz" "$rev"
