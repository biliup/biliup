from .stream_gears import *
from .pyobject import Segment, Credit

__all__ = ['Segment', 'Credit']
__doc__ = stream_gears.__doc__
if hasattr(stream_gears, "__all__"):
    __all__.extend(stream_gears.__all__)
