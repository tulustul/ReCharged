"""Toy-store gondola shelf, variant B (red header, star pictogram). See shelfkit.py.

    blender -b -P tools/blender/props/toy/shelf_b.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import shelfkit  # noqa: E402

shelfkit.build('shelf_b', seed=21, header_color=shelfkit.RED, pictogram='star')
