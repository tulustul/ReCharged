"""Self-checks for the music generator. Run: python3 tools/audio/music/test_music.py (~10 s)."""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

import analysis  # noqa: E402
import drums as D  # noqa: E402
import dsp  # noqa: E402
import instruments as I  # noqa: E402
import song as S  # noqa: E402


def test_svf_matches_rbj_biquad():
    x = dsp.rng("t").standard_normal(1 << 15)
    f = np.fft.rfftfreq(len(x), 1 / dsp.SR)
    for mode, kind in (("lp", "lp"), ("hp", "hp")):
        ref = np.fft.irfft(np.fft.rfft(x) * dsp.response([(kind, 1000.0, 2.0)], f), len(x))
        assert np.abs(dsp.svf(x, 1000.0, 2.0, mode)[4000:-4000] - ref[4000:-4000]).max() < 1e-9


def test_loudness_meter_calibration():
    t = np.arange(10 * dsp.SR) / dsp.SR
    x = 0.1 * np.sin(2 * np.pi * 997 * t)
    assert abs(dsp.lufs(np.vstack([x, x])) + 20.0) < 0.05          # BS.1770: 0 dBFS L+R sine = 0 LUFS
    assert abs(dsp.lufs(np.vstack([x, 0 * x])) + 23.01) < 0.05


def test_limiter_holds_true_peak_ceiling():
    x = dsp.rng("lim").standard_normal((2, 3 * dsp.SR)) * 0.6
    assert dsp.true_peak_db(dsp.limiter(x, -2.0)) < -1.9


def test_drum_grid_and_notes_land_on_the_beat():
    s = S.Song("t", bpm=120)
    s.section("a", 2, "C G")
    tr = s.track("k", D.kick)
    s.beat(tr, "X...x...|..o.....", "a")
    assert [(n.t, n.v) for n in tr.notes] == [(0.0, 1.0), (1.0, 0.78), (2.5, 0.5),
                                              (4.0, 1.0), (5.0, 0.78), (6.5, 0.5)]
    mel = s.track("m", I.lead)
    s.notes(mel, "C4:2 [E4,G4]:4! -:2 D4:8~", "a")
    assert [(n.t, n.p, n.slide) for n in mel.notes][:4] == [(0.0, 60, False), (0.5, 64, False),
                                                            (0.5, 67, False), (2.0, 62, True)]
    b = s.track("b", I.bass)
    s.rel(b, "R:8 N-1:8", "a", floor=28)
    assert [n.p for n in b.notes] == [36, 30, 31, 35]               # C2, F#1 (->G), G1, B1 (->C)


def test_loop_render_is_seamless_and_deterministic():
    def build():
        s = S.Song("t", bpm=120, seed=7)
        s.section("a", 2, "Am F")
        s.beat(s.track("k", D.kick, level=-6, sidechain=True), "X...X...X...X...", "a")
        s.chords(s.track("p", I.pad, level=-10, duck=0.5, verb=-6, release=1.5), "a")
        s.notes(s.track("l", I.lead, mono=True, delay=-6), "E5:6 C5:2 A4:8", "a")
        return s
    y1 = S.master(S.mix(build()), True)
    S.CACHE.clear()
    y2 = S.master(S.mix(build()), True)
    assert np.array_equal(y1, y2)
    assert analysis.seam(y1)["jump_pct"] < 99.0
    assert abs(dsp.lufs(y1) + 14.0) < 0.1 and dsp.true_peak_db(y1) <= -1.5


if __name__ == "__main__":
    for name, fn in list(globals().items()):
        if name.startswith("test_"):
            fn()
            print("ok", name)
