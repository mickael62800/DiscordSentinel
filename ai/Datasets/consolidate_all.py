"""
Consolide TOUS les datasets texte en un seul fichier JSONL unifie.

Input : tous les sous-dossiers de ai/Datasets/* (formats heterogenes : CSV, JSON,
        JSONL, XLSX, TXT).
Output : ai/Datasets/consolidated.jsonl — une ligne par exemple avec schema :
    {
        "text": "...",
        "label": 0-4,
        "label_name": "neutral|anger|rage|threat|harassment",
        "source": "toxifrench|hate_superset|mlma_fr|...",
        "language": "fr"
    }

Deduplication par hash normalise (whitespace + lowercase).

Usage :
    python ai/Datasets/consolidate_all.py

Options :
    --output <path>   Chemin du fichier de sortie (default: consolidated.jsonl)
    --format jsonl|csv  Format de sortie (default: jsonl)
    --neutral-cap N   Plafonne le nombre de neutres par source (default: 30000)
    --stats           Affiche stats par classe et par source
"""
from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
DATASETS_ROOT = SCRIPT_DIR
OUTPUT_DEFAULT = SCRIPT_DIR / "consolidated.jsonl"

LABEL_NAMES = {0: "neutral", 1: "anger", 2: "rage", 3: "threat", 4: "harassment"}

# ── Parsers par source ──

def norm_key(text: str) -> str:
    """Cle de dedup : normalise whitespace + lowercase."""
    return re.sub(r"\s+", " ", text.strip().lower())


def parse_toxifrench(csv_path: Path):
    """ToxiFrench : parse S/H/V/R/A -> 5 classes."""
    label_re = re.compile(r"S(?P<S>\d)\s*/\s*H(?P<H>\d)\s*/\s*V(?P<V>\d)\s*/\s*R(?P<R>\d)\s*/\s*A(?P<A>\d)")

    def map_label(s):
        V, H, S_, R, A = s["V"], s["H"], s["S"], s["R"], s["A"]
        if V >= 2:
            return 3  # threat
        if H >= 2 or R >= 2 or S_ >= 2:
            return 4
        if V >= 1 and A >= 2:
            return 2
        if A >= 3:
            return 2
        if H >= 1 or S_ >= 1 or R >= 1:
            return 4
        if A >= 1:
            return 1
        return 0

    with csv_path.open(encoding="utf-8", errors="replace", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            text = (row.get("content") or "").strip()
            raw = row.get("CoT_labels") or row.get("literal_conclusion") or ""
            m = label_re.search(raw)
            if not text or not m:
                continue
            scores = {k: int(v) for k, v in m.groupdict().items()}
            yield text, map_label(scores)


def parse_french_hate_superset(csv_path: Path):
    """Binaire 0/1 -> 0 (neutral) / 4 (harassment)."""
    with csv_path.open(encoding="utf-8", errors="replace", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            text = (row.get("text") or "").strip()
            label = row.get("label", "0").strip()
            if not text:
                continue
            yield text, 4 if label in ("1", "true", "True") else 0


def parse_mlma_fr(csv_path: Path):
    """normal/abusive/hateful -> 0/4/4."""
    with csv_path.open(encoding="utf-8", errors="replace", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            text = (row.get("tweet") or "").strip()
            sentiment = (row.get("sentiment") or "").lower().strip()
            if not text:
                continue
            if sentiment in ("abusive", "hateful", "offensive"):
                yield text, 4
            elif sentiment == "normal":
                yield text, 0


def parse_textdetox_fr(csv_path: Path):
    """Binaire toxic 0/1 -> 0 (neutral) / 1 (anger)."""
    with csv_path.open(encoding="utf-8", errors="replace", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            text = (row.get("text") or "").strip()
            toxic = (row.get("toxic") or "0").strip()
            if not text:
                continue
            yield text, 1 if toxic == "1" else 0


def parse_hatecheck(csv_path: Path):
    """hateful/non-hateful -> 4/0."""
    with csv_path.open(encoding="utf-8", errors="replace", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            text = (row.get("test_case") or "").strip()
            gold = (row.get("label_gold") or "").lower().strip()
            if not text:
                continue
            yield text, 4 if gold == "hateful" else 0


def parse_cyberagression_ado(csv_path: Path):
    """HATE + VERBAL_ABUSE + ROLE → mapping heuristique."""
    with csv_path.open(encoding="utf-8", errors="replace", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            text = (row.get("TEXT") or "").strip()
            if not text:
                continue
            hate = (row.get("HATE") or "").upper()
            abuse = (row.get("VERBAL_ABUSE") or "").upper()
            if hate in ("OAG", "NAG") or "HATE" in hate:
                yield text, 4
            elif abuse in ("OAG", "NAG") or "ABUSE" in abuse:
                yield text, 1
            else:
                yield text, 0


def parse_existing_toxic_jsonl(jsonl_path: Path):
    """training/text/datasets/toxic/*.jsonl : {text, label}."""
    with jsonl_path.open(encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            text = (obj.get("text") or "").strip()
            label = obj.get("label")
            if not text or label is None:
                continue
            try:
                yield text, int(label)
            except (ValueError, TypeError):
                continue


def parse_neutral_txt(txt_path: Path):
    """training/text/datasets/neutral/*.txt : une phrase par ligne."""
    with txt_path.open(encoding="utf-8", errors="replace") as f:
        for line in f:
            text = line.strip()
            if text:
                yield text, 0


# ── Orchestration ──

SOURCES = [
    # (source_name, parser, glob pattern, relative to DATASETS_ROOT)
    ("toxifrench", parse_toxifrench, "toxifrench/toxifrench.csv"),
    ("hate_superset", parse_french_hate_superset, "french_hate_superset/*.csv"),
    ("mlma_fr", parse_mlma_fr, "mlma_fr/*.csv"),
    ("textdetox_fr", parse_textdetox_fr, "textdetox_fr/*.csv"),
    ("hatecheck_fr", parse_hatecheck, "hatecheck_french/*.csv"),
    ("cyberagression_ado", parse_cyberagression_ado, "cyberagression_ado/*.csv"),
]

# Datasets pre-traites dans training/text/datasets/
TRAINING_ROOT = DATASETS_ROOT.parent / "training" / "text" / "datasets"
PRETRAITE = [
    ("pretraite_toxic", parse_existing_toxic_jsonl, TRAINING_ROOT / "toxic" / "*.jsonl"),
    ("pretraite_neutral", parse_neutral_txt, TRAINING_ROOT / "neutral" / "*.txt"),
]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", type=Path, default=OUTPUT_DEFAULT)
    ap.add_argument("--format", choices=["jsonl", "csv"], default="jsonl")
    ap.add_argument("--neutral-cap", type=int, default=30000,
                    help="Cap neutres par source (default 30000, 0 = pas de cap)")
    ap.add_argument("--stats", action="store_true")
    args = ap.parse_args()

    seen: set[str] = set()
    per_source: dict[str, Counter[int]] = defaultdict(Counter)
    per_source_neutral: dict[str, int] = defaultdict(int)
    total = 0
    written = 0
    out_lines: list[dict] = []

    def process_source(name: str, parser, path_or_glob):
        nonlocal total, written
        if isinstance(path_or_glob, Path) and path_or_glob.is_file():
            paths = [path_or_glob]
        else:
            # glob pattern (str) vs Path-with-glob
            if isinstance(path_or_glob, Path):
                parent = path_or_glob.parent
                pattern = path_or_glob.name
            else:
                pattern_full = DATASETS_ROOT / path_or_glob
                parent = pattern_full.parent
                pattern = pattern_full.name
            paths = sorted(parent.glob(pattern)) if parent.exists() else []

        for p in paths:
            if not p.exists():
                continue
            try:
                for text, label in parser(p):
                    total += 1
                    if label not in LABEL_NAMES:
                        continue
                    if len(text) < 2 or len(text) > 2000:
                        continue
                    # Cap neutres par source
                    if label == 0 and args.neutral_cap > 0:
                        if per_source_neutral[name] >= args.neutral_cap:
                            continue
                        per_source_neutral[name] += 1
                    key = norm_key(text)
                    if key in seen:
                        continue
                    seen.add(key)
                    per_source[name][label] += 1
                    out_lines.append({
                        "text": text,
                        "label": label,
                        "label_name": LABEL_NAMES[label],
                        "source": name,
                        "language": "fr",
                    })
                    written += 1
            except Exception as e:
                print(f"[WARN] {name} {p.name}: {e}", file=sys.stderr)

    for name, parser, glob in SOURCES:
        process_source(name, parser, glob)
    for name, parser, glob in PRETRAITE:
        process_source(name, parser, glob)

    # Write
    args.output.parent.mkdir(parents=True, exist_ok=True)
    if args.format == "jsonl":
        with args.output.open("w", encoding="utf-8") as f:
            for row in out_lines:
                f.write(json.dumps(row, ensure_ascii=False) + "\n")
    else:
        csv_path = args.output.with_suffix(".csv")
        with csv_path.open("w", encoding="utf-8", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=["text", "label", "label_name", "source", "language"])
            writer.writeheader()
            writer.writerows(out_lines)
        args.output = csv_path

    print(f"[OK] Consolidation : {written} lignes écrites dans {args.output}")
    print(f"     (total parsé : {total}, dedup/filtrés : {total - written})")

    if args.stats:
        print("\n=== Stats par classe ===")
        overall = Counter()
        for src, cnt in per_source.items():
            for lbl, n in cnt.items():
                overall[lbl] += n
        for lbl in sorted(overall):
            print(f"  {lbl} {LABEL_NAMES[lbl]:<12} : {overall[lbl]:>10,}")

        print("\n=== Stats par source ===")
        for src in sorted(per_source):
            total_src = sum(per_source[src].values())
            dist = "  ".join(
                f"{LABEL_NAMES[l]}={per_source[src].get(l, 0)}" for l in sorted(LABEL_NAMES)
            )
            print(f"  {src:<25} : total={total_src:>8,}  |  {dist}")


if __name__ == "__main__":
    main()
