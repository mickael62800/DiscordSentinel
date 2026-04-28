#!/usr/bin/env python3
"""
For each `pub use SUBMOD::*;` glob in a handler mod.rs (handlers/coude/mod.rs,
handlers/casino/blackjack/mod.rs, handlers/moderation/mod.rs, handlers/system/mod.rs),
parse SUBMOD for its public functions/types, then sed call sites that used
`PARENT::ITEM` to `PARENT::SUBMOD::ITEM`. Finally drop the glob from mod.rs.
"""

import re
import sys
from pathlib import Path

API_ROOT = Path("services/api/src")

# (parent_mod_dir, parent_path_used_in_callers)
PARENTS = [
    (API_ROOT / "adapters/inbound/http/handlers/coude",
     "handlers::coude"),
    (API_ROOT / "adapters/inbound/http/handlers/casino/blackjack",
     "handlers::casino::blackjack"),
    (API_ROOT / "adapters/inbound/http/handlers/moderation",
     "handlers::moderation"),
    (API_ROOT / "adapters/inbound/http/handlers/system",
     "handlers::system"),
]

PUB_USE_GLOB = re.compile(r"^\s*pub use (\w+)::\*;\s*$", re.MULTILINE)
# Public items in a submodule file : pub fn / pub async fn / pub struct / pub enum / pub type / pub const
PUB_ITEM = re.compile(
    r"^\s*pub(?:\(\w+\))?\s+(?:async\s+)?(?:fn|struct|enum|type|const|trait)\s+(\w+)",
    re.MULTILINE,
)


def collect_items(submod_path: Path):
    if not submod_path.exists():
        return set()
    text = submod_path.read_text(encoding="utf-8")
    return set(m.group(1) for m in PUB_ITEM.finditer(text))


def process_parent(parent_dir: Path, parent_path: str):
    mod_rs = parent_dir / "mod.rs"
    if not mod_rs.exists():
        return 0
    text = mod_rs.read_text(encoding="utf-8")

    globs = PUB_USE_GLOB.findall(text)
    if not globs:
        return 0

    # Build : item_name -> submod
    item_to_submod = {}
    for submod in globs:
        items = collect_items(parent_dir / f"{submod}.rs")
        # Also inspect submod/mod.rs (for nested modules)
        items |= collect_items(parent_dir / submod / "mod.rs")
        for it in items:
            if it in item_to_submod and item_to_submod[it] != submod:
                print(f"WARN: {it} in both {item_to_submod[it]} and {submod}", file=sys.stderr)
            item_to_submod[it] = submod

    # Sed in all .rs files : `parent_path::ITEM` -> `parent_path::SUBMOD::ITEM`
    pattern = re.compile(
        r"\b" + re.escape(parent_path) + r"::(\w+)(?![\w:])"
    )
    total = 0
    for rs in API_ROOT.rglob("*.rs"):
        # Don't rewrite the submod files themselves (they don't reference parent path that way)
        rs_text = rs.read_text(encoding="utf-8")

        def repl(m):
            item = m.group(1)
            if item in item_to_submod:
                return f"{parent_path}::{item_to_submod[item]}::{item}"
            return m.group(0)

        new = pattern.sub(repl, rs_text)
        if new != rs_text:
            rs.write_text(new, encoding="utf-8")
            total += 1

    # Strip `pub use SUBMOD::*;` lines
    new_mod = PUB_USE_GLOB.sub("", text)
    new_mod = re.sub(r"\n{3,}", "\n\n", new_mod)
    mod_rs.write_text(new_mod, encoding="utf-8")

    print(f"{parent_path} : {len(globs)} globs, {len(item_to_submod)} items mapped, {total} files patched.")
    return total


def main():
    total = 0
    for parent_dir, parent_path in PARENTS:
        total += process_parent(parent_dir, parent_path)
    print(f"\nTotal files patched : {total}")


if __name__ == "__main__":
    main()
