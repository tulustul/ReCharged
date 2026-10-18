#!/usr/bin/env python3
"""Render the ReCharged soundtrack: assets/audio/music/<cue>.ogg + analysis PNGs.

usage: python3 tools/audio/music/render.py [cue ...] [--png DIR] [--jobs N]

Each race cue is rendered together with its `_final` variant (same worker, shared note cache).
After encoding, every OGG is decoded again and checked: loudness (own meter + ffmpeg ebur128),
true peak, length and loop seam. Output is deterministic.
"""
from __future__ import annotations

import argparse
import importlib
import json
import subprocess
import sys
import time
from multiprocessing import Pool
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import analysis  # noqa: E402
import dsp  # noqa: E402
import song as S  # noqa: E402

OUT = HERE.parents[2] / "assets" / "audio" / "music"
GROUPS = {   # group -> [(cue file name, "module:builder", kwargs)]
    "menu_theme": [("menu_theme", "menu_theme:build", {})],
    **{f"race_{i}": [(f"race_{i}", f"race_{i}:build", {}),
                     (f"race_{i}_final", f"race_{i}:build", {"final": True})] for i in range(1, 5)},
    "stingers": [("results_jingle", "stingers:results_jingle", {}),
                 ("podium_fanfare", "stingers:podium_fanfare", {}),
                 ("cup_victory", "stingers:cup_victory", {})],
}
TARGET = {"_final": -13.5}   # final-lap variants are mastered 0.5 LU hotter for lift
# libvorbis quality. At q5/q6 the last ~40 ms of a stream carry up to 3.4 dB more coding error
# than the same audio mid-stream; at q8 the seam is as clean as anywhere else (see seam_codec_db).
VORBIS_Q = {"loop": "8", "oneshot": "6"}


def encode(x, path, title, q):
    pcm = np.ascontiguousarray(x.T, dtype="<f4").tobytes()
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "f32le", "-ar", str(dsp.SR), "-ac", "2",
                    "-i", "pipe:", "-c:a", "libvorbis", "-q:a", q,
                    "-fflags", "+bitexact", "-flags:a", "+bitexact",   # fixed Ogg serial -> same bytes
                    "-metadata",
                    f"title={title}", "-metadata", "artist=ReCharged (procedural)", str(path)],
                   input=pcm, check=True)


def seam_codec_db(y, dec, tmp):
    """Codec error at the loop seam (last + first 40 ms of the stream) minus the error on the same
    audio when it sits mid-stream (loop encoded rotated by half). ~0 dB = the seam is clean."""
    n, h, w = y.shape[1], y.shape[1] // 2, int(0.04 * dsp.SR)
    encode(np.roll(y, h, axis=1), tmp, "", VORBIS_Q["loop"])
    er = analysis.decode(tmp) - np.roll(y, h, axis=1)
    tmp.unlink()
    e = dec - y
    ms = lambda a: float(np.mean(a ** 2) + 1e-30)
    return 10 * np.log10(ms(np.concatenate([e[:, n - w:], e[:, :w]], axis=1)) / ms(er[:, h - w: h + w]))


def finish_oneshot(y, song):
    """Trim a stinger once its tail falls below -50 dBFS and fade the last 300 ms."""
    env = np.abs(y).max(axis=0)
    end = max(song.length(), int(np.nonzero(env > dsp.undb(-50))[0][-1]) + 1)
    y = y[:, : min(end, y.shape[1])].copy()
    return dsp.fade_tail(y, 0.3)


def render_cue(name, ref, kw, png_dir):
    t0 = time.time()
    mod, fn = ref.split(":")
    song = getattr(importlib.import_module(f"cues.{mod}"), fn)(**kw)
    target = TARGET.get(name[-6:], song.target)
    ceiling = -2.0
    x = S.mix(song)
    for _ in range(3):   # re-master if the encoded file's true peak overshoots -1 dBTP
        y = S.master(x, song.loop, target, ceiling)
        if not song.loop:
            y = finish_oneshot(y, song)
        path = OUT / f"{name}.ogg"
        encode(y, path, name, VORBIS_Q["loop" if song.loop else "oneshot"])
        dec = analysis.decode(path)
        ff = analysis.ffmpeg_loudness(path)
        if ff["TP"] <= -1.2:
            break
        ceiling -= ff["TP"] + 1.3
    m = {"name": name, "bpm": song.bpm, "key": song.key, "bars": song.bars, "loop": song.loop,
         "samples": y.shape[1], "decoded": dec.shape[1], "seconds": round(y.shape[1] / dsp.SR, 3),
         "lufs": round(dsp.lufs(dec), 2), "ff_I": ff["I"], "ff_LRA": ff["LRA"], "ff_TP": ff["TP"],
         "peak": round(float(dsp.todb(np.abs(dec).max())), 2), "ceiling": round(ceiling, 2),
         "kb": path.stat().st_size // 1024, "render_s": round(time.time() - t0, 1)}
    if song.loop:
        m.update(analysis.seam(dec), seam_codec_db=round(seam_codec_db(y, dec, png_dir / f"{name}_rot.ogg"), 2))
        analysis.plot_seam(dec, png_dir / f"{name}_seam.png")
    lines = [f"{song.key}, {song.bpm} BPM, {song.bars} bars, {m['seconds']} s, "
             f"{'loop' if song.loop else 'one-shot'}",
             f"LUFS {m['lufs']} (ffmpeg {ff['I']}), LRA {ff['LRA']}, true peak {ff['TP']} dBTP, "
             f"sample peak {m['peak']} dBFS" + (f", seam jump {m['jump']:.4f} ({m['jump_pct']:.1f}th pct of "
                                                  f"|diff|), seam codec error {m['seam_codec_db']:+.1f} dB vs "
                                                  "mid-stream" if song.loop else "")]
    analysis.plot(dec, png_dir / f"{name}.png", name, song.sections, song.bpm, lines)
    np.save(png_dir / f"{name}_ltas.npy", np.vstack(analysis.ltas(dec)))
    return m


def run_group(args):
    group, png_dir = args
    S.CACHE.clear()
    return [render_cue(n, b, kw, Path(png_dir)) for n, b, kw in GROUPS[group]]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cues", nargs="*", help=f"groups to render (default all): {' '.join(GROUPS)}")
    ap.add_argument("--png", default=str(HERE.parents[2] / "target" / "music-preview"))
    ap.add_argument("--jobs", type=int, default=len(GROUPS))
    a = ap.parse_args()
    groups = a.cues or list(GROUPS)
    png = Path(a.png)
    png.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    with Pool(min(a.jobs, len(groups))) as pool:
        res = [m for ms in pool.imap_unordered(run_group, [(g, str(png)) for g in groups]) for m in ms]
    res.sort(key=lambda m: m["name"])
    curves = {}
    for f in sorted(png.glob("*_ltas.npy")):
        fs, db = np.load(f)
        curves[f.stem[:-5]] = (fs, db)
    analysis.plot_ltas(curves, png / "ltas.png")
    print(f"{'cue':16s} {'sec':>8s} {'LUFS':>6s} {'ffI':>6s} {'TP':>6s} {'peak':>6s} {'LRA':>5s} "
          f"{'seam%':>6s} {'codec':>6s} {'kB':>5s} {'t':>5s}")
    for m in res:
        print(f"{m['name']:16s} {m['seconds']:8.3f} {m['lufs']:6.2f} {m['ff_I']:6.1f} {m['ff_TP']:6.1f} "
              f"{m['peak']:6.2f} {m['ff_LRA']:5.1f} {m.get('jump_pct', float('nan')):6.1f} "
              f"{m.get('seam_codec_db', float('nan')):6.1f} {m['kb']:5d} {m['render_s']:5.1f}")
    (png / "report.json").write_text(json.dumps(res, indent=1))
    bad = [m["name"] for m in res if m["ff_TP"] > -1.0 or abs(m["lufs"] - (-14.0)) > 1.0
           or m["samples"] != m["decoded"]
           or (m["loop"] and (m["jump_pct"] > 99.9 or m["seam_codec_db"] > 1.5))]
    if bad:
        sys.exit(f"FAILED checks: {bad}")


if __name__ == "__main__":
    main()
