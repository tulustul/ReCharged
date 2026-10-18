"""Toy-store gondola shelf, variant A (blue header, bear pictogram). See shelfkit.py.

    blender -b -P tools/blender/props/toy/shelf_a.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import shelfkit  # noqa: E402

shelfkit.build('shelf_a', seed=7, header_color=shelfkit.BLUE, pictogram='bear')
