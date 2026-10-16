#!/bin/sh
# Differential test of musiccraft-flp against PyFLP.
#
# Reads every file in tests/assets with PyFLP (writing tests/golden, which
# `cargo test -p musiccraft-flp` compares against) and with the Rust port,
# then prints a report of every difference.
#
# Needs PyFLP at commit f937126 (the version ported; its main branch as of
# July 2023) on Python 3.10, as PyFLP does not run on newer Pythons:
#   uv python install 3.10
#   uv venv -p 3.10 venv && venv/bin/pip install \
#     git+https://github.com/demberto/PyFLP@f937126b888ce94271bfea631b89166c74056530 \
#     construct==2.10.68 construct-typing==0.5.6 f-enum==0.2.0 \
#     sortedcontainers==2.4.0 typing_extensions==4.7.1
#   PYTHON=venv/bin/python crates/musiccraft-flp/difftest/run.sh
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
CRATE=$(dirname "$HERE")
ASSETS=$CRATE/tests/assets
GOLDEN=$CRATE/tests/golden
PYTHON=${PYTHON:-python3.10}
OUT=$(mktemp -d)
trap 'rm -rf "$OUT"' EXIT

cargo build -q -p musiccraft-flp --example flp_dump --manifest-path "$CRATE/Cargo.toml"
BIN=$(cargo metadata --format-version 1 --no-deps --manifest-path "$CRATE/Cargo.toml" |
  "$PYTHON" -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/debug/examples/flp_dump

rm -f "$GOLDEN"/*.json
find "$ASSETS" -type f \( -name '*.flp' -o -name '*.fst' -o -name '*.fsc' \) | sort | while read -r f; do
  key=$(echo "${f#"$ASSETS"/}" | tr '/ ' '__')
  # -I and a neutral working directory: nothing next to the assets is imported.
  (cd "$OUT" && "$PYTHON" -I "$HERE/dump_pyflp.py" "$f") > "$GOLDEN/$key.json"
  cp "$GOLDEN/$key.json" "$OUT/py-$key.json"
  "$BIN" "$f" > "$OUT/rs-$key.json"
  case "$f" in *.fst|*.fsc)
    (cd "$OUT" && "$PYTHON" -I "$HERE/dump_pyflp.py" "$f" preset) > "$GOLDEN/preset-$key.json"
    cp "$GOLDEN/preset-$key.json" "$OUT/py-preset-$key.json"
    "$BIN" "$f" preset > "$OUT/rs-preset-$key.json" ;;
  esac
done
"$PYTHON" -I "$HERE/compare.py" "$OUT"
