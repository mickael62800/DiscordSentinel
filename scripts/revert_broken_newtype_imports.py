#!/usr/bin/env python3
"""
Revert broken newtype imports introduced by PR8-14 migrations on workers
and bot files. These files import `sentinel_api::domain::entities::system
::discord_ids::*` but workers/bot don't depend on `sentinel-api` (and the
bot has type collisions with serenity's `GuildId`/`ChannelId`/etc.).

Strategy : delete the broken imports, then revert any usage of the
newtype as a struct field or function param type back to `String`.
"""

import re
import sys
from pathlib import Path

NEWTYPES = ["GuildId", "UserId", "ChannelId", "MessageId", "RoleId"]

BROKEN_IMPORT_PATTERNS = [
    re.compile(r"^use sentinel_api::domain::entities::system::discord_ids::\w+;\n", re.MULTILINE),
    re.compile(r"^use crate::domain::entities::system::discord_ids::\w+;\n", re.MULTILINE),
]


def revert_file(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    new = text

    # 1. Delete broken use lines
    for pat in BROKEN_IMPORT_PATTERNS:
        new = pat.sub("", new)

    # 2. Revert struct fields and function params : `: NewType,` and `: NewType\n`
    #    But NOT when prefixed by `serenity::` or `::` (that would be a path)
    for nt in NEWTYPES:
        # Match `pub field: NewType,` or `field: NewType,` or `field: NewType\n` or `field: NewType)`
        # Use negative lookbehind to skip when preceded by `::` (path qualified)
        new = re.sub(
            rf"(?<!:):\s*{nt}\b(?!::)",
            ": String",
            new,
        )

    if new != text:
        path.write_text(new, encoding="utf-8")
        return True
    return False


def main():
    files_arg = sys.argv[1:] if len(sys.argv) > 1 else None
    if files_arg:
        files = [Path(f) for f in files_arg]
    else:
        # Default: all known broken
        roots = [Path("services/workers"), Path("bots/sentinel-bot/src"), Path("bots/shared/src")]
        broken = set()
        for root in roots:
            for rs in root.rglob("*.rs"):
                t = rs.read_text(encoding="utf-8")
                if any(p.search(t) for p in BROKEN_IMPORT_PATTERNS):
                    broken.add(rs)
        files = sorted(broken)

    patched = 0
    for f in files:
        if revert_file(f):
            patched += 1
            print(f"PATCHED {f}")
    print(f"\n{patched} files patched")


if __name__ == "__main__":
    main()
