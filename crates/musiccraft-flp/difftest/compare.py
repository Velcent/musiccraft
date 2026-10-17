"""Compares PyFLP's dumps (py-*.json) with musiccraft-flp's (rs-*.json).

Usage: python compare.py DIR
A PyFLP {"error": ...} where Rust gives null counts as agreement; one where
Rust gives a value is listed as "fixed" (the port reads what PyFLP raises on).
Everything else that differs is a mismatch. Exit status 1 on any mismatch.
"""

import collections
import json
import math
import pathlib
import re
import sys


def is_err(v):
    return isinstance(v, dict) and set(v) == {"error"}


def walk(py, rs, path, out):
    if is_err(py):
        agree = rs is None or is_err(rs)
        out["agree_error" if agree else "fixed"][(path, py["error"])] += 1
        return
    if isinstance(py, float) and math.isnan(py) and rs is None:
        return  # serde_json writes NaN as null
    if isinstance(py, float) or isinstance(rs, float):
        if isinstance(py, (int, float)) and isinstance(rs, (int, float)) \
                and not isinstance(py, bool) and not isinstance(rs, bool) \
                and math.isclose(py, rs, rel_tol=1e-9, abs_tol=1e-9):
            return
        out["mismatch"][(path, f"{py!r} != {rs!r}")] += 1
        return
    if isinstance(py, dict) and isinstance(rs, dict):
        for k in sorted(set(py) | set(rs)):
            if k not in rs:
                out["mismatch"][(f"{path}.{k}", "missing in rust")] += 1
            elif k not in py:
                out["mismatch"][(f"{path}.{k}", "extra in rust")] += 1
            else:
                walk(py[k], rs[k], f"{path}.{k}", out)
        return
    if isinstance(py, list) and isinstance(rs, list):
        if len(py) != len(rs):
            out["mismatch"][(path, f"length {len(py)} != {len(rs)}")] += 1
        for i, (a, b) in enumerate(zip(py, rs)):
            walk(a, b, f"{path}[{i}]", out)
        return
    if py != rs or type(py) is not type(rs):
        out["mismatch"][(path, f"{py!r} != {rs!r}")] += 1


def generic(path):
    return re.sub(r"\[\d+\]", "[]", path)


def main():
    d = pathlib.Path(sys.argv[1])
    total = collections.Counter()
    bad = False
    for py_file in sorted(d.glob("py-*.json")):
        name = py_file.name[3:]
        rs_file = d / f"rs-{name}"
        py = json.loads(py_file.read_text() or "null")
        rs = json.loads(rs_file.read_text() or "null")
        out = collections.defaultdict(collections.Counter)
        walk(py, rs, "", out)
        for kind in ("agree_error", "fixed", "mismatch"):
            total[kind] += sum(out[kind].values())
        print(f"== {name}: {sum(out['mismatch'].values())} mismatches, "
              f"{sum(out['fixed'].values())} fixed, {sum(out['agree_error'].values())} py-error/rs-null")
        grouped = collections.Counter()
        examples = {}
        for (path, msg), n in out["mismatch"].items():
            key = generic(path)
            grouped[key] += n
            examples.setdefault(key, f"{path}: {msg}")
        for key, n in grouped.most_common():
            bad = True
            print(f"   MISMATCH x{n} {examples[key]}")
        fixed = collections.Counter()
        for (path, err), n in out["fixed"].items():
            fixed[(generic(path), err)] += n
        for (key, err), n in sorted(fixed.items()):
            print(f"   fixed x{n} {key} (PyFLP {err})")
    print(f"TOTAL {dict(total)}")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
