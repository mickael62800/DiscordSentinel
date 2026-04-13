"""
Integre ToxiFrench dans training/text/datasets/ (dir effectivement lu par le trainer).

- Parse les labels S/H/V/R/A pour mapper vers 5 classes
- Dedup contre les fichiers toxic/*.jsonl + neutral/*.txt existants
- Plafonne le neutre ToxiFrench pour ne pas ecraser le signal toxique
- Ecrit :
    training/text/datasets/toxic/train_toxifrench.jsonl
    training/text/datasets/neutral/train_toxifrench.txt
"""

import csv
import json
import re
from collections import Counter
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
AI_ROOT = SCRIPT_DIR.parent
DATA_DIR = AI_ROOT / "training" / "text" / "datasets"
TOXIC_DIR = DATA_DIR / "toxic"
NEUTRAL_DIR = DATA_DIR / "neutral"

TOXIFRENCH_CSV = SCRIPT_DIR / "toxifrench" / "toxifrench.csv"

# Plafond sur le neutre ToxiFrench (domaine JVC) pour rester equilibre.
NEUTRAL_CAP = 30000

LABEL_NAMES = {0: "neutral", 1: "anger", 2: "rage", 3: "threat", 4: "harassment"}

LABEL_RE = re.compile(
    r"S(?P<S>\d)\s*/\s*H(?P<H>\d)\s*/\s*V(?P<V>\d)\s*/\s*R(?P<R>\d)\s*/\s*A(?P<A>\d)"
)


def parse_scores(raw: str) -> dict | None:
    if not raw:
        return None
    m = LABEL_RE.search(raw)
    return {k: int(v) for k, v in m.groupdict().items()} if m else None


def map_to_sentinel(s: dict) -> int:
    V, H, S_, R, A = s["V"], s["H"], s["S"], s["R"], s["A"]
    if V >= 2:
        return 3  # threat
    if H >= 2 or R >= 2 or S_ >= 2:
        return 4  # harassment (haine/religion/sexuel marques)
    if V >= 1 and A >= 2:
        return 2  # rage
    if A >= 3:
        return 2  # rage
    if H >= 1 or S_ >= 1 or R >= 1:
        return 4  # harassment leger
    return 1  # anger par defaut


def norm_key(text: str) -> str:
    return re.sub(r"\s+", " ", text.strip().lower())


def load_existing_keys() -> set[str]:
    seen: set[str] = set()
    if TOXIC_DIR.exists():
        for p in TOXIC_DIR.glob("*.jsonl"):
            for line in p.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if not line:
                    continue
                try:
                    seen.add(norm_key(json.loads(line)["text"]))
                except (json.JSONDecodeError, KeyError):
                    pass
    if NEUTRAL_DIR.exists():
        for p in NEUTRAL_DIR.glob("*.txt"):
            for line in p.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if line:
                    seen.add(norm_key(line))
    return seen


def main() -> None:
    if not TOXIFRENCH_CSV.exists():
        raise SystemExit(f"Fichier absent : {TOXIFRENCH_CSV}")

    print(f"Chargement index existant depuis {DATA_DIR}...")
    seen = load_existing_keys()
    print(f"  {len(seen)} textes existants indexes")

    csv.field_size_limit(10_000_000)

    toxic_new: list[dict] = []
    neutral_new: list[str] = []
    dupes = 0
    skipped_len = 0
    skipped_parse = 0

    with open(TOXIFRENCH_CSV, encoding="utf-8", newline="") as f:
        for row in csv.DictReader(f):
            text = (row.get("content") or "").strip()
            if not text:
                continue
            text = re.sub(r"\s+", " ", text).strip()
            if not (5 <= len(text) <= 500):
                skipped_len += 1
                continue

            key = norm_key(text)
            if key in seen:
                dupes += 1
                continue
            seen.add(key)

            conclusion = (row.get("literal_conclusion_annotator") or "").strip().lower()
            if conclusion == "non":
                neutral_new.append(text)
                continue
            if conclusion != "oui":
                continue

            scores = parse_scores(row.get("CoT_labels") or "")
            if scores is None:
                skipped_parse += 1
                label = 1  # anger par defaut
            else:
                label = map_to_sentinel(scores)
            toxic_new.append({"text": text, "label": label})

    print(f"\nBruts extraits:")
    print(f"  toxic  : {len(toxic_new)}")
    print(f"  neutral: {len(neutral_new)}")
    print(f"  doublons ecartes    : {dupes}")
    print(f"  longueur hors bornes: {skipped_len}")
    print(f"  parse CoT echec     : {skipped_parse}")

    # Plafonner le neutre
    if len(neutral_new) > NEUTRAL_CAP:
        import random
        random.seed(42)
        random.shuffle(neutral_new)
        neutral_new = neutral_new[:NEUTRAL_CAP]
        print(f"  neutral plafonne a : {NEUTRAL_CAP}")

    # Stats toxic par classe
    counts = Counter(s["label"] for s in toxic_new)
    print(f"\nRepartition toxic ToxiFrench:")
    for lb in sorted(counts):
        print(f"  {LABEL_NAMES[lb]:12s}: {counts[lb]}")

    # Ecriture
    TOXIC_DIR.mkdir(parents=True, exist_ok=True)
    NEUTRAL_DIR.mkdir(parents=True, exist_ok=True)

    toxic_out = TOXIC_DIR / "train_toxifrench.jsonl"
    with open(toxic_out, "w", encoding="utf-8") as f:
        for s in toxic_new:
            f.write(json.dumps(s, ensure_ascii=False) + "\n")

    neutral_out = NEUTRAL_DIR / "train_toxifrench.txt"
    with open(neutral_out, "w", encoding="utf-8") as f:
        f.write("\n".join(neutral_new))

    print(f"\nEcrit:")
    print(f"  {toxic_out} ({len(toxic_new)} lignes)")
    print(f"  {neutral_out} ({len(neutral_new)} lignes)")


if __name__ == "__main__":
    main()
