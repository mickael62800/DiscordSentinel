"""
Integre les 3 nouveaux datasets dans le format Sentinel (5 classes).
Mapping : 0=neutral, 1=anger, 2=rage, 3=threat, 4=harassment

Sources :
- HateCheck-French (3718 samples, 27 sous-categories)
- CyberAgressionAdo-Large (5608 messages, labels OAG/CAG/NAG + verbal_abuse)
- TextDetox Multilingual FR (5000 samples, binaire toxic/non-toxic)
"""

import csv
import json
import re
from pathlib import Path
from collections import Counter

SCRIPT_DIR = Path(__file__).parent
OUTPUT_DIR = SCRIPT_DIR.parent / "training" / "text" / "datasets_clean"

LABEL_NAMES = {0: "neutral", 1: "anger", 2: "rage", 3: "threat", 4: "harassment"}

# Mots-cles pour reclassifier les samples binaires
THREAT_KW = [
    "tuer", "tue ", "mort", "crever", "buter", "bute",
    "defoncer", "frapper", "tabasser", "egorger",
    "exploser", "massacrer", "exterminer", "bruler",
    "couteau", "arme ", "fusil", "balle ",
    "te retrouver", "je vais te", "on va te",
    "casser la gueule", "peter la gueule",
    "te planter", "te crever", "te tuer", "te buter",
    "pendaison", "pendre", "noyer", "violer",
]

RAGE_KW = [
    "nique", "enculer", "fils de pute", "fdp", "ntm",
    "ta mere", "bande de", "tous des", "race de",
    "sous-race", "sous race", "sale race",
    "tous crever", "tous mourir",
]

HARASSMENT_KW = [
    "degage", "casse-toi", "casse toi", "tire-toi",
    "personne t'aime", "tout le monde te",
    "t'es qu'un", "t'es qu'une", "t'es rien",
    "ferme ta gueule", "ferme-la", "ta gueule",
    "retourne chez", "rentre chez",
    "on veut pas de toi",
]


def has_kw(text: str, keywords: list[str]) -> bool:
    low = text.lower()
    # Supprimer accents simples pour matching
    low = low.replace("é", "e").replace("è", "e").replace("ê", "e")
    low = low.replace("à", "a").replace("â", "a")
    low = low.replace("ù", "u").replace("û", "u")
    low = low.replace("ô", "o").replace("î", "i")
    return any(k in low for k in keywords)


def classify_by_keywords(text: str) -> int:
    """Classifie un texte toxique binaire en anger/rage/threat/harassment."""
    if has_kw(text, THREAT_KW):
        return 3
    if has_kw(text, RAGE_KW):
        return 2
    if has_kw(text, HARASSMENT_KW):
        return 4
    return 1  # anger par defaut


# ── HateCheck-French ──

def process_hatecheck() -> tuple[list[dict], list[str]]:
    """Retourne (toxic_samples, neutral_samples)."""
    path = SCRIPT_DIR / "hatecheck_french" / "test_raw.csv"
    if not path.exists():
        print("  HateCheck: fichier introuvable, skip")
        return [], []

    toxic = []
    neutral = []

    # Mapping functionality -> label Sentinel
    threat_funcs = {"threat_dir_h", "threat_norm_h"}
    derog_funcs = {"derog_neg_emote_h", "derog_neg_attrib_h", "derog_dehum_h", "derog_impl_h"}
    slur_funcs = {"slur_h", "profanity_h"}
    # Les spell variants gardent le meme mapping que leur categorie de base
    spell_funcs = {"spell_space_del_h", "spell_space_add_h", "spell_leet_h",
                   "spell_char_swap_h", "spell_char_del_h"}

    with open(path, encoding="utf-8") as f:
        for row in csv.DictReader(f):
            text = row["test_case"].strip()
            if not text or len(text) < 5:
                continue

            label_gold = row["label_gold"]
            func = row["functionality"]

            if label_gold == "non-hateful":
                neutral.append(text)
                continue

            # hateful -> classifier selon functionality
            if func in threat_funcs:
                toxic.append({"text": text, "label": 3})
            elif func in derog_funcs:
                toxic.append({"text": text, "label": 4})  # harassment (denigrement)
            elif func in slur_funcs:
                toxic.append({"text": text, "label": 1})  # anger (insultes)
            elif func in spell_funcs:
                # Les variantes orthographiques: classifier par keywords
                label = classify_by_keywords(text)
                toxic.append({"text": text, "label": label})
            elif "negate_pos" in func or "phrase_opinion" in func:
                toxic.append({"text": text, "label": 4})  # harassment
            elif "ref_subs" in func:
                toxic.append({"text": text, "label": 4})  # harassment
            else:
                # Fallback: classifier par keywords
                label = classify_by_keywords(text)
                toxic.append({"text": text, "label": label})

    return toxic, neutral


# ── CyberAgressionAdo-Large ──

def process_cyberagression() -> tuple[list[dict], list[str]]:
    """Retourne (toxic_samples, neutral_samples)."""
    path = SCRIPT_DIR / "cyberagression_ado" / "cyberagression_ado_large_majority_vote.csv"
    if not path.exists():
        print("  CyberAgression: fichier introuvable, skip")
        return [], []

    toxic = []
    neutral = []

    with open(path, encoding="utf-8") as f:
        for row in csv.DictReader(f):
            text = row.get("TEXT", "").strip()
            if not text or len(text) < 5:
                continue

            # Nettoyer les balises type <nom>
            text = re.sub(r"<[^>]+>", "", text).strip()
            if len(text) < 5:
                continue

            hate = row.get("HATE", "").strip()
            verbal_abuse = row.get("VERBAL_ABUSE", "").strip()
            sentiment = row.get("SENTIMENT", "").strip()

            if hate == "NAG":
                neutral.append(text)
                continue

            # OAG (overtement agressif) ou CAG (couvertement agressif)
            if verbal_abuse == "THR":
                toxic.append({"text": text, "label": 3})  # threat
            elif hate == "OAG" and sentiment == "NEG":
                # OAG + negatif fort
                if has_kw(text, RAGE_KW):
                    toxic.append({"text": text, "label": 2})  # rage
                elif verbal_abuse == "DNG":
                    toxic.append({"text": text, "label": 4})  # harassment (denigrement)
                elif verbal_abuse == "NCG":
                    toxic.append({"text": text, "label": 1})  # anger (name-calling)
                else:
                    toxic.append({"text": text, "label": 1})  # anger par defaut
            elif hate == "CAG":
                toxic.append({"text": text, "label": 4})  # harassment (agressivite couverte)
            else:
                toxic.append({"text": text, "label": 1})  # anger

    return toxic, neutral


# ── TextDetox Multilingual FR ──

def process_textdetox() -> tuple[list[dict], list[str]]:
    """Retourne (toxic_samples, neutral_samples)."""
    path = SCRIPT_DIR / "textdetox_fr" / "textdetox_fr.csv"
    if not path.exists():
        print("  TextDetox: fichier introuvable, skip")
        return [], []

    toxic = []
    neutral = []

    with open(path, encoding="utf-8") as f:
        for row in csv.DictReader(f):
            text = row.get("text", "").strip()
            if not text or len(text) < 10:
                continue

            is_toxic = row.get("toxic", "0") == "1"

            if not is_toxic:
                neutral.append(text)
                continue

            # Toxic binaire -> reclassifier par keywords
            label = classify_by_keywords(text)
            toxic.append({"text": text, "label": label})

    return toxic, neutral


# ── Main ──

def main():
    all_toxic = []
    all_neutral = []

    print("=== HateCheck-French ===")
    t, n = process_hatecheck()
    print(f"  Toxic: {len(t)}, Neutral: {len(n)}")
    all_toxic.extend(t)
    all_neutral.extend(n)

    print("\n=== CyberAgressionAdo-Large ===")
    t, n = process_cyberagression()
    print(f"  Toxic: {len(t)}, Neutral: {len(n)}")
    all_toxic.extend(t)
    all_neutral.extend(n)

    print("\n=== TextDetox FR ===")
    t, n = process_textdetox()
    print(f"  Toxic: {len(t)}, Neutral: {len(n)}")
    all_toxic.extend(t)
    all_neutral.extend(n)

    # Stats
    toxic_counts = Counter(s["label"] for s in all_toxic)
    print(f"\n{'='*60}")
    print("NOUVEAUX SAMPLES A INTEGRER")
    print(f"{'='*60}")
    total = len(all_toxic) + len(all_neutral)
    print(f"  neutral     : {len(all_neutral)}")
    for label in sorted(toxic_counts.keys()):
        print(f"  {LABEL_NAMES[label]:12s}: {toxic_counts[label]}")
    print(f"  TOTAL       : {total}")

    # Charger le dataset clean existant et merger
    print(f"\n=== Fusion avec dataset_clean existant ===")

    existing_toxic = []
    existing_neutral = []

    for jsonl in OUTPUT_DIR.glob("toxic/*.jsonl"):
        for line in jsonl.read_text(encoding="utf-8").splitlines():
            if line.strip():
                try:
                    existing_toxic.append(json.loads(line))
                except json.JSONDecodeError:
                    pass

    for txt in OUTPUT_DIR.glob("neutral/*.txt"):
        for line in txt.read_text(encoding="utf-8").splitlines():
            if line.strip():
                existing_neutral.append(line.strip())

    print(f"  Existant toxic: {len(existing_toxic)}")
    print(f"  Existant neutral: {len(existing_neutral)}")

    # Deduplication par texte
    seen_texts = set()
    for s in existing_toxic:
        seen_texts.add(s["text"].strip().lower())
    for s in existing_neutral:
        seen_texts.add(s.strip().lower())

    new_toxic_dedup = []
    new_neutral_dedup = []
    dupes = 0

    for s in all_toxic:
        key = s["text"].strip().lower()
        if key not in seen_texts:
            new_toxic_dedup.append(s)
            seen_texts.add(key)
        else:
            dupes += 1

    for s in all_neutral:
        key = s.strip().lower()
        if key not in seen_texts:
            new_neutral_dedup.append(s)
            seen_texts.add(key)
        else:
            dupes += 1

    print(f"  Doublons supprimes: {dupes}")
    print(f"  Nouveaux toxic uniques: {len(new_toxic_dedup)}")
    print(f"  Nouveaux neutral uniques: {len(new_neutral_dedup)}")

    # Merger
    merged_toxic = existing_toxic + new_toxic_dedup
    merged_neutral = existing_neutral + new_neutral_dedup

    # Ecrire les fichiers fusionnes
    import random
    random.seed(42)

    random.shuffle(merged_toxic)
    split = int(len(merged_toxic) * 0.9)
    train_toxic = merged_toxic[:split]
    test_toxic = merged_toxic[split:]

    random.shuffle(merged_neutral)
    split = int(len(merged_neutral) * 0.9)
    train_neutral = merged_neutral[:split]
    test_neutral = merged_neutral[split:]

    (OUTPUT_DIR / "toxic").mkdir(parents=True, exist_ok=True)
    (OUTPUT_DIR / "neutral").mkdir(parents=True, exist_ok=True)

    with open(OUTPUT_DIR / "toxic" / "train.jsonl", "w", encoding="utf-8") as f:
        for s in train_toxic:
            f.write(json.dumps(s, ensure_ascii=False) + "\n")

    with open(OUTPUT_DIR / "toxic" / "test.jsonl", "w", encoding="utf-8") as f:
        for s in test_toxic:
            f.write(json.dumps(s, ensure_ascii=False) + "\n")

    with open(OUTPUT_DIR / "neutral" / "train.txt", "w", encoding="utf-8") as f:
        f.write("\n".join(train_neutral))

    with open(OUTPUT_DIR / "neutral" / "test.txt", "w", encoding="utf-8") as f:
        f.write("\n".join(test_neutral))

    # Stats finales
    final_counts = Counter(s["label"] for s in merged_toxic)
    final_total = len(merged_toxic) + len(merged_neutral)

    print(f"\n{'='*60}")
    print("DATASET FINAL")
    print(f"{'='*60}")
    print(f"  {'neutral':12s}: {len(merged_neutral):>6d}  ({len(merged_neutral)/final_total*100:5.1f}%)")
    for label in sorted(final_counts.keys()):
        print(f"  {LABEL_NAMES[label]:12s}: {final_counts[label]:>6d}  ({final_counts[label]/final_total*100:5.1f}%)")
    print(f"  {'TOTAL':12s}: {final_total:>6d}")
    print(f"\n  Train toxic : {len(train_toxic)}")
    print(f"  Test toxic  : {len(test_toxic)}")
    print(f"  Train neutral: {len(train_neutral)}")
    print(f"  Test neutral : {len(test_neutral)}")
    print(f"\n  Ecrit dans: {OUTPUT_DIR}")


if __name__ == "__main__":
    main()
