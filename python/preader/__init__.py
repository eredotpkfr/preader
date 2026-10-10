"""A Python package that reads a file along with its read percentage."""

from . import preader
from .preader import *  # noqa: F403

__doc__ = getattr(preader, "__doc__", "")
__all__ = list(getattr(preader, "__all__", []))
