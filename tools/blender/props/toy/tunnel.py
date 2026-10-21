"""Giant rainbow pop-up play tunnel the track drives through: fabric bands in rainbow colours
over spring-steel hoops every 2 u, an arched (semicircular) cross-section of inner radius 6.6 u
and outer 7.0 u centred at the floor, 12 u long along Y (prop z), open at both ends. A glowing
LED strip runs along every hoop's inner side (`lamp_*`, dimmed by the time of day; the catalog's
`lights` light the inside). Symmetric left-right. Static.

    blender -b -P tools/blender/props/toy/tunnel.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

R_IN, R_OUT, HALF, PITCH = 6.6, 7.0, 6.0, 2.0
SEG = 32                                   # arc segments
core.reset()
BANDS = [RED, ORANGE, YELLOW, GREEN, BLUE, PURPLE]
for k, c in enumerate(BANDS):
    fabric(f'band{k}', c, 0.85)
plastic('hoop', '#24305E', 0.5)            # navy fabric binding over the steel
LEDS = {'lamp_magenta': ('#FF7AE6', (1.0, 0.0, 0.75)), 'lamp_cyan': ('#7AF0FF', (0.0, 0.75, 1.0)),
        'lamp_lime': ('#C8FF7A', (0.45, 1.0, 0.0)), 'lamp_amber': ('#FFC870', (1.0, 0.45, 0.0))}
for name, (base, glow) in LEDS.items():
    core.material(name, base, 0.0, 0.3, emission=glow, strength=24.0)
mi = core.mi


def arc(r, a0=0.0, a1=math.pi, n=SEG):
    return [V((r * math.cos(a0 + (a1 - a0) * i / n), r * math.sin(a0 + (a1 - a0) * i / n)))
            for i in range(n + 1)]


bands = core.new_bm()
ring = arc(R_OUT - 0.08) + arc(R_IN + 0.02)[::-1]   # annulus in (x, z)
n_bands = int(2 * HALF / PITCH)
for k in range(n_bands):
    y0 = -HALF + k * PITCH
    extrude(bands, 'y', ring, y0, y0 + PITCH, mi(f'band{k % len(BANDS)}'))

hoops, leds = core.new_bm(), core.new_bm()
lamp_names = list(LEDS)
for k in range(n_bands + 1):
    y = -HALF + k * PITCH
    # binding tube centred in the shell; its ends stop 0.2 u above the floor, a foot pad below
    a_end = math.asin(0.2 / 6.8)
    path = [V((p.x, y, p.y)) for p in arc(6.8, a_end, math.pi - a_end)]
    geo.tube(hoops, path, 0.2, mi('hoop'), n=8)
    for s in (-1, 1):
        geo.box(hoops, V((s * 6.8, y, 0.1)), (0.5, 0.5, 0.2), mi('hoop'))
    a_led = math.asin(0.45 / 6.5)
    led = [V((p.x, y, p.y)) for p in arc(6.52, a_led, math.pi - a_led, 28)]
    geo.tube(leds, led, 0.07, mi(lamp_names[k % len(lamp_names)]), n=5)

propkit.finish(ENV, 'tunnel', [smooth('bands', bands, angle=40), smooth('hoops', hoops),
                               smooth('leds', leds)],
               budget=propkit.HERO_TRIS, car=(1.5, -9.0, 0))
