from htmy.core import Formatter, SafeStr, XBool
from htmy.core import xml_format_string as xml_format_string

from htmy_rs._native import configure as _configure

# Must run before any use of the native module: tag construction and rendering
# both depend on the interned htmy types.
_configure(SafeStr, XBool, Formatter, xml_format_string)
