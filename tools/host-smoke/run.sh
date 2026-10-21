#!/usr/bin/env bash
# Smoke tests of the exported plugins in real hosts (docs/smoke-tests.md):
# REAPER (CLAP and VST3) and Ardour (VST3), headless under Xvfb.
#
# usage: tools/host-smoke/run.sh [bundle folder]   (default: target/bundled)
#
# Needs: xvfb-run, jackd (JACK's dummy driver gives REAPER a running audio
# engine), python3; REAPER for Linux (set REAPER to its `reaper` binary, or
# have it on PATH) and/or Ardour 8 (`ardour8-lua`). A missing host is skipped.
# Plugins are installed into a temporary HOME, so your own plugin folders and
# host settings are not touched.
set -u
here=$(cd "$(dirname "$0")" && pwd)
bundles=$(cd "${1:-$here/../../target/bundled}" && pwd)
work=$(mktemp -d)
export HOME="$work/home"
mkdir -p "$HOME/.clap" "$HOME/.vst3" "$work/reaper" "$work/ardour"
cp -R "$bundles"/*.clap "$HOME/.clap/"
cp -R "$bundles"/*.vst3 "$HOME/.vst3/"
echo "bundles from $bundles; work in $work"

jack_pid=""
if command -v jackd >/dev/null && ! pgrep -x jackd >/dev/null; then
  jackd --no-realtime -d dummy -r 48000 -p 512 >"$work/jackd.txt" 2>&1 &
  jack_pid=$!
  sleep 2
fi

# Runs one REAPER script until it logs "done".
reaper_run() {
  local script=$1 phase=$2 log=$3 out=$4
  MC_LOG=$log MC_DIR=$out MC_PHASE=$phase \
    xvfb-run -a "$reaper" -nosplash -newinst "$here/$script" >"$work/reaper-$phase.txt" 2>&1 &
  local pid=$!
  for _ in $(seq 1 600); do
    grep -q "^done" "$log" 2>/dev/null && break
    kill -0 $pid 2>/dev/null || break
    sleep 1
  done
  sleep 1
  pkill -f "$reaper" 2>/dev/null
  wait $pid 2>/dev/null
}

reaper=${REAPER:-$(command -v reaper || true)}
if [ -n "$reaper" ] && [ -x "$reaper" ]; then
  echo "REAPER: $reaper"
  reaper_run reaper-smoke.lua build "$work/reaper/build.log" "$work/reaper"
  reaper_run reaper-smoke.lua reopen "$work/reaper/reopen.log" "$work/reaper"
  reaper_run reaper-sidechain.lua sidechain "$work/reaper/sidechain.log" "$work/reaper"
else
  echo "REAPER: not found (set REAPER=/path/to/reaper); skipped"
fi

if command -v ardour8-lua >/dev/null; then
  echo "Ardour: $(command -v ardour8-lua)"
  scanner=$(ls /usr/lib/ardour8/ardour*-vst3-scanner 2>/dev/null | head -1)
  for b in "$HOME"/.vst3/*.vst3; do
    LD_LIBRARY_PATH=$(dirname "$scanner") "$scanner" "$b" >>"$work/ardour/scan.txt" 2>&1
  done
  MC_DIR="$work/ardour/session" ardour8-lua "$here/ardour-smoke.lua" >"$work/ardour/smoke.log" 2>&1
else
  echo "Ardour: ardour8-lua not found; skipped"
fi

[ -n "$jack_pid" ] && kill "$jack_pid" 2>/dev/null
python3 "$here/check.py" "$work"
