#!/usr/bin/env python3
"""
Remove all `pub use` re-exports from API mod.rs files and rewrite consumer
imports to use fully-qualified paths.

For each mod.rs of the form:
    pub mod ai;
    pub mod coude;
    pub use coude::bet::{CoudeBet, FighterBetBonus as CoudeFighterBetBonus};
    pub use coude::bet;            # module re-export
    pub use system::analytics;     # module re-export

Builds a mapping :
    crate::PARENT::CoudeBet                  -> crate::PARENT::coude::bet::CoudeBet
    crate::PARENT::CoudeFighterBetBonus      -> crate::PARENT::coude::bet::FighterBetBonus
    crate::PARENT::bet::                     -> crate::PARENT::coude::bet::
    crate::PARENT::analytics::               -> crate::PARENT::system::analytics::

Then walks all *.rs in services/api/src and sed's the mappings.
Finally rewrites each mod.rs to keep only `pub mod XXX;` lines.
"""

import re
import sys
from pathlib import Path

API_ROOT = Path("services/api/src")
API_TESTS_ROOT = Path("services/api/tests")

# (mod.rs path, parent path used by consumers)
MODS = [
    (API_ROOT / "domain/entities/mod.rs",            "crate::domain::entities"),
    (API_ROOT / "domain/services/mod.rs",            "crate::domain::services"),
    (API_ROOT / "domain/value_objects/mod.rs",       "crate::domain::value_objects"),
    (API_ROOT / "domain/enums/mod.rs",               "crate::domain::enums"),
    (API_ROOT / "application/mod.rs",                "crate::application"),
    (API_ROOT / "ports/inbound/mod.rs",              "crate::ports::inbound"),
    (API_ROOT / "ports/outbound/mod.rs",             "crate::ports::outbound"),
    (API_ROOT / "adapters/inbound/http/handlers/mod.rs",
                                                     "crate::adapters::inbound::http::handlers"),
    # rbac.rs has only `pub use crate::domain::enums::Role;` — handled inline
]

# Block parser : match `pub use PATH;` or `pub use PATH::{...};` (multi-line)
RE_PUB_USE = re.compile(
    r"^pub\s+use\s+([\w:]+(?:::\{[^}]*\})?);\s*$",
    re.MULTILINE,
)
# In one path::{...}, items may include alias: `Foo as Bar`
RE_ITEM = re.compile(r"\s*(\w+)(?:\s+as\s+(\w+))?\s*$")


def resolve_full(parent: str, base: str) -> str:
    """If base is absolute (starts with `crate::` or `self::`), use it as-is.
    Otherwise prepend parent."""
    if base.startswith("crate::") or base.startswith("self::"):
        return base
    return f"{parent}::{base}"


def parse_mod(mod_path: Path, parent: str):
    """Yield (kind, short_full_in_parent, true_full) tuples for every re-export."""
    text = mod_path.read_text(encoding="utf-8")

    # Strip line comments to avoid matching `// pub use ...`
    text_no_comments = re.sub(r"//[^\n]*", "", text)

    # Collapse newlines within braces so the regex on a single line works
    flattened = re.sub(r"\{([^{}]*)\}",
                       lambda m: "{" + m.group(1).replace("\n", " ") + "}",
                       text_no_comments)

    for m in RE_PUB_USE.finditer(flattened):
        full = m.group(1).strip()
        if "::{" in full:
            base, items_block = full.split("::{", 1)
            items_block = items_block.rstrip("}")
            base = base.strip()
            items = [it.strip() for it in items_block.split(",") if it.strip()]
            base_full = resolve_full(parent, base)
            for it in items:
                im = RE_ITEM.match(it)
                if not im:
                    continue
                original, alias = im.group(1), im.group(2)
                consumer_name = alias or original
                if original == "self":
                    # `pub use coude::bet::{self, ...}` -> module `bet` at parent
                    short = f"{parent}::{base.split('::')[-1]}"
                    yield ("module", short, base_full)
                else:
                    short = f"{parent}::{consumer_name}"
                    yield ("item", short, f"{base_full}::{original}")
        else:
            # `pub use ai::inference_limiter;` (or even `pub use crate::foo::bar;`)
            base_full = resolve_full(parent, full)
            short = f"{parent}::{full.split('::')[-1]}"
            yield ("module", short, base_full)


def build_mappings():
    """Return (item_map, module_map). Item map: 'short_path' -> 'full_path'."""
    item_map = {}
    module_map = {}
    for mod_path, parent in MODS:
        if not mod_path.exists():
            continue
        for kind, short, full in parse_mod(mod_path, parent):
            if kind == "item":
                if short in item_map and item_map[short] != full:
                    print(f"WARN duplicate item {short}: {item_map[short]} vs {full}", file=sys.stderr)
                item_map[short] = full
            else:
                module_map[short] = full
    return item_map, module_map


def expand_grouped_uses(text: str) -> str:
    """Expand `use base::{A, B as C, sub::D};` into one `use ...;` per item
    so each individual path can be mapped independently."""
    USE_BLOCK = re.compile(
        r"^(?P<indent>[ \t]*)(?P<vis>(?:pub(?:\(\w+\))?\s+)?)use\s+"
        r"(?P<base>[\w:]+)::\{(?P<items>[^{}]*)\};\s*$",
        re.MULTILINE,
    )

    def expand(m):
        indent = m.group("indent")
        vis = m.group("vis")
        base = m.group("base")
        items_block = m.group("items")
        items = [it.strip() for it in items_block.split(",") if it.strip()]
        lines = []
        for it in items:
            # `self` -> import the module itself, not `base::self`
            if it == "self":
                lines.append(f"{indent}{vis}use {base};")
            elif it.startswith("self as "):
                alias = it[len("self as "):].strip()
                lines.append(f"{indent}{vis}use {base} as {alias};")
            else:
                lines.append(f"{indent}{vis}use {base}::{it};")
        return "\n".join(lines)

    # Repeat until stable (handles nested braces — though Rust `use` rarely nests)
    prev = None
    while prev != text:
        prev = text
        text = USE_BLOCK.sub(expand, text)
    return text


def apply_to_file(path: Path, item_map: dict, module_map: dict, dry_run=False):
    """Apply substitutions. Step 1 : expand grouped uses. Step 2 : sed paths."""
    original = path.read_text(encoding="utf-8")
    text = expand_grouped_uses(original)

    # Items can also be prefixes when accessing enum variants : `Action::None`,
    # `Role::Admin`. Merge maps for prefix matching.
    combined = {**module_map, **item_map}
    sorted_combined = sorted(combined.items(), key=lambda kv: -len(kv[0]))

    # Also match `sentinel_api::...` (used in integration tests where the
    # crate is referenced by its public name).
    PATH_RE = re.compile(r"(?<![\w:])((?:crate|sentinel_api)(?:::\w+)+)")

    def replace(m):
        path_str = m.group(1)
        # If path is `sentinel_api::...`, normalise to `crate::...` for lookup
        # then translate back.
        prefix = ""
        if path_str.startswith("sentinel_api::"):
            prefix = "sentinel_api::"
            path_for_lookup = "crate::" + path_str[len("sentinel_api::"):]
        else:
            path_for_lookup = path_str

        for short, full in sorted_combined:
            if path_for_lookup == short or path_for_lookup.startswith(short + "::"):
                replaced = full + path_for_lookup[len(short):]
                if prefix:
                    return prefix + replaced[len("crate::"):]
                return replaced
        return path_str

    # Iterate until fixed-point so chained re-exports compose
    # (e.g. value_objects -> enums -> enums/moderation/...).
    total = 0
    for _ in range(5):  # safety cap
        new, n = PATH_RE.subn(replace, text)
        total += n
        if new == text:
            break
        text = new

    if (text != original) and not dry_run:
        path.write_text(text, encoding="utf-8")
    return total


def rewrite_mod(mod_path: Path):
    """Strip all `pub use` statements (incl. `as ALIAS` and multi-line braces)."""
    text = mod_path.read_text(encoding="utf-8")
    # `pub use a::b::c::{...};` (single-line, possibly multi-line in source but
    # collapse here)
    # First collapse `pub use ... { ... };` spanning newlines
    def collapse_braces(m):
        return m.group(0).replace("\n", " ")
    text = re.sub(r"pub\s+use\s+[\w:]+::\{[^}]*\};", collapse_braces, text)
    # Now strip them
    text = re.sub(
        r"^pub\s+use\s+[\w:]+(?:\s*::\s*\{[^}]*\})?(?:\s+as\s+\w+)?\s*;\s*\n",
        "",
        text,
        flags=re.MULTILINE,
    )
    text = re.sub(r"\n{3,}", "\n\n", text)
    mod_path.write_text(text, encoding="utf-8")


def expand_glob_imports(item_map: dict, module_map: dict):
    """For test files using `use crate::PARENT::*;`, replace the glob with
    explicit `use` statements based on the mappings. Must run BEFORE
    `pub use` lines are stripped from mod.rs (we don't depend on them, but
    the substitutions in apply_to_file would already have rewritten the
    glob's parent path otherwise)."""
    glob_paths = [
        "crate::domain::entities",
        "crate::ports::inbound",
        "crate::ports::outbound",
        "crate::application",
        "crate::domain::value_objects",
        "crate::domain::enums",
    ]
    combined = {**module_map, **item_map}
    by_parent = {}
    for short, full in combined.items():
        m = re.match(r"^(crate::[\w:]+)::(\w+)$", short)
        if m:
            by_parent.setdefault(m.group(1), {})[m.group(2)] = full

    all_globs = glob_paths + [p.replace("crate::", "sentinel_api::") for p in glob_paths]

    affected = 0
    for root in (API_ROOT, API_TESTS_ROOT):
        if not root.exists():
            continue
        for rs in root.rglob("*.rs"):
            text = rs.read_text(encoding="utf-8")
            new = text
            for parent in all_globs:
                glob_use = f"use {parent}::*;"
                if glob_use not in new:
                    continue
                # Lookup canonical (crate::) form regardless of source prefix
                canonical = parent.replace("sentinel_api::", "crate::")
                prefix_xform = (lambda f: f.replace("crate::", "sentinel_api::")) \
                    if parent.startswith("sentinel_api::") else (lambda f: f)
                names = by_parent.get(canonical, {})
                used = sorted(
                    n for n in names
                    if re.search(r"(?<![\w:])" + re.escape(n) + r"(?![\w])", new)
                )
                if not used:
                    new = new.replace(glob_use + "\n", "").replace(glob_use, "")
                    continue
                by_mod = {}
                for n in used:
                    full = names[n]
                    mp = "::".join(full.split("::")[:-1])
                    by_mod.setdefault(mp, []).append((n, full))
                lines = []
                for mp, items in by_mod.items():
                    has_alias = any(it[1].split("::")[-1] != it[0] for it in items)
                    if has_alias:
                        for n, full in items:
                            last = full.split("::")[-1]
                            f_xf = prefix_xform(full)
                            lines.append(f"use {f_xf};" if last == n else f"use {f_xf} as {n};")
                    else:
                        lines.append(f"use {prefix_xform(mp)}::*;")
                lines.sort()
                new = new.replace(glob_use, "\n".join(lines))
            if new != text:
                rs.write_text(new, encoding="utf-8")
                affected += 1
    print(f"Expanded globs in {affected} files.")


def main():
    item_map, module_map = build_mappings()
    print(f"Built {len(item_map)} item mappings, {len(module_map)} module mappings.")

    # Order matters :
    # 1. expand globs (uses parent paths still present in mod.rs)
    expand_glob_imports(item_map, module_map)

    # 2. apply qualified-path substitutions to lib + integration tests
    total = 0
    for root in (API_ROOT, API_TESTS_ROOT):
        if not root.exists():
            continue
        for rs in root.rglob("*.rs"):
            total += apply_to_file(rs, item_map, module_map)
    print(f"Applied {total} substitutions.")

    # 3. strip `pub use` blocks from mod.rs files
    for mod_path, _ in MODS:
        if mod_path.exists():
            rewrite_mod(mod_path)
    print("Stripped `pub use` from mod.rs files.")

    # Special case rbac.rs — strip the Role re-export there
    rbac = API_ROOT / "adapters/inbound/http/middleware/rbac.rs"
    if rbac.exists():
        text = rbac.read_text(encoding="utf-8")
        # Keep the comment but remove the pub use line
        new = re.sub(
            r"// Le type `Role`.*?\npub use crate::domain::enums::Role;\n",
            "",
            text,
            flags=re.DOTALL,
        )
        if new != text:
            rbac.write_text(new, encoding="utf-8")
            print("Cleaned middleware/rbac.rs Role re-export.")


if __name__ == "__main__":
    main()
