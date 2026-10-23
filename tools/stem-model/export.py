"""Exports Demucs's HTDemucs to ONNX for MusicCraft's stem splitter.

MusicCraft runs the network with ONNX Runtime and does the rest itself
(`crates/musiccraft-demucs`): splitting the song into segments, the short-time
transforms on both sides of the network and the overlap-add. So the exported
graph is HTDemucs's `forward` between the spectrogram and the mask:

    inputs  mix  (1, 2, L)          one segment, 44.1 kHz stereo
            spec (1, 4, 2048, T)    its spectrogram, real and imaginary parts
                                    of each channel as four planes
    outputs freq (1, S, 4, 2048, T) each source's spectrogram, same layout
            time (1, S, 2, L)       each source's time-branch waveform

L is the model's training segment (343 980 samples for htdemucs) and
T = ceil(L / 1024). The source order is written to the model's metadata.

Usage:
    python export.py out.onnx                 # the released htdemucs weights
    python export.py out.onnx --random-weights  # same network, untrained,
                                                # for tests without a download
    python export.py out.onnx --reference dir/   # also writes a test case

Demucs is by Alexandre Défossez and Meta, MIT licence
(https://github.com/adefossez/demucs).
"""

import argparse
import json
import math

import torch
from einops import rearrange
from torch import nn


class Core(nn.Module):
    """HTDemucs.forward from the normalised spectrogram to the outputs."""

    def __init__(self, model):
        super().__init__()
        self.m = model

    def forward(self, mix, mag):
        m = self.m
        x = mag
        B, C, Fq, T = x.shape
        mean = x.mean(dim=(1, 2, 3), keepdim=True)
        std = x.std(dim=(1, 2, 3), keepdim=True)
        x = (x - mean) / (1e-5 + std)
        xt = mix
        meant = xt.mean(dim=(1, 2), keepdim=True)
        stdt = xt.std(dim=(1, 2), keepdim=True)
        xt = (xt - meant) / (1e-5 + stdt)

        saved, saved_t, lengths, lengths_t = [], [], [], []
        for idx, encode in enumerate(m.encoder):
            lengths.append(x.shape[-1])
            inject = None
            if idx < len(m.tencoder):
                lengths_t.append(xt.shape[-1])
                tenc = m.tencoder[idx]
                xt = tenc(xt)
                if not tenc.empty:
                    saved_t.append(xt)
                else:
                    inject = xt
            x = encode(x, inject)
            if idx == 0 and m.freq_emb is not None:
                frs = torch.arange(x.shape[-2], device=x.device)
                emb = m.freq_emb(frs).t()[None, :, :, None].expand_as(x)
                x = x + m.freq_emb_scale * emb
            saved.append(x)
        if m.crosstransformer:
            if m.bottom_channels:
                b, c, f, t = x.shape
                x = rearrange(x, "b c f t-> b c (f t)")
                x = m.channel_upsampler(x)
                x = rearrange(x, "b c (f t)-> b c f t", f=f)
                xt = m.channel_upsampler_t(xt)
            x, xt = m.crosstransformer(x, xt)
            if m.bottom_channels:
                x = rearrange(x, "b c f t-> b c (f t)")
                x = m.channel_downsampler(x)
                x = rearrange(x, "b c (f t)-> b c f t", f=f)
                xt = m.channel_downsampler_t(xt)
        for idx, decode in enumerate(m.decoder):
            skip = saved.pop(-1)
            x, pre = decode(x, skip, lengths.pop(-1))
            offset = m.depth - len(m.tdecoder)
            if idx >= offset:
                tdec = m.tdecoder[idx - offset]
                length_t = lengths_t.pop(-1)
                if tdec.empty:
                    pre = pre[:, :, 0]
                    xt, _ = tdec(pre, None, length_t)
                else:
                    skip = saved_t.pop(-1)
                    xt, _ = tdec(xt, skip, length_t)
        S = len(m.sources)
        x = x.view(B, S, -1, Fq, T)
        x = x * std[:, None] + mean[:, None]
        xt = xt.view(B, S, -1, mix.shape[-1])
        xt = xt * stdt[:, None] + meant[:, None]
        return x, xt


def load(random_weights):
    from demucs.pretrained import get_model

    if random_weights:
        from demucs.htdemucs import HTDemucs

        # htdemucs's own settings (demucs/remote/htdemucs.yaml and its
        # training config), with untrained weights.
        torch.manual_seed(0)
        model = HTDemucs(
            sources=["drums", "bass", "other", "vocals"],
            samplerate=44100,
            segment=7.8,
            channels=48,
            t_layers=5,
            bottom_channels=512,
            depth=4,
            nfft=4096,
            cac=True,
        )
    else:
        bag = get_model("htdemucs")
        model = bag.models[0]
    model.eval()
    return model


def export(model, path):
    length = int(model.segment * model.samplerate)
    frames = math.ceil(length / model.hop_length)
    mix = torch.zeros(1, 2, length)
    spec = torch.zeros(1, 4, model.nfft // 2, frames)
    core = Core(model).eval()
    # The fused attention kernel has no ONNX form; the plain one does.
    torch.backends.mha.set_fastpath_enabled(False)
    with torch.no_grad():
        torch.onnx.export(
            core,
            (mix, spec),
            path,
            input_names=["mix", "spec"],
            output_names=["freq", "time"],
            opset_version=17,
            dynamo=False,
        )
    import onnx

    graph = onnx.load(path)
    info = {
        "sources": list(model.sources),
        "sample_rate": model.samplerate,
        "segment": length,
        "nfft": model.nfft,
        "hop": model.hop_length,
    }
    entry = graph.metadata_props.add()
    entry.key = "musiccraft"
    entry.value = json.dumps(info)
    onnx.save(graph, path)
    print(f"wrote {path}: {info}")


def reference(model, folder, seconds=12.0):
    """A test case for `crates/musiccraft-demucs`: a mix and what Demucs
    itself makes of it (no random shifts), as raw little-endian floats."""
    import os

    from demucs.apply import apply_model

    torch.manual_seed(1)
    n = int(seconds * model.samplerate)
    t = torch.arange(n) / model.samplerate
    left = 0.3 * torch.sin(2 * math.pi * 220 * t) + 0.05 * torch.randn(n)
    right = 0.3 * torch.sin(2 * math.pi * 330 * t) + 0.05 * torch.randn(n)
    wav = torch.stack([left, right])
    ref = wav.mean(0)
    mean, std = ref.mean(), ref.std() + 1e-8
    with torch.no_grad():
        out = apply_model(model, ((wav - mean) / std)[None], shifts=0, split=True, overlap=0.25)
    out = out[0] * std + mean
    os.makedirs(folder, exist_ok=True)
    wav.numpy().astype("<f4").tofile(os.path.join(folder, "mix.f32"))
    out.numpy().astype("<f4").tofile(os.path.join(folder, "stems.f32"))
    with open(os.path.join(folder, "reference.json"), "w") as f:
        json.dump({"frames": n, "sources": list(model.sources)}, f)
    print(f"wrote {folder}: stems {tuple(out.shape)}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("out")
    parser.add_argument("--random-weights", action="store_true")
    parser.add_argument("--reference")
    args = parser.parse_args()
    model = load(args.random_weights)
    export(model, args.out)
    if args.reference:
        reference(model, args.reference)


if __name__ == "__main__":
    main()
