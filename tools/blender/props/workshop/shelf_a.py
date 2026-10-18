"""Stocked steel shelving (decor), variant a. See shelfkit.py.

    blender -b -P tools/blender/props/workshop/shelf_a.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import shelfkit  # noqa: E402

shelfkit.build('shelf_a', seed=1)
