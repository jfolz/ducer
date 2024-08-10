from importlib import metadata

from ._fst import *

__all__ = (
    "Automaton",
    "Buffer",
    "Map",
    "Op",
    "Set",
)

try:
    __version__ = metadata.version("ducer")
except metadata.PackageNotFoundError:
    __version__ = "1.0.3"
del metadata
