#!/usr/bin/env python3
"""Converte docs/openapi.yaml em docs/openapi.json (embutido no binario)."""
import json
import pathlib
import sys

try:
    import yaml
except ImportError:
    sys.exit("instale pyyaml: python -m pip install pyyaml")

root = pathlib.Path(__file__).resolve().parent.parent
src = root / "docs" / "openapi.yaml"
dst = root / "docs" / "openapi.json"

with src.open(encoding="utf-8") as fh:
    spec = yaml.safe_load(fh)

with dst.open("w", encoding="utf-8", newline="\n") as fh:
    json.dump(spec, fh, ensure_ascii=False, indent=2)
    fh.write("\n")

print(f"{dst} atualizado")
