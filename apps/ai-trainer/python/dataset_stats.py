"""
Stats dataset CLI — scanne les dossiers text/ et vision/ et ecrit le resultat
en JSON unique sur stdout (pas de JSONL, pas d'events intermediaires).
"""

import argparse
import io
import json
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

if sys.stdout.encoding != "utf-8":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")

TEXT_EXTS = {".jsonl", ".txt"}
IMAGE_EXTS = {".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"}
LABEL_NAMES = {0: "safe", 1: "severe"}
# Projette les labels bruts du dataset (toxifrench 5 classes) sur 2 classes binaires.
_LABEL_REMAP = {0: 0, 1: 0, 2: 1, 3: 1, 4: 0}


def scan_text(root: Path) -> dict:
    labels: Counter[str] = Counter()
    last_mod: float | None = None

    neutral_dir = root / "neutral"
    if neutral_dir.exists():
        for f in neutral_dir.iterdir():
            if f.suffix == ".txt":
                labels["neutral"] += sum(1 for line in f.read_text(encoding="utf-8").splitlines() if line.strip())
                mod = f.stat().st_mtime
                if last_mod is None or mod > last_mod:
                    last_mod = mod

    toxic_dir = root / "toxic"
    if toxic_dir.exists():
        for f in toxic_dir.iterdir():
            if f.suffix not in TEXT_EXTS:
                continue
            if f.suffix == ".jsonl":
                for line in f.read_text(encoding="utf-8").splitlines():
                    if line.strip():
                        try:
                            entry = json.loads(line)
                        except json.JSONDecodeError:
                            continue
                        label_id = entry.get("label", 1)
                        mapped = _LABEL_REMAP.get(label_id, label_id)
                        labels[LABEL_NAMES.get(mapped, str(mapped))] += 1
            elif f.suffix == ".txt":
                labels["anger"] += sum(1 for line in f.read_text(encoding="utf-8").splitlines() if line.strip())
            mod = f.stat().st_mtime
            if last_mod is None or mod > last_mod:
                last_mod = mod

    return {
        "model_type": "text-sentiment",
        "total_samples": sum(labels.values()),
        "label_distribution": dict(labels),
        "last_updated": datetime.fromtimestamp(last_mod, tz=timezone.utc).isoformat() if last_mod else None,
    }


def scan_vision(root: Path) -> dict:
    labels: Counter[str] = Counter()
    last_mod: float | None = None

    if root.exists():
        for class_dir in root.iterdir():
            if class_dir.is_dir():
                for f in class_dir.iterdir():
                    if f.suffix.lower() in IMAGE_EXTS:
                        labels[class_dir.name] += 1
                        mod = f.stat().st_mtime
                        if last_mod is None or mod > last_mod:
                            last_mod = mod

    return {
        "model_type": "image-classification",
        "total_samples": sum(labels.values()),
        "label_distribution": dict(labels),
        "last_updated": datetime.fromtimestamp(last_mod, tz=timezone.utc).isoformat() if last_mod else None,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--data-root", required=True)
    args = parser.parse_args()

    data_root = Path(args.data_root).resolve()
    data_root.mkdir(parents=True, exist_ok=True)

    result = [
        scan_text(data_root / "text" / "datasets"),
        scan_vision(data_root / "vision" / "datasets"),
    ]
    sys.stdout.write(json.dumps(result, ensure_ascii=False))
    sys.stdout.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main())
