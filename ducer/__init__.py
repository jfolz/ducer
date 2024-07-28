from importlib import metadata

from ._fst import *

try:
    __version__ = metadata.version("ducer")
except metadata.PackageNotFoundError:
    __version__ = "1.0.0"
del metadata
