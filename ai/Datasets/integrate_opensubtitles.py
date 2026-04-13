"""
Telecharge OpenSubtitles (paire fr-en) et extrait la partie FR comme corpus
neutre conversationnel (dialogues de films/series, registre oral court).

- Streame le dataset pour limiter la RAM
- Filtre : 20 <= len <= 200, pas d'indications musicales, pas de didascalies
- Dedup contre l'existant + ToxiFrench integre
- Plafond : 25k lignes (ne pas ecraser le signal)
- Ecrit : training/text/datasets/neutral/train_opensubtitles.txt
"""

import re
from pathlib import Path

from datasets import load_dataset

SCRIPT_DIR = Path(__file__).parent
AI_ROOT = SCRIPT_DIR.parent
DATA_DIR = AI_ROOT / "training" / "text" / "datasets"
NEUTRAL_DIR = DATA_DIR / "neutral"
TOXIC_DIR = DATA_DIR / "toxic"

CAP = 25000
MIN_LEN = 20
MAX_LEN = 200

# Sous-titres contiennent parfois ces patterns a rejeter
BAD_PATTERNS = re.compile(r"[♪♫]|<[^>]+>|^\[.*\]$|^\(.*\)$|www\.|\.com|\.org|subtitle|sync and corrections", re.I)


def norm_key(text: str) -> str:
    return re.sub(r"\s+", " ", text.strip().lower())


def load_existing_keys() -> set[str]:
    seen: set[str] = set()
    if NEUTRAL_DIR.exists():
        for p in NEUTRAL_DIR.glob("*.txt"):
            for line in p.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if line:
                    seen.add(norm_key(line))
    # Inutile de charger toxic pour le neutre, mais on le fait pour eviter
    # d'introduire un texte qui serait accidentellement le meme qu'un toxic.
    if TOXIC_DIR.exists():
        import json
        for p in TOXIC_DIR.glob("*.jsonl"):
            for line in p.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if not line:
                    continue
                try:
                    seen.add(norm_key(json.loads(line)["text"]))
                except Exception:
                    pass
    return seen


def main() -> None:
    print("Chargement index existant...")
    seen = load_existing_keys()
    print(f"  {len(seen)} textes indexes")

    print("\nStreaming opus-100 (en-fr)...")
    try:
        ds = load_dataset(
            "Helsinki-NLP/opus-100",
            "en-fr",
            split="train",
            streaming=True,
        )
    except Exception as e:
        print(f"Erreur load: {e}")
        return

    kept: list[str] = []
    seen_examined = 0

    for row in ds:
        if len(kept) >= CAP:
            break
        seen_examined += 1
        if seen_examined % 50000 == 0:
            print(f"  examine={seen_examined}  gardes={len(kept)}")
        trans = row.get("translation") or {}
        text = (trans.get("fr") or "").strip()
        if not text:
            continue
        text = re.sub(r"\s+", " ", text)
        if not (MIN_LEN <= len(text) <= MAX_LEN):
            continue
        if BAD_PATTERNS.search(text):
            continue
        # Retirer tirets de tour de parole en debut
        text = re.sub(r"^[-–—]\s*", "", text).strip()
        if len(text) < MIN_LEN:
            continue
        key = norm_key(text)
        if key in seen:
            continue
        seen.add(key)
        kept.append(text)

    print(f"\nTotal examine: {seen_examined}")
    print(f"Gardes       : {len(kept)}")

    NEUTRAL_DIR.mkdir(parents=True, exist_ok=True)
    out = NEUTRAL_DIR / "train_opensubtitles.txt"
    out.write_text("\n".join(kept), encoding="utf-8")
    print(f"Ecrit : {out}")


if __name__ == "__main__":
    main()
