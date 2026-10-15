#!/usr/bin/env bash
# Runs on the server, as the app's user, from the deploy workflow: release.sh <bundle.tar.gz> <revision>
# Unpacks the build into ~/apps/pebblelab/releases/<revision>, swaps the binary and web app in, restarts the
# service, and puts the previous release back if it does not answer /health. The database migrates itself on start.
set -euo pipefail
bundle=$1 rev=$2
app=${PEBBLELAB_APP_DIR:-$HOME/apps/pebblelab}
health=${PEBBLELAB_HEALTH:-http://127.0.0.1:3000/health}
rel=$app/releases/$rev

mkdir -p "$rel" "$app/ui"
tar xzf "$bundle" -C "$rel"

# what is running now, to go back to
prev=""
if [ -L "$app/current" ]; then
  prev=$(readlink "$app/current")
elif [ -x "$app/pebblelab" ] && [ -d "$app/ui/dist" ]; then
  # the first deploy: keep the hand-installed build as a release to go back to
  prev=$app/releases/before-ci
  mkdir -p "$prev/ui" && cp "$app/pebblelab" "$prev/" && cp -r "$app/ui/dist" "$prev/ui/dist"
fi

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
sudo -n /usr/bin/systemctl restart pebblelab
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
