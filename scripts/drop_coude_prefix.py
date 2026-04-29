#!/usr/bin/env python3
"""PR7 — Drop the `Coude` prefix on Rust types where it doesn't conflict with
proto-generated names. Repositories (port traits + Pg* concrete impls), enums,
configs, and types that have no proto equivalent are renamed. Domain entities
that overlap with proto messages (CoudePlayer, CoudeCombat, etc.) are KEPT to
avoid breaking the proto bridge — they'll be renamed in a follow-up PR after
the proto-side rename.
"""

import re
from pathlib import Path

# Symbols renamed by this PR. Order matters: longest first to avoid partial
# overlap (e.g. CoudePlayerRepository before CoudePlayer would have been a
# bug if we renamed CoudePlayer too — but we don't).
RENAMES = {
    # Repositories (port traits + Pg* concrete impls). Renamed in pairs so that
    # `impl XxxRepository for PgXxxRepository` stays consistent.
    "CoudeBetRepository":              "BetRepository",
    "CoudeBountyRepository":           "BountyRepository",
    "CoudeCashboxRepository":          "CashboxRepository",
    "CoudeCoalitionRepository":        "CoalitionRepository",
    "CoudeCombatRepository":           "CombatRepository",
    "CoudeCursesRepository":           "CursesRepository",
    "CoudeEconomyRepository":          "EconomyRepository",
    "CoudeFlavorTemplatesRepository":  "FlavorTemplatesRepository",
    "CoudeHeistRepository":            "HeistRepository",
    "CoudeInventoryRepository":        "InventoryRepository",
    "CoudePlayerRepository":           "PlayerRepository",
    "CoudeRefusalCountRepository":     "RefusalCountRepository",
    "CoudeSafetyNetRepository":        "SafetyNetRepository",
    "CoudeSocialRepository":           "SocialRepository",
    "CoudeStealBoostRepository":       "StealBoostRepository",
    "CoudeStealProtectionRepository":  "StealProtectionRepository",
    "CoudeTauntsRepository":           "TauntsRepository",
    "CoudeToutOuRienRepository":       "ToutOuRienRepository",
    "CoudeUltimateRepository":         "UltimateRepository",
    "CoudeVendettaRepository":         "VendettaRepository",
    # Pg* concrete impls
    "PgCoudeBetRepository":             "PgBetRepository",
    "PgCoudeBountyRepository":          "PgBountyRepository",
    "PgCoudeCashboxRepository":         "PgCashboxRepository",
    "PgCoudeCoalitionRepository":       "PgCoalitionRepository",
    "PgCoudeCombatRepository":          "PgCombatRepository",
    "PgCoudeCursesRepository":          "PgCursesRepository",
    "PgCoudeEconomyRepository":         "PgEconomyRepository",
    "PgCoudeFlavorTemplatesRepository": "PgFlavorTemplatesRepository",
    "PgCoudeHeistRepository":           "PgHeistRepository",
    "PgCoudeInventoryRepository":       "PgInventoryRepository",
    "PgCoudePlayerRepository":          "PgPlayerRepository",
    "PgCoudeRefusalCountRepository":    "PgRefusalCountRepository",
    "PgCoudeSafetyNetRepository":       "PgSafetyNetRepository",
    "PgCoudeSocialRepository":          "PgSocialRepository",
    "PgCoudeStealBoostRepository":      "PgStealBoostRepository",
    "PgCoudeStealProtectionRepository": "PgStealProtectionRepository",
    "PgCoudeTauntsRepository":          "PgTauntsRepository",
    "PgCoudeToutOuRienRepository":      "PgToutOuRienRepository",
    "PgCoudeUltimateRepository":        "PgUltimateRepository",
    "PgCoudeVendettaRepository":        "PgVendettaRepository",
    # Configs / enums / non-proto types (safe renames — don't conflict with proto)
    "CoudeBalanceParams":     "BalanceParams",
    "CoudeCashbox":           "Cashbox",          # proto has CoudeCashboxState, different
    "CoudeCatalog":           "Catalog",          # proto has CoudeCatalogResponse, different
    "CoudeClass":             "PlayerClass",      # `Class` alone is reserved-ish; PlayerClass is clearer
    "CoudeConfig":            "Config",
    # Skipped (proto conflict — message exists in proto/coude.proto with same name):
    #   CoudeEvent, CoudeCurrentSeason, CoudeLeaderboardEntry
    # Will be renamed in a follow-up PR after the proto-side rename.
    "CoudeGuildSettings":     "GuildSettings",
    "CoudeHeistAttempt":      "HeistAttempt",
    "CoudePrisonState":       "PrisonState",
    "CoudeTauntsConfig":      "TauntsConfig",
}

# Order longest-first to avoid partial-substring substitutions.
ORDERED = sorted(RENAMES.items(), key=lambda kv: -len(kv[0]))


def rewrite(text: str) -> str:
    out = text
    for old, new in ORDERED:
        out = re.sub(r"\b" + re.escape(old) + r"\b", new, out)
    return out


def main():
    roots = [
        Path("services/api/src"),
        Path("services/api/tests"),
        Path("bots/sentinel-bot/src/modules/coude"),
        Path("services/workers/coude-worker/src"),
        Path("bots/shared/src"),
    ]
    total = 0
    for root in roots:
        if not root.exists():
            continue
        for rs in root.rglob("*.rs"):
            text = rs.read_text(encoding="utf-8")
            new = rewrite(text)
            if new != text:
                rs.write_text(new, encoding="utf-8")
                total += 1
    print(f"Patched {total} files.")


if __name__ == "__main__":
    main()
