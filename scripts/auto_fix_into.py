#!/usr/bin/env python3
"""Lit la sortie de `cargo check` sur stdin et insere `.into()` aux sites
E0308 ChannelId/RoleId/MessageId/UserId/GuildId quand le pattern est trivial
(une seule expression suivie d'une virgule ou fin de ligne).

Usage : cargo check -p sentinel-api 2>&1 | python scripts/auto_fix_into.py
"""

import re
import sys
from pathlib import Path

# Parser pour les erreurs cargo
ERROR_RE = re.compile(r"-->\s+(.+?):(\d+):(\d+)")
EXPECTED_RE = re.compile(
    r"expected `(GuildId|UserId|ChannelId|MessageId|RoleId)`,\s+found `String`|"
    r"expected `String`,\s+found `(GuildId|UserId|ChannelId|MessageId|RoleId)`|"
    r"expected `Option<(GuildId|UserId|ChannelId|MessageId|RoleId)>`,\s+found `Option<String>`"
)


def parse_cargo(text):
    """Yields (file, line, col) for each E0308 error."""
    blocks = re.split(r"\nerror\[", text)
    for block in blocks:
        if not block.startswith("E0308]"):
            continue
        # Match expected/found
        if not EXPECTED_RE.search(block):
            continue
        m = ERROR_RE.search(block)
        if m:
            yield m.group(1), int(m.group(2)), int(m.group(3))


def fix_line(line):
    """Insere .into() avant la virgule de fin (ou la fin de ligne)."""
    # Si la ligne se termine par une virgule, insere .into() juste avant.
    stripped = line.rstrip("\n").rstrip("\r")
    trailing = line[len(stripped):]  # \n or \r\n
    if stripped.endswith(","):
        body = stripped[:-1]
        # Ne pas double-coller .into()
        if body.rstrip().endswith(".into()"):
            return line
        return body.rstrip() + ".into(),\n" + trailing[1:] if "\r\n" in line else body.rstrip() + ".into(),\n"
    elif stripped.endswith(")"):
        # Cas `func(arg)` au bout, ne touche pas
        return line
    else:
        if stripped.rstrip().endswith(".into()"):
            return line
        return stripped.rstrip() + ".into()" + trailing


def main():
    text = sys.stdin.read()
    sites = list(parse_cargo(text))
    # Group by file, then process highest line numbers first to avoid offset shifts
    by_file = {}
    for f, l, _ in sites:
        by_file.setdefault(f, set()).add(l)
    patched = 0
    for f, lines_set in by_file.items():
        # Normalize Windows path
        path = Path(f.replace("\\", "/"))
        if not path.exists():
            continue
        text2 = path.read_text(encoding="utf-8")
        lines = text2.splitlines(keepends=True)
        for l in sorted(lines_set, reverse=True):
            idx = l - 1
            if 0 <= idx < len(lines):
                lines[idx] = fix_line(lines[idx])
        new_text = "".join(lines)
        if new_text != text2:
            path.write_text(new_text, encoding="utf-8")
            patched += 1
    print(f"patched {patched} files, {len(sites)} sites")


if __name__ == "__main__":
    main()
