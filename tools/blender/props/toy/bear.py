"""Giant plush teddy bear (1.5 m display bear), sitting with its legs forward (+Y): honey-brown
fur, cream muzzle, ear insides and paw pads, black button eyes, a red bow tie. ~6.5 u tall.
Static.

    blender -b -P tools/blender/props/toy/bear.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from toykit import *  # noqa: E402,F403

core.reset()
fabric('fur', '#A8743F')
fabric('cream', '#E9D9BC')
plastic('button', '#1B1A1C', 0.15)
fabric('bow', '#C8322A', 0.7)
fabric('stitch', '#5A3A22')
mi = core.mi
fur, deco = core.new_bm(), core.new_bm()


def blob(bm, c, r, mat, scale=(1, 1, 1), n=20, rings=10):
    return geo.sphere(bm, V(c), r, mi(mat), n=n, rings=rings, scale=scale)


blob(fur, (0, 0, 1.75), 1.3, 'fur', (1.0, 0.85, 1.12))              # body
blob(deco, (0, 0.62, 1.6), 0.75, 'cream', (1.0, 0.55, 1.15), 20, 10)  # tummy patch
blob(fur, (0, 0.1, 3.95), 1.15, 'fur', (1.05, 0.95, 0.95))            # head
blob(deco, (0, 0.98, 3.72), 0.48, 'cream', (1.1, 0.75, 0.82), 20, 10)  # muzzle
blob(deco, (0, 1.32, 3.86), 0.16, 'button', (1.3, 0.9, 0.95), 12, 6)   # nose
for s in (-1, 1):
    blob(deco, (s * 0.42, 0.98, 4.25), 0.12, 'button', (1, 0.8, 1), 12, 6)  # eyes
    blob(fur, (s * 0.85, 0.0, 4.85), 0.42, 'fur', (1, 0.55, 1), 14, 7)        # ears
    blob(deco, (s * 0.85, 0.18, 4.83), 0.27, 'cream', (1, 0.4, 1), 12, 6)
    arm = [V((s * 1.05, 0.15, 2.55)), V((s * 1.45, 0.55, 1.7)), V((s * 1.35, 0.95, 1.0))]
    geo.tube(fur, arm, 0.42, mi('fur'), n=12, corner=0.5, cseg=3)
    blob(fur, arm[-1], 0.44, 'fur', (1, 1, 1), 14, 7)                      # paw
    leg = [V((s * 0.7, 0.3, 0.56)), V((s * 0.85, 1.6, 0.6))]  # resting on the floor
    geo.tube(fur, leg, 0.55, mi('fur'), n=14)
    blob(fur, leg[0], 0.55, 'fur', (1, 1, 1), 14, 7)
    blob(fur, leg[1], 0.6, 'fur', (1, 0.8, 1), 16, 8)                      # foot
    blob(deco, (s * 0.85, 2.05, 0.62), 0.42, 'cream', (1, 0.2, 1.05), 14, 7)  # foot pad
# bow tie under the chin
c = V((0, 0.82, 3.05))
for s in (-1, 1):
    geo.loft(deco, [[c + V((s * 0.08, 0, dz)) for dz in (-0.12, 0.12)] +
                    [c + V((s * 0.62, 0, dz)) for dz in (0.3, -0.3)],
                    [c + V((s * 0.08, 0.14, dz)) for dz in (-0.12, 0.12)] +
                    [c + V((s * 0.62, 0.14, dz)) for dz in (0.3, -0.3)]], mi('bow'))
blob(deco, c + V((0, 0.08, 0)), 0.16, 'bow', (1, 0.9, 1), 12, 6)
propkit.finish(ENV, 'bear', [smooth('fur', fur), smooth('details', deco)],
               car=(2.8, 3.2, 200))
