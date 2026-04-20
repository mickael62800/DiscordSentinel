"""
Enrichit `consolidated.jsonl` avec des datasets EN traduits FR.

Cible principale : booster les classes sous-representees (rage, threat) en
pompant des donnees anglaises (Jigsaw, HateXplain, DynaHate) et en les
traduisant via Helsinki-NLP/opus-mt-tc-big-en-fr (local, gratuit, Apache 2.0).

Pipeline :
    HF datasets download -> filtre pertinence -> traduction batch -> mapping
    sentinel -> dedup vs consolidated.jsonl -> append au consolidated.jsonl

Usage :
    # Installation dependances (dans l'env ai)
    pip install datasets transformers sentencepiece

    # Dry run (pas de traduction, montre ce qui serait ajoute)
    python ai/Datasets/enrich_from_web.py --dry-run

    # Jigsaw Unintended Bias uniquement (le plus gros gain threat/rage)
    python ai/Datasets/enrich_from_web.py --sources jigsaw

    # Tout
    python ai/Datasets/enrich_from_web.py --sources jigsaw hatexplain dynahate

    # Avec GPU
    python ai/Datasets/enrich_from_web.py --sources jigsaw --device cuda

Options :
    --sources       Liste des sources (jigsaw, hatexplain, dynahate)
    --limit N       Max lignes par source (default: pas de limite)
    --dry-run       N'ecrit rien, affiche juste stats
    --device        cpu / cuda (default: cpu)
    --batch-size N  Batch traduction (default: 32)
    --model         Modele HF de traduction (default: Helsinki-NLP/opus-mt-tc-big-en-fr)
    --output        Fichier sortie (default: ai/Datasets/consolidated.jsonl — append)
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
CONSOLIDATED = SCRIPT_DIR / "consolidated.jsonl"

LABEL_NAMES = {0: "neutral", 1: "anger", 2: "rage", 3: "threat", 4: "harassment"}


def norm_key(text: str) -> str:
    return re.sub(r"\s+", " ", text.strip().lower())


def load_existing_keys() -> set[str]:
    """Hash les textes deja dans consolidated.jsonl pour eviter doublons."""
    keys: set[str] = set()
    if not CONSOLIDATED.exists():
        return keys
    with CONSOLIDATED.open(encoding="utf-8") as f:
        for line in f:
            try:
                obj = json.loads(line)
                if t := obj.get("text"):
                    keys.add(norm_key(t))
            except json.JSONDecodeError:
                continue
    return keys


# ── Filtres par source ──

def jigsaw_filter(ds):
    """Civil Comments (Jigsaw Unintended Bias). Seuils ajustes pour nourrir
    les classes sous-representees (rage, threat) et capper les classes deja
    saturees (anger, neutral).

    Seuils :
    - threat >= 0.3 (classe tres rare, on baisse le seuil)
    - severe >= 0.3 (rage, idem)
    - identity >= 0.5 -> harassment
    - insult >= 0.7 -> anger (seuil haut car deja 35k en banque)
    - toxicity < 0.1 -> neutral (seuil tres strict pour ne pas polluer)

    Caps :
    - anger cap 15k
    - neutral cap 3k
    - pas de cap sur threat/rage/harassment (on en veut plus)
    """
    caps = {0: 3000, 1: 15000}
    counts = {0: 0, 1: 0}
    for row in ds:
        text = (row.get("text") or row.get("comment_text") or "").strip()
        if not text or len(text) > 2000:
            continue
        toxicity = float(row.get("toxicity", 0) or 0)
        threat = float(row.get("threat", 0) or 0)
        severe = float(row.get("severe_toxicity", 0) or 0)
        insult = float(row.get("insult", 0) or 0)
        identity = float(row.get("identity_attack", 0) or 0)

        label = None
        if threat >= 0.3:
            label = 3  # threat — priorite absolue
        elif severe >= 0.2:
            label = 2  # rage (seuil bas car classe tres sous-representee)
        elif identity >= 0.5:
            label = 4  # harassment
        elif insult >= 0.7:
            label = 1  # anger (seuil haut)
        elif toxicity < 0.1:
            label = 0  # neutral (tres strict)
        else:
            continue

        # Cap sur classes saturees
        if label in caps:
            if counts[label] >= caps[label]:
                continue
            counts[label] += 1
        yield text, label


def hatexplain_filter(ds):
    """HateXplain : hatespeech -> harassment, offensive -> anger, normal -> neutral."""
    for row in ds:
        # Schema HateXplain : post_tokens (list) + annotators[0].label
        tokens = row.get("post_tokens") or []
        text = " ".join(tokens).strip() if tokens else (row.get("text") or "").strip()
        if not text or len(text) > 2000:
            continue
        annotators = row.get("annotators") or {}
        label = annotators.get("label") if isinstance(annotators, dict) else None
        if isinstance(label, list) and label:
            label = label[0]
        if label == 0 or label == "normal":
            yield text, 0
        elif label == 1 or label == "offensive":
            yield text, 1  # anger
        elif label == 2 or label == "hatespeech":
            yield text, 4  # harassment


def dynahate_filter(ds):
    """DynaHate : labels hate/nothate + target_ethnicity, etc.
    Heuristique : check 'label' + 'target' pour détecter threats.
    """
    for row in ds:
        text = (row.get("text") or "").strip()
        if not text or len(text) > 2000:
            continue
        label = str(row.get("label", "")).lower()
        target = str(row.get("target", "") or "").lower()
        hate_type = str(row.get("type", "") or "").lower()

        if label in ("hate", "1", "true"):
            if "threat" in hate_type or "threatening" in hate_type:
                yield text, 3  # threat
            elif "dehumanization" in hate_type or "animosity" in hate_type:
                yield text, 2  # rage
            else:
                yield text, 4  # harassment
        elif label in ("nothate", "0"):
            yield text, 0


SOURCES = {
    "jigsaw": {
        "hf_name": "google/civil_comments",
        "split": "train",
        "filter": jigsaw_filter,
        "description": "Jigsaw Unintended Bias (via Civil Comments, CC0)",
    },
    # HateXplain & DynaHate utilisent d'anciens dataset scripts non supportes
    # par datasets >= 4.x. On les skip sauf si l'user rollback vers datasets 2.x
    # ou utilise load_dataset(trust_remote_code=True) avec datasets <= 3.x.
    # Alternatives modernes proposees ci-dessous.
    "hatexplain": {
        "hf_name": "Paul/hatecheck-french",  # hatecheck FR deja multilingue
        "split": "test",
        "filter": hatexplain_filter,
        "description": "HateCheck French (CC-BY 4.0)",
    },
    "dynahate": {
        "hf_name": "aps/dynahate",
        "split": "train",
        "filter": dynahate_filter,
        "description": "DynaHate v0.2.3 (peut necessiter datasets 2.x)",
    },
}


# ── Traduction ──

class Translator:
    def __init__(self, model_name: str, device: str):
        from transformers import MarianMTModel, MarianTokenizer
        print(f"[TRANSLATE] Loading {model_name} on {device}...", file=sys.stderr)
        self.tokenizer = MarianTokenizer.from_pretrained(model_name)
        self.model = MarianMTModel.from_pretrained(model_name).to(device)
        self.device = device

    def translate_batch(self, texts: list[str]) -> list[str]:
        if not texts:
            return []
        import torch
        with torch.no_grad():
            inputs = self.tokenizer(texts, return_tensors="pt", padding=True,
                                    truncation=True, max_length=256).to(self.device)
            out = self.model.generate(**inputs, max_length=256, num_beams=2)
        return [self.tokenizer.decode(t, skip_special_tokens=True) for t in out]


# ── Main ──

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sources", nargs="+", choices=list(SOURCES.keys()),
                    default=["jigsaw"])
    ap.add_argument("--limit", type=int, default=0, help="0 = pas de limite")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--device", default="cpu", choices=["cpu", "cuda"])
    ap.add_argument("--batch-size", type=int, default=32)
    ap.add_argument("--model", default="Helsinki-NLP/opus-mt-tc-big-en-fr")
    ap.add_argument("--output", type=Path, default=CONSOLIDATED)
    args = ap.parse_args()

    try:
        from datasets import load_dataset
    except ImportError:
        print("ERROR: pip install datasets transformers sentencepiece torch",
              file=sys.stderr)
        sys.exit(1)

    existing_keys = load_existing_keys()
    print(f"[LOAD] {len(existing_keys):,} textes deja dans {CONSOLIDATED.name}",
          file=sys.stderr)

    # Load translator (skip en dry-run)
    translator = None
    if not args.dry_run:
        translator = Translator(args.model, args.device)

    new_rows: list[dict] = []
    stats: dict[str, Counter[int]] = {s: Counter() for s in args.sources}

    for source in args.sources:
        cfg = SOURCES[source]
        print(f"\n[DL] {source} : {cfg['description']}", file=sys.stderr)
        try:
            ds = load_dataset(cfg["hf_name"], split=cfg["split"], streaming=False)
        except Exception as e:
            print(f"[ERROR] {source}: {e}", file=sys.stderr)
            continue

        print(f"[DL] {source} : {len(ds):,} lignes téléchargées", file=sys.stderr)

        # Filtre + dedup + collect
        to_translate: list[tuple[str, int]] = []
        for i, (text, label) in enumerate(cfg["filter"](ds)):
            if args.limit and len(to_translate) >= args.limit:
                break
            en_key = norm_key(text)
            # On pourrait deja dedup en EN mais on prefere dedup apres traduction
            # car deux textes EN differents peuvent donner meme texte FR.
            to_translate.append((text, label))
            if (i + 1) % 10000 == 0:
                print(f"  [{source}] {i+1:,} lignes filtrees",
                      file=sys.stderr)

        print(f"[FILTER] {source} : {len(to_translate):,} candidats après filtre",
              file=sys.stderr)

        if args.dry_run:
            for _, lbl in to_translate:
                stats[source][lbl] += 1
            continue

        # Traduction par batch
        for i in range(0, len(to_translate), args.batch_size):
            batch = to_translate[i:i + args.batch_size]
            texts_en = [t for t, _ in batch]
            try:
                texts_fr = translator.translate_batch(texts_en)
            except Exception as e:
                print(f"[ERROR] batch translate: {e}", file=sys.stderr)
                continue
            for (_, label), fr in zip(batch, texts_fr):
                fr = fr.strip()
                if not fr or len(fr) < 2:
                    continue
                key = norm_key(fr)
                if key in existing_keys:
                    continue
                existing_keys.add(key)
                stats[source][label] += 1
                new_rows.append({
                    "text": fr,
                    "label": label,
                    "label_name": LABEL_NAMES[label],
                    "source": f"{source}_translated",
                    "language": "fr",
                })
            if (i // args.batch_size) % 50 == 0:
                print(f"  [{source}] traduit {i+len(batch):,}/{len(to_translate):,}",
                      file=sys.stderr)

    # Write (append)
    if not args.dry_run and new_rows:
        with args.output.open("a", encoding="utf-8") as f:
            for row in new_rows:
                f.write(json.dumps(row, ensure_ascii=False) + "\n")

    # Stats
    print("\n=== Ajouts ===", file=sys.stderr)
    total = 0
    for source, cnt in stats.items():
        subtotal = sum(cnt.values())
        total += subtotal
        dist = "  ".join(f"{LABEL_NAMES[l]}={cnt.get(l, 0)}" for l in sorted(LABEL_NAMES))
        print(f"  {source:<15} : +{subtotal:>7,}  |  {dist}", file=sys.stderr)

    mode = "simules" if args.dry_run else "ajoutes"
    print(f"\n[OK] {total:,} lignes {mode} dans {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
