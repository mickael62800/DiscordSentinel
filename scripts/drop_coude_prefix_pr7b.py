#!/usr/bin/env python3
"""PR7B — drop `Coude` prefix on the remaining proto-anchored types.

Renames the proto messages in coude.proto (NOT the services — renaming a
service path breaks gRPC wire compat with old clients) AND the corresponding
Rust types (domain entities + Grpc adapter wrappers).

Wire compat note : protobuf wire format is field-tag based, so renaming a
*message* type is wire-compatible. Renaming a *service* changes the gRPC
method path (`coude.v1.CoudePlayerService/X`) and breaks old clients — kept
out of scope here.
"""

import re
from pathlib import Path

# Proto messages + their domain/Rust counterparts.
RENAMES = {
    # Proto messages (bridged 1:1 with domain entities)
    "CoudePlayer":            "Player",
    "CoudeCombat":            "Combat",
    "CoudeBet":               "Bet",
    "CoudeInventoryItem":     "InventoryItem",
    "CoudePrime":             "Prime",
    "CoudeInsurance":         "Insurance",
    "CoudeStealProtection":   "StealProtection",
    "CoudeStealBoost":        "StealBoost",
    "CoudeEvent":             "Event",
    # CoudeCurrentSeason : KEPT — collision avec `rpc CurrentSeason(...)`
    # dans CoudeSocialService (renommer le rpc casserait la wire compat).
    "CoudeLeaderboardEntry":  "LeaderboardEntry",
    # Proto-only wrapper messages (no domain sibling — pure RPC envelopes)
    "CoudeCashboxState":      "CashboxState",
    "CoudeCatalogResponse":   "CatalogResponse",
    # Grpc adapter wrappers (Rust-only, in adapters/inbound/grpc/coude/)
    "CoudePlayerGrpc":        "PlayerGrpc",
    "CoudeCombatsGrpc":       "CombatsGrpc",
    "CoudeBetsGrpc":          "BetsGrpc",
    "CoudeEconomyGrpc":       "EconomyGrpc",
    "CoudeInventoryGrpc":     "InventoryGrpc",
    "CoudeSocialGrpc":        "SocialGrpc",
}

ORDERED = sorted(RENAMES.items(), key=lambda kv: -len(kv[0]))


def rewrite(text: str) -> str:
    out = text
    for old, new in ORDERED:
        out = re.sub(r"\b" + re.escape(old) + r"\b", new, out)
    return out


def main():
    targets = [
        Path("services/proto/proto/coude.proto"),  # proto messages
    ]
    rust_roots = [
        Path("services/api/src"),
        Path("services/api/tests"),
        Path("services/proto/src"),
        Path("bots/sentinel-bot/src/modules/coude"),
        Path("services/workers/coude-worker/src"),
        Path("bots/shared/src"),
    ]

    total = 0
    for f in targets:
        text = f.read_text(encoding="utf-8")
        new = rewrite(text)
        if new != text:
            f.write_text(new, encoding="utf-8")
            total += 1

    for root in rust_roots:
        if not root.exists():
            continue
        for rs in root.rglob("*.rs"):
            text = rs.read_text(encoding="utf-8")
            new = rewrite(text)
            if new != text:
                rs.write_text(new, encoding="utf-8")
                total += 1

    print(f"Patched {total} files (1 .proto + {total - 1} .rs).")


if __name__ == "__main__":
    main()
