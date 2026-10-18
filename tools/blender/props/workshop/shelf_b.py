"""Stocked steel shelving (decor), variant b. See shelfkit.py.

    blender -b -P tools/blender/props/workshop/shelf_b.py -- [--preview DIR]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import shelfkit  # noqa: E402

shelfkit.build('shelf_b', seed=2)
