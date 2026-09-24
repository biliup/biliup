"""Load the repository's root ``biliup`` package under this name.

Editable installs (``maturin develop``, ``pip install -e .``) only put
``python-source`` on ``sys.path``, and the root ``biliup/`` reaches wheels
through ``include``, which editable installs skip. Wheels never contain this file.
"""

import importlib.util
import sys
from pathlib import Path

_package = Path(__file__).resolve().parents[3] / "biliup"
_spec = importlib.util.spec_from_file_location(
    __name__, _package / "__init__.py", submodule_search_locations=[str(_package)]
)
_module = importlib.util.module_from_spec(_spec)
sys.modules[__name__] = _module
_spec.loader.exec_module(_module)
