#!/usr/bin/env bash
# Fetches Microsoft's official ONNX Runtime build that the release packages ship
# for Demucs stem splitting (docs/spec/stems.md), and puts its shared libraries
# in <out-dir>:
#
#   linux    libonnxruntime.so      CPU (a system ONNX Runtime with CUDA or ROCm
#                                   is preferred when one is installed)
#   macos    libonnxruntime.dylib   universal; CoreML (GPU, Neural Engine) and CPU
#   windows  onnxruntime.dll        DirectML (any DirectX 12 GPU) and CPU; it uses
#                                   Windows's own DirectML.dll. WITH_DIRECTML=1 also
#                                   ships Microsoft's newer redistributable
#                                   DirectML.dll, which is under Microsoft's own
#                                   licence, not MIT: the owner decides.
#
# ONNX Runtime is Microsoft's, MIT licence; the licence goes beside the
# libraries as ONNXRUNTIME-LICENSE.txt.
#
# Usage: packaging/onnxruntime/fetch.sh <linux|macos|windows> <out-dir>
set -euo pipefail

ORT=1.22.0
DIRECTML=1.15.4

platform=${1:?platform}
out=${2:?out dir}
mkdir -p "$out"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fetch() { curl -fsSL --retry 3 -o "$2" "$1"; }
unpack() {
  if command -v unzip >/dev/null; then unzip -q "$1" -d "$2"; else python -m zipfile -e "$1" "$2"; fi
}

case "$platform" in
  linux)
    name="onnxruntime-linux-x64-$ORT"
    fetch "https://github.com/microsoft/onnxruntime/releases/download/v$ORT/$name.tgz" "$tmp/ort.tgz"
    tar -xzf "$tmp/ort.tgz" -C "$tmp"
    cp "$tmp/$name/lib/libonnxruntime.so.$ORT" "$out/libonnxruntime.so"
    cp "$tmp/$name/LICENSE" "$out/ONNXRUNTIME-LICENSE.txt"
    ;;
  macos)
    name="onnxruntime-osx-universal2-$ORT"
    fetch "https://github.com/microsoft/onnxruntime/releases/download/v$ORT/$name.tgz" "$tmp/ort.tgz"
    tar -xzf "$tmp/ort.tgz" -C "$tmp"
    cp "$tmp/$name/lib/libonnxruntime.$ORT.dylib" "$out/libonnxruntime.dylib"
    install_name_tool -id "@rpath/libonnxruntime.dylib" "$out/libonnxruntime.dylib" 2>/dev/null || true
    cp "$tmp/$name/LICENSE" "$out/ONNXRUNTIME-LICENSE.txt"
    ;;
  windows)
    fetch "https://www.nuget.org/api/v2/package/Microsoft.ML.OnnxRuntime.DirectML/$ORT" "$tmp/ort.zip"
    mkdir -p "$tmp/ort"
    unpack "$tmp/ort.zip" "$tmp/ort"
    cp "$tmp/ort/runtimes/win-x64/native/onnxruntime.dll" "$out/"
    licence=$(find "$tmp/ort" -maxdepth 1 -iname 'LICENSE*' | head -n 1)
    cp "$licence" "$out/ONNXRUNTIME-LICENSE.txt"
    if [ "${WITH_DIRECTML:-0}" = 1 ]; then
      fetch "https://www.nuget.org/api/v2/package/Microsoft.AI.DirectML/$DIRECTML" "$tmp/dml.zip"
      mkdir -p "$tmp/dml"
      unpack "$tmp/dml.zip" "$tmp/dml"
      cp "$tmp/dml/bin/x64-win/DirectML.dll" "$out/"
    fi
    ;;
  *)
    echo "unknown platform $platform" >&2
    exit 2
    ;;
esac
ls -l "$out"
