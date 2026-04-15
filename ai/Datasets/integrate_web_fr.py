"""
Integre plusieurs datasets FR trouves sur HuggingFace / GitHub :

- MLMA hate speech (FR subset, ~4014 tweets)       -> rage/harassment/threat/anger
- JusteLeo/French-emotion (colere, neutre)          -> anger + neutral

Les doublons contre l'existant (toxic/*.jsonl + neutral/*.txt) sont ecartes.
Sorties :
    training/text/datasets/toxic/train_mlma_fr.jsonl
    training/text/datasets/toxic/train_french_emotion.jsonl
    training/text/datasets/neutral/train_french_emotion.txt

A lancer depuis ai/Datasets/ (ou n'importe ou) :
    python integrate_web_fr.py
"""
from __future__ import annotations

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

MLMA_CSV = SCRIPT_DIR / "mlma_fr" / "fr_dataset.csv"

LABEL_NAMES = {0: "neutral", 1: "anger", 2: "rage", 3: "threat", 4: "harassment"}


# ---------- util ----------

def clean_tweet(t: str) -> str:
    t = re.sub(r"@\w+", "", t)
    t = re.sub(r"https?://\S+", "", t)
    t = re.sub(r"#\w+", "", t)
    t = re.sub(r"rt\s+", "", t, flags=re.IGNORECASE)
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


def write_jsonl(path: Path, entries: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        for e in entries:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")


def write_lines(path: Path, lines: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")


# ---------- MLMA FR ----------

def map_mlma(sentiment: str, group: str) -> int | None:
    """
    MLMA : sentiment est un chapelet de tokens joints par '_'.
    Tokens possibles: normal, offensive, disrespectful, abusive, hateful, fearful.
    group : 'individual' = cible une personne -> harassment, sinon -> rage.
    """
    tokens = set(sentiment.split("_"))
    is_individual = group == "individual"

    if "fearful" in tokens:
        return 3  # threat (peur induite)

    if "abusive" in tokens or "hateful" in tokens:
        return 4 if is_individual else 2

    if "offensive" in tokens or "disrespectful" in tokens:
        return 4 if is_individual else 1

    return None  # pur 'normal' : on jette (le normal MLMA est bruite)


def integrate_mlma(seen: set[str]) -> list[dict]:
    if not MLMA_CSV.exists():
        print(f"[MLMA] skip, absent : {MLMA_CSV}")
        return []
    out: list[dict] = []
    dupes = skipped = 0
    with MLMA_CSV.open(encoding="utf-8", newline="") as f:
        for row in csv.DictReader(f):
            raw = (row.get("tweet") or "").strip()
            text = clean_tweet(raw)
            if not (5 <= len(text) <= 500):
                skipped += 1
                continue
            label = map_mlma(row.get("sentiment") or "", row.get("group") or "")
            if label is None:
                skipped += 1
                continue
            key = norm_key(text)
            if key in seen:
                dupes += 1
                continue
            seen.add(key)
            out.append({"text": text, "label": label})
    counts = Counter(e["label"] for e in out)
    print(f"[MLMA] kept={len(out)} dupes={dupes} skipped={skipped}")
    for lb in sorted(counts):
        print(f"       {LABEL_NAMES[lb]:12s}: {counts[lb]}")
    return out


# ---------- French-emotion ----------

def integrate_french_emotion(seen: set[str]) -> tuple[list[dict], list[str]]:
    try:
        from datasets import load_dataset
    except ImportError:
        print("[french-emotion] datasets non installe, skip")
        return [], []

    ds_all = load_dataset("JusteLeo/French-emotion")
    toxic_out: list[dict] = []
    neutral_out: list[str] = []
    dupes = 0
    for split in ds_all.keys():
        for row in ds_all[split]:
            text = re.sub(r"\s+", " ", (row["text"] or "").strip())
            if not (5 <= len(text) <= 500):
                continue
            key = norm_key(text)
            if key in seen:
                dupes += 1
                continue
            seen.add(key)
            lab = row["label"]
            if lab == "colere":
                toxic_out.append({"text": text, "label": 1})  # anger
            elif lab == "neutre":
                neutral_out.append(text)
    print(f"[french-emotion] anger={len(toxic_out)} neutral={len(neutral_out)} dupes={dupes}")
    return toxic_out, neutral_out


# ---------- main ----------

def main() -> None:
    print("Index existant...")
    seen = load_existing_keys()
    print(f"  {len(seen)} textes indexes\n")

    mlma = integrate_mlma(seen)
    fe_tox, fe_neu = integrate_french_emotion(seen)

    if mlma:
        write_jsonl(TOXIC_DIR / "train_mlma_fr.jsonl", mlma)
        print(f"  -> {TOXIC_DIR / 'train_mlma_fr.jsonl'}")
    if fe_tox:
        write_jsonl(TOXIC_DIR / "train_french_emotion.jsonl", fe_tox)
        print(f"  -> {TOXIC_DIR / 'train_french_emotion.jsonl'}")
    if fe_neu:
        write_lines(NEUTRAL_DIR / "train_french_emotion.txt", fe_neu)
        print(f"  -> {NEUTRAL_DIR / 'train_french_emotion.txt'}")


if __name__ == "__main__":
    main()
