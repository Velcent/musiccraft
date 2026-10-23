"""Writes crates/musiccraft-demucs/src/spectrum.json: HTDemucs's own
transforms (`_spec`, `_ispec`) at a small size, for the Rust tests."""

import json
import sys
import types

import torch
from demucs.htdemucs import HTDemucs

torch.manual_seed(2)
me = types.SimpleNamespace(nfft=256, hop_length=64)
x = torch.randn(1, 1000)
z = HTDemucs._spec(me, x)[0]
w = torch.randn(z.shape, dtype=torch.complex64)
y = HTDemucs._ispec(me, w[None], length=1000)[0]
def short(t):
    return [round(v, 6) for v in t.tolist()]


json.dump(
    {
        "x": short(x[0]),
        "spec_re": short(z.real.flatten()),
        "spec_im": short(z.imag.flatten()),
        "frames": z.shape[-1],
        "z_re": short(w.real.flatten()),
        "z_im": short(w.imag.flatten()),
        "inverse": short(y),
    },
    open(sys.argv[1], "w"),
)
