#!/usr/bin/env python3
"""
For test files that used glob imports `use crate::domain::entities::*;` etc.,
replace the glob with explicit `use` statements based on the map built from
mod.rs files. The map building is shared with remove_reexports.py — we just
re-import that module."""

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from remove_reexports import build_mappings  # type: ignore  # noqa: E402

API_ROOT = Path("services/api/src")

# Globs we'll expand
GLOB_PATHS = [
    "crate::domain::entities",
    "crate::ports::inbound",
    "crate::ports::outbound",
    "crate::application",
    "crate::domain::value_objects",
    "crate::domain::enums",
]


def main():
    item_map, module_map = build_mappings()
    combined = {**module_map, **item_map}

    # Reverse map per parent prefix : parent -> { short_name -> full_path }
    by_parent = {}
    for short, full in combined.items():
        m = re.match(r"^(crate::[\w:]+)::(\w+)$", short)
        if not m:
            continue
        parent, name = m.group(1), m.group(2)
        by_parent.setdefault(parent, {})[name] = full

    affected = 0
    for rs in API_ROOT.rglob("*.rs"):
        text = rs.read_text(encoding="utf-8")
        new = text
        for parent in GLOB_PATHS:
            glob_use = f"use {parent}::*;"
            if glob_use not in new:
                continue
            # Find unresolved names by scanning the body for tokens that match
            # parent's known short names.
            names = by_parent.get(parent, {})
            if not names:
                continue
            used = []
            for name in names:
                # Match name as standalone identifier (not part of another path)
                pattern = re.compile(r"(?<![\w:])" + re.escape(name) + r"(?![\w])")
                if pattern.search(new):
                    used.append(name)
            if not used:
                # No references — just remove the glob line
                new = new.replace(glob_use + "\n", "").replace(glob_use, "")
                continue
            used.sort()
            # Build explicit use lines
            lines = []
            for name in used:
                full = names[name]
                # Strip the parent::leaf where leaf == name -> use full
                # If the original symbol differs (alias), need `as name`
                last_seg = full.split("::")[-1]
                if last_seg == name:
                    lines.append(f"use {full};")
                else:
                    lines.append(f"use {full} as {name};")
            replacement = "\n".join(lines)
            new = new.replace(glob_use, replacement)
        if new != text:
            rs.write_text(new, encoding="utf-8")
            affected += 1
    print(f"Patched {affected} files.")


if __name__ == "__main__":
    main()
