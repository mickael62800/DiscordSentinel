#!/usr/bin/env python3
"""Migrate `<id>: String` -> `<id>: <NewType>` on struct fields.

Generic version of the per-id scripts used in PR9-PR12. Pass the field
name + newtype name as args.

Usage : python scripts/migrate_id.py user_id UserId
"""

import re
import sys
from pathlib import Path


def main():
    if len(sys.argv) != 3:
        print("Usage: migrate_id.py FIELD NEWTYPE", file=sys.stderr)
        sys.exit(1)
    field, newtype = sys.argv[1], sys.argv[2]

    ROOTS = [
        Path("services/api/src"),
        Path("services/api/tests"),
        Path("bots/sentinel-bot/src"),
        Path("services/workers"),
        Path("bots/shared/src"),
    ]
    CRATE_IMPORT = f"use crate::domain::entities::system::discord_ids::{newtype};"
    EXTERNAL_IMPORT = f"use sentinel_api::domain::entities::system::discord_ids::{newtype};"

    # Regex : `pub <field>: String` (struct field), but NOT `<field>: String::new()`
    # The negative lookahead `(?!\s*::)` skips paths.
    field_re = re.compile(
        rf"\b(pub\s+)?{re.escape(field)}:\s*String(?!\s*::)\b"
    )
    # Word-boundary check : ensures we don't false-positive on substrings like
    # `XxxUserId` containing `UserId`.
    has_newtype_re = re.compile(rf"\b{re.escape(newtype)}\b")

    patched = 0
    for root in ROOTS:
        if not root.exists():
            continue
        for rs in root.rglob("*.rs"):
            text = rs.read_text(encoding="utf-8")
            new = field_re.sub(
                lambda m: (m.group(1) or "") + f"{field}: {newtype}", text
            )
            if new == text:
                continue
            if not has_newtype_re.search(text):
                in_external_crate = (
                    "services/workers" in str(rs).replace("\\", "/")
                    or "bots/sentinel-bot" in str(rs).replace("\\", "/")
                    or "bots/shared" in str(rs).replace("\\", "/")
                )
                import_line = EXTERNAL_IMPORT if in_external_crate else CRATE_IMPORT
                lines = new.split("\n")
                last_use = -1
                for i, line in enumerate(lines):
                    if line.startswith("use "):
                        last_use = i
                if last_use >= 0:
                    lines.insert(last_use + 1, import_line)
                    new = "\n".join(lines)
            rs.write_text(new, encoding="utf-8")
            patched += 1
    print(f"patched {patched} files")


if __name__ == "__main__":
    main()
