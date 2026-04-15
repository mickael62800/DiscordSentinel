"""
Integre Jigsaw Toxic Comment (train, traduction FR) depuis HuggingFace
Intuit-GenSRF/jigsaw-toxic-comment-train-fr -> 223k commentaires Wikipedia FR,
dont ~34k toxiques multi-labels (toxic, profane, insult, hate, threat).

Mapping vers 5 classes Sentinel :
  threat                    -> threat (3)
  hate | insult             -> harassment (4)
  profane (sans insult/hate)-> rage (2)
  toxic seul                -> anger (1)
  aucun label               -> neutral (0, plafonne)

Sorties :
    training/text/datasets/toxic/train_jigsaw_fr.jsonl
    training/text/datasets/neutral/train_jigsaw_fr.txt
"""
from __future__ import annotations

import json
import random
import re
from collections import Counter
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
AI_ROOT = SCRIPT_DIR.parent
DATA_DIR = AI_ROOT / "training" / "text" / "datasets"
TOXIC_DIR = DATA_DIR / "toxic"
NEUTRAL_DIR = DATA_DIR / "neutral"

NEUTRAL_CAP = 10000
MAX_LEN = 400
MIN_LEN = 5

LABEL_NAMES = {0: "neutral", 1: "anger", 2: "rage", 3: "threat", 4: "harassment"}


def clean(t: str) -> str:
    t = re.sub(r"\[\[[^\]]*\]\]", " ", t)
    t = re.sub(r"https?://\S+", " ", t)
    t = re.sub(r"={2,}[^=]*={2,}", " ", t)
    t = re.sub(r"\s+", " ", t).strip()
    return t


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


def map_labels(labs: list[str]) -> int:
    s = set(labs)
    if "threat" in s:
        return 3
    if "hate" in s or "insult" in s:
        return 4
    if "profane" in s:
        return 2
    if "toxic" in s:
        return 1
    return 0


def main() -> None:
    from datasets import load_dataset

    print("Index existant...")
    seen = load_existing_keys()
    print(f"  {len(seen)} textes indexes\n")

    print("Telechargement Jigsaw FR...")
    ds = load_dataset("Intuit-GenSRF/jigsaw-toxic-comment-train-fr", split="train")
    print(f"  {len(ds)} lignes\n")

    toxic_out: list[dict] = []
    neutral_pool: list[str] = []
    stats = Counter()
    dupes = 0
    too_long = 0
    too_short = 0

    for row in ds:
        text = clean(row["text"] or "")
        if len(text) < MIN_LEN:
            too_short += 1
            continue
        if len(text) > MAX_LEN:
            too_long += 1
            continue
        key = norm_key(text)
        if key in seen:
            dupes += 1
            continue
        seen.add(key)
        label = map_labels(row["labels"])
        if label == 0:
            neutral_pool.append(text)
        else:
            toxic_out.append({"text": text, "label": label})
        stats[label] += 1

    # plafonner le neutre
    if len(neutral_pool) > NEUTRAL_CAP:
        random.seed(42)
        random.shuffle(neutral_pool)
        neutral_pool = neutral_pool[:NEUTRAL_CAP]

    print(f"Doublons                 : {dupes}")
    print(f"Trop courts              : {too_short}")
    print(f"Trop longs               : {too_long}")
    print(f"Toxic retenus            : {len(toxic_out)}")
    print(f"Neutral retenus (plafond): {len(neutral_pool)}")
    print("\nRepartition (avant plafond neutre) :")
    for lb in sorted(stats):
        print(f"  {LABEL_NAMES[lb]:12s}: {stats[lb]}")

    TOXIC_DIR.mkdir(parents=True, exist_ok=True)
    NEUTRAL_DIR.mkdir(parents=True, exist_ok=True)

    out_tox = TOXIC_DIR / "train_jigsaw_fr.jsonl"
    with out_tox.open("w", encoding="utf-8") as f:
        for e in toxic_out:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")
    print(f"\n  -> {out_tox}")

    out_neu = NEUTRAL_DIR / "train_jigsaw_fr.txt"
    out_neu.write_text("\n".join(neutral_pool), encoding="utf-8")
    print(f"  -> {out_neu}")


if __name__ == "__main__":
    main()
