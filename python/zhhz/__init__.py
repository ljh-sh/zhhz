"""zhhz — Simplified/Traditional Chinese converter (PyO3 binding)."""

# maturin builds the entire Rust extension as the top-level `zhhz` module
# (per [tool.maturin] in pyproject.toml). This __init__.py exists so
# pip can install as a regular Python package and so future Python-only
# helpers (formatters, CLI wrappers) can live alongside the native code.
from zhhz import (  # re-export from the native extension module
    Converter,
    Detection,
    convert,
    convert_region,
    convert_with_custom,
    detect,
    configs,
    locales,
)

__version__ = "0.7.8"
__all__ = [
    "Converter",
    "Detection",
    "convert",
    "convert_region",
    "convert_with_custom",
    "detect",
    "configs",
    "locales",
]