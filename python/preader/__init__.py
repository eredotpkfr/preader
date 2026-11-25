"""A file reader that tracks read percentage and manages read state."""

from . import preader
from .preader import *  # noqa: F403

__doc__ = getattr(preader, "__doc__", "")
__all__ = list(getattr(preader, "__all__", []))
