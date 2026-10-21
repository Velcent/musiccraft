"""Checks the logs and renders that run.sh collected (docs/smoke-tests.md).

usage: python3 check.py <work folder>
Prints one line per check and exits 1 if any failed.
"""

import math
import re
import sys
import wave
from pathlib import Path

work = Path(sys.argv[1])
failures = 0


def check(ok, what, detail=""):
    global failures
    failures += 0 if ok else 1
    print(("PASS " if ok else "FAIL ") + what + (f" ({detail})" if detail else ""))


def text(path):
    return path.read_text(errors="replace") if path.exists() else ""


def samples(path):
    """The left channel of a 16/24/32-bit WAV, as floats."""
    with wave.open(str(path)) as w:
        width, channels = w.getsampwidth(), w.getnchannels()
        raw = w.readframes(w.getnframes())
    scale = float(1 << (8 * width - 1))
    values = [
        int.from_bytes(raw[i : i + width], "little", signed=True) / scale
        for i in range(0, len(raw), width)
    ]
    return values[0::channels]


def rms(x):
    return math.sqrt(sum(v * v for v in x) / max(len(x), 1))


def biggest_difference(a, b):
    return max((abs(x - y) for x, y in zip(a, b)), default=1.0)


reaper = work / "reaper"
build = text(reaper / "build.log")
if build:
    print("-- REAPER", build.split("\n", 1)[0])
    check("LOAD-FAILED" not in build and "NO-PARAMS" not in build, "every plugin loads, with parameters")
    counts = dict(re.findall(r"^loaded (\w+) (\d+)$", build, re.M))
    expected = {"CLAP": "44", "VST3": "44", "CLAPi": "12", "VST3i": "12"}
    check(counts == expected, "44 effects and 12 instruments per format", str(counts))
    check(build.count("editor open true") == 4, "editors open in REAPER windows")
    params = re.findall(r"^param (\w+ \w+) Time = (.*) ([\d.]+)$", build, re.M)
    check(
        len(params) == 4 and all(p[1] == "1.0 beats" and abs(float(p[2]) - 0.25) < 1e-4 for p in params),
        "host-set parameters reach the plugins and read back with our text",
        "; ".join(f"{p[0]}: {p[1]}" for p in params),
    )
    reopen = text(reaper / "reopen.log")
    kept = re.findall(r"param after reopen (.*) ([\d.]+)$", reopen, re.M)
    check(
        len(kept) == 4 and all(k[0] == "1.0 beats" and abs(float(k[1]) - 0.25) < 1e-4 for k in kept),
        "values come back when the project is reopened",
    )
    for name in ["CLAP-Synth", "CLAP-Antiserum", "VST3-Synth", "VST3-Antiserum"]:
        first, again = reaper / f"first-{name}.wav", reaper / f"again-{name}.wav"
        if not (first.exists() and again.exists()):
            check(False, f"{name} renders", "missing")
            continue
        a, b = samples(first), samples(again)
        check(rms(a) > 0.01, f"{name} plays notes", f"rms {rms(a):.3f}")
        check(biggest_difference(a, b) == 0.0, f"{name} renders the same after reopening")
    a, b = reaper / "first-CLAP-Synth.wav", reaper / "first-VST3-Synth.wav"
    if a.exists() and b.exists():
        check(biggest_difference(samples(a), samples(b)) == 0.0, "the CLAP and VST3 synths render the same")
    for fmt in ["CLAP", "VST3"]:
        names = ["dry", "off", "on", "on-no-key"]
        paths = {n: reaper / f"sc-{fmt}-{n}.wav" for n in names}
        if not all(p.exists() for p in paths.values()):
            check(False, f"{fmt} sidechain renders", "missing")
            continue
        s = {n: samples(p) for n, p in paths.items()}
        check(biggest_difference(s["on"], s["off"]) > 0.01, f"{fmt} compressor: the sidechain switch changes the sound")
        check(
            biggest_difference(s["on-no-key"], s["dry"]) < 1e-6,
            f"{fmt} compressor: a silent key through REAPER's routing reduces nothing",
        )
        check(biggest_difference(s["on"], s["on-no-key"]) > 0.01, f"{fmt} compressor: the key track's signal arrives")
else:
    print("-- REAPER: not run")

ardour = text(work / "ardour" / "smoke.log")
if ardour:
    print("-- Ardour")
    peaks = re.findall(r"^(\w+) master peak before notes (\S+) dB, while playing (\S+) dB", ardour, re.M)
    check(len(peaks) == 2, "Synth and Antiserum load as VST3 instruments in a session")
    for inst, before, playing in peaks:
        check(float(playing) > -30.0 and float(playing) > float(before) + 10.0, f"{inst} plays live MIDI", f"{before} -> {playing} dB")
    reopened = re.findall(r"^reopened (\w+) (.*) delay param ([\d.]+)", ardour, re.M)
    check(
        len(reopened) == 2 and all(abs(float(r[2]) - 0.6) < 1e-4 for r in reopened),
        "plugins and a parameter set through Ardour come back when the session is reopened",
        "; ".join(f"{r[0]}: {r[1]}" for r in reopened),
    )
else:
    print("-- Ardour: not run")

print(f"{failures} failed")
sys.exit(1 if failures else 0)
