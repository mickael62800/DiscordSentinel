#!/usr/bin/env python3
"""Replace removed DomainError variants with their generic equivalents."""

import re
from pathlib import Path

ROOTS = [Path("services/api/src"), Path("services/api/tests")]

# Constructor replacements (in expression positions only — patterns left alone)
CTORS = [
    (re.compile(r"DomainError::RuleNotFound\(([^)]+)\)"),
     lambda m: m.group(0) if m.group(1).strip() == "_"
                else f'DomainError::NotFound(format!("Regle {{}}", {m.group(1)}))'),
    (re.compile(r"DomainError::InfractionNotFound\(([^)]+)\)"),
     lambda m: m.group(0) if m.group(1).strip() == "_"
                else f'DomainError::NotFound(format!("Infraction {{}}", {m.group(1)}))'),
    (re.compile(r"DomainError::TicketNotFound\(([^)]+)\)"),
     lambda m: m.group(0) if m.group(1).strip() == "_"
                else f'DomainError::NotFound(format!("Ticket {{}}", {m.group(1)}))'),
    (re.compile(r"DomainError::InvalidRule\(([^)]+)\)"),
     lambda m: m.group(0) if m.group(1).strip() == "_"
                else f'DomainError::ValidationError({m.group(1)})'),
]

# Match arm removal : `DomainError::XxxNotFound(_) | DomainError::YyyNotFound(_) | DomainError::NotFound(_)`
# becomes `DomainError::NotFound(_)`. Same for InvalidRule -> ValidationError.
# Easiest : compile a regex per variant for `DomainError::X(_)\s*\|\s*` and strip.
# We do a multi-pass : strip each variant's pattern with the trailing | (OR with leading | if last).
ARM_REMOVALS = [
    "DomainError::RuleNotFound",
    "DomainError::InfractionNotFound",
    "DomainError::TicketNotFound",
]

# InvalidRule is more involved : its arm maps to InvalidArgument, same as ValidationError.
# After removing InvalidRule, the arms collapse if they had the same target.
# Easiest : remove `DomainError::InvalidRule(_)` from any `|` chain. If it stands alone,
# the arm becomes invalid — but we know from grep there's no standalone arm.

ARM_REMOVALS.append("DomainError::InvalidRule")


def strip_arm(text: str, variant: str) -> str:
    # Pattern : variant(_) followed by `|` and whitespace -> remove
    text = re.sub(re.escape(variant) + r"\(_\)\s*\|\s*", "", text)
    # Pattern : `|` and whitespace then variant(_) -> remove
    text = re.sub(r"\|\s*" + re.escape(variant) + r"\(_\)", "", text)
    # Pattern : variant(_) standalone (followed by =>) -> remove the entire arm line
    text = re.sub(
        r"^\s*" + re.escape(variant) + r"\(_\)\s*=>[^,\n]*[,\n]?\n?",
        "",
        text,
        flags=re.MULTILINE,
    )
    return text


def main():
    for root in ROOTS:
        for rs in root.rglob("*.rs"):
            text = rs.read_text(encoding="utf-8")
            new = text
            for pat, repl_fn in CTORS:
                new = pat.sub(repl_fn, new)
            for variant in ARM_REMOVALS:
                new = strip_arm(new, variant)
            if new != text:
                rs.write_text(new, encoding="utf-8")
    print("done")


if __name__ == "__main__":
    main()
