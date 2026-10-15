#!/usr/bin/env bash
# Installs a build on the server, as the app's user. Two ways in:
# - from the deploy workflow, as the forced command of its key in ~/.ssh/authorized_keys: the build arrives on
#   stdin and the revision is the one word the workflow asks to run (SSH_ORIGINAL_COMMAND);
# - by hand: release.sh <bundle.tar.gz> <revision>
# Unpacks the build into ~/apps/pebblelab/releases/<revision>, swaps the binary and web app in, restarts the
# service, and puts the previous release back if it does not answer /health. The database migrates itself on start.
# This copy is the one installed at ~/apps/pebblelab/release.sh; a deploy never replaces it.
set -euo pipefail
umask 022
app=${PEBBLELAB_APP_DIR:-$HOME/apps/pebblelab}
health=${PEBBLELAB_HEALTH:-http://127.0.0.1:3000/health}

if [ -n "${SSH_ORIGINAL_COMMAND+x}" ]; then
  rev=$SSH_ORIGINAL_COMMAND
  [[ $rev =~ ^[0-9a-f]{7,40}$ ]] || { echo "error: expected a commit hash, got '$rev'" >&2; exit 2; }
  bundle=$(mktemp)
  trap 'rm -f "$bundle"' EXIT
  head -c 200M > "$bundle"
else
  [ $# -eq 2 ] || { echo "usage: release.sh <bundle.tar.gz> <revision>" >&2; exit 2; }
  bundle=$1 rev=$2
  [[ $rev =~ ^[A-Za-z0-9._-]+$ ]] || { echo "error: bad revision '$rev'" >&2; exit 2; }
fi
rel=$app/releases/$rev
mkdir -p "$app/releases" "$app/ui"

# what is running now, to go back to
prev=""
if [ -L "$app/current" ]; then
  prev=$(readlink "$app/current")
elif [ -x "$app/pebblelab" ] && [ -d "$app/ui/dist" ]; then
  # the first deploy: keep the hand-installed build as a release to go back to
  prev=$app/releases/before-ci
  mkdir -p "$prev/ui" && cp "$app/pebblelab" "$prev/" && cp -r "$app/ui/dist" "$prev/ui/dist"
fi
# a push to prod and a prod tag on the same commit deploy it twice: the second has nothing to do
if [ "$prev" = "$rel" ]; then
  echo "pebblelab $rev is already live"
  exit 0
fi

rm -rf "$rel" && mkdir -p "$rel"
tar xzf "$bundle" -C "$rel" --no-same-owner
[ -x "$rel/pebblelab" ] && [ -d "$rel/ui/dist" ] || { echo "error: the build has no pebblelab binary or ui/dist" >&2; exit 1; }

swap_in() { # <release dir>
  install -m 755 "$1/pebblelab" "$app/pebblelab.new" && mv -f "$app/pebblelab.new" "$app/pebblelab"
  rm -rf "$app/ui/dist.new" && cp -r "$1/ui/dist" "$app/ui/dist.new"
  rm -rf "$app/ui/dist.old" && { [ -d "$app/ui/dist" ] && mv "$app/ui/dist" "$app/ui/dist.old" || true; }
  mv "$app/ui/dist.new" "$app/ui/dist"
  ln -sfn "$1" "$app/current"
}

healthy() {
  for _ in $(seq 1 30); do
    curl -fs -o /dev/null "$health" && return 0
    sleep 1
  done
  return 1
}

swap_in "$rel"
if ! sudo -n /usr/bin/systemctl restart pebblelab; then
  # the old build is still the one running: put its files back so the next deploy of this revision is not "already live"
  echo "error: could not restart pebblelab; the deploy user needs the sudoers rule in the README" >&2
  if [ -n "$prev" ] && [ -d "$prev" ]; then swap_in "$prev"; else rm -f "$app/current"; fi
  rm -rf "$rel"
  exit 1
fi
if healthy; then
  echo "pebblelab $rev is up"
  # keep the last five releases
  ls -1dt "$app"/releases/*/ | grep -v "/before-ci/$" | tail -n +6 | xargs -r rm -rf
  exit 0
fi

echo "error: pebblelab $rev did not answer $health" >&2
journalctl -u pebblelab -n 40 --no-pager >&2 || true
if [ -n "$prev" ] && [ -d "$prev" ]; then
  echo "going back to $(basename "$prev")" >&2
  swap_in "$prev"
  sudo -n /usr/bin/systemctl restart pebblelab
  healthy && echo "$(basename "$prev") is back up" >&2
fi
exit 1
