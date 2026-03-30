"""
Nettoyage et reequilibrage du dataset text Sentinel.

Problemes corriges :
1. Neutres pollues (mots toxiques classes neutres) -> reclasses ou supprimes
2. 85% des "threats" sont des faux positifs -> reclasses en anger/harassment
3. Rage sous-represente (523 samples) -> augmentation
4. Desequilibre massif (78% neutres) -> sous-echantillonnage
"""

import json
import random
import re
from pathlib import Path
from collections import Counter

random.seed(42)

DATASETS_DIR = Path(__file__).parent / "datasets"
TOXIC_DIR = DATASETS_DIR / "toxic"
NEUTRAL_DIR = DATASETS_DIR / "neutral"
OUTPUT_DIR = Path(__file__).parent / "datasets_clean"

LABEL_NAMES = {0: "neutral", 1: "anger", 2: "rage", 3: "threat", 4: "harassment"}

# ── Vocabulaire de detection ──

THREAT_KEYWORDS = [
    "tuer", "tue ", "mort ", "morts", "crever", "buter", "buté",
    "defoncer", "défoncer", "frapper", "tabasser",
    "égorger", "egorger", "exploser", "massacrer", "exterminer",
    "détruire", "detruire", "brûler", "bruler",
    "balle dans", "coup de couteau", "couteau", "arme ", "fusil",
    "te retrouver", "je vais te", "on va te",
    "attaquer", "envahir", "suicide", "suicid",
    "mourir", "éliminer", "eliminer",
    "violer", "viol ",
    "casser la gueule", "péter la gueule", "peter la gueule",
    "nique ta mere", "nique ta mère",
    "te planter", "te buter", "te crever", "te tuer",
    "pendaison", "pendre", "noyer",
]

HARASSMENT_KEYWORDS = [
    "dégage", "degage", "casse-toi", "casse toi", "tire-toi",
    "personne t'aime", "tout le monde te", "personne te",
    "t'es qu'un", "t'es qu'une", "t'es rien", "t'es nul",
    "ferme ta gueule", "ferme-la", "ta gueule",
    "on veut pas de toi", "tu sers a rien", "tu sers à rien",
    "retourne ", "rentre chez toi",
]

RAGE_KEYWORDS = [
    "nique", "niquer", "enculé", "enculer", "fils de pute",
    "fdp", "ntm", "ta mère", "ta mere",
    "bande de", "tous des", "race de",
    "AAAA", "!!!!",
    "tous crever", "tous mourir", "tous niquer",
    "sale race", "sous-race", "sous race",
]

TOXIC_WORDS = [
    "putain", "merde", "connard", "connasse", "gueule",
    "attardé", "attardée", "mongol", "débile", "enculé",
    "nique", "fdp", "pute", "salope", "bâtard", "batard",
    "abruti", "crétin", "cretin", "bouffon", "déchet",
    "ordure", "pourriture", "taré", "demeuré",
    "con ", "cons ", "conne ",
]


def has_keywords(text: str, keywords: list[str]) -> bool:
    low = text.lower()
    return any(k in low for k in keywords)


def is_short_or_empty(text: str) -> bool:
    clean = re.sub(r'[:@#]\w+', '', text).strip()
    return len(clean) < 10


def reclassify_threat(text: str) -> int:
    """Reclassifie un sample 'threat' mal classe."""
    if has_keywords(text, THREAT_KEYWORDS):
        return 3  # Vraie menace
    if has_keywords(text, RAGE_KEYWORDS):
        return 2  # Rage
    if has_keywords(text, HARASSMENT_KEYWORDS):
        return 4  # Harcelement
    if has_keywords(text, TOXIC_WORDS):
        return 1  # Anger
    return -1  # Pas toxique -> supprimer


def reclassify_neutral(text: str) -> int:
    """Reclassifie un sample 'neutre' qui contient des mots toxiques."""
    if has_keywords(text, THREAT_KEYWORDS):
        return 3
    if has_keywords(text, RAGE_KEYWORDS):
        return 2
    if has_keywords(text, HARASSMENT_KEYWORDS):
        return 4
    if has_keywords(text, TOXIC_WORDS):
        return 1
    return 0  # Reste neutre


# ── Templates d'augmentation rage ──

RAGE_TEMPLATES = [
    "PUTAIN MAIS {insult} VOUS ÊTES TOUS DES {insult2}",
    "JE VAIS TOUS VOUS NIQUER BANDE DE {insult2}",
    "MAIS FERME TA GUEULE ESPÈCE DE {insult} DE MERDE",
    "NIQUE TA MÈRE SALE {insult} VA",
    "T'ES QU'UN GROS {insult} DE MERDE JE VAIS TE NIQUER",
    "VOUS ÊTES TOUS DES {insult2} JE VOUS HAIS",
    "AAAA MAIS ENCULÉ {insult} T'AS PAS COMPRIS",
    "BANDE DE {insult2} VOUS MÉRITEZ TOUS DE CREVER",
    "SALE {insult} DE MERDE TU VAS VOIR CE QUI VA T'ARRIVER",
    "NIQUE TOUT LE MONDE ICI BANDE DE {insult2} DE MERDE",
    "MAIS PUTAIN DE {insult} LÂCHEZ-MOI",
    "ALLEZ TOUS VOUS FAIRE FOUTRE BANDE DE {insult2}",
    "JE HAIS CE SERVEUR ET TOUS LES {insult2} DESSUS",
    "ENCULÉ DE {insult} JE VAIS PÉTER UN CÂBLE",
    "FERMEZ VOS GUEULES BANDE DE {insult2} DE MERDE",
]

INSULTS_SINGLE = [
    "connard", "abruti", "débile", "attardé", "bouffon",
    "déchet", "taré", "demeuré", "crétin", "enculé",
    "minable", "raté", "tocard", "naze", "pauvre type",
]

INSULTS_PLURAL = [
    "connards", "abrutis", "débiles", "attardés", "bouffons",
    "déchets", "tarés", "demeurés", "crétins", "enculés",
    "minables", "ratés", "tocards", "nazes", "incapables",
]


def generate_rage_samples(count: int) -> list[dict]:
    samples = []
    for _ in range(count):
        template = random.choice(RAGE_TEMPLATES)
        text = template.format(
            insult=random.choice(INSULTS_SINGLE),
            insult2=random.choice(INSULTS_PLURAL),
        )
        samples.append({"text": text, "label": 2})
    return samples


def main():
    stats_before = Counter()
    stats_after = Counter()
    stats_reclassified = Counter()

    all_toxic: list[dict] = []
    all_neutral: list[str] = []

    # ── 1. Charger et nettoyer les fichiers toxiques ──

    print("=== Chargement des fichiers toxiques ===")
    for jsonl_file in TOXIC_DIR.glob("*.jsonl"):
        for line in jsonl_file.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                continue

            text = entry.get("text", "")
            label = entry.get("label", 1)
            stats_before[label] += 1

            if is_short_or_empty(text):
                continue

            # Reclassifier les threats (85% sont des faux positifs)
            if label == 3:
                new_label = reclassify_threat(text)
                if new_label == -1:
                    stats_reclassified["threat_supprime"] += 1
                    continue
                if new_label != 3:
                    stats_reclassified[f"threat->{LABEL_NAMES[new_label]}"] += 1
                label = new_label

            all_toxic.append({"text": text, "label": label})

    # ── 2. Charger et nettoyer les neutres ──

    print("=== Chargement des fichiers neutres ===")
    reclassified_to_toxic = []

    for txt_file in NEUTRAL_DIR.glob("*.txt"):
        for line in txt_file.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or is_short_or_empty(line):
                continue

            stats_before[0] += 1
            new_label = reclassify_neutral(line)
            if new_label != 0:
                reclassified_to_toxic.append({"text": line, "label": new_label})
                stats_reclassified[f"neutral->{LABEL_NAMES[new_label]}"] += 1
            else:
                all_neutral.append(line)

    all_toxic.extend(reclassified_to_toxic)

    # ── 3. Augmenter rage ──

    current_rage = sum(1 for t in all_toxic if t["label"] == 2)
    rage_target = 2500
    rage_to_generate = max(0, rage_target - current_rage)
    print(f"\n=== Augmentation rage: {current_rage} existants + {rage_to_generate} generes ===")
    all_toxic.extend(generate_rage_samples(rage_to_generate))

    # ── 4. Compter les toxiques par label ──

    toxic_counts = Counter(t["label"] for t in all_toxic)
    total_toxic = sum(toxic_counts.values())
    print(f"\nToxiques apres nettoyage: {total_toxic}")
    for label in sorted(toxic_counts.keys()):
        print(f"  {LABEL_NAMES[label]}: {toxic_counts[label]}")

    # ── 5. Sous-echantillonner les neutres ──

    # Objectif : ~40% neutres, ~60% toxiques (pour contrebalancer le biais)
    neutral_target = int(total_toxic * 0.7)
    print(f"\nNeutres disponibles: {len(all_neutral)}")
    print(f"Neutres cible: {neutral_target}")

    if len(all_neutral) > neutral_target:
        random.shuffle(all_neutral)
        all_neutral = all_neutral[:neutral_target]

    # ── 6. Ecrire les fichiers propres ──

    output_toxic = OUTPUT_DIR / "toxic"
    output_neutral = OUTPUT_DIR / "neutral"
    output_toxic.mkdir(parents=True, exist_ok=True)
    output_neutral.mkdir(parents=True, exist_ok=True)

    # Toxic JSONL
    random.shuffle(all_toxic)
    split_idx = int(len(all_toxic) * 0.9)
    train_toxic = all_toxic[:split_idx]
    test_toxic = all_toxic[split_idx:]

    with open(output_toxic / "train.jsonl", "w", encoding="utf-8") as f:
        for entry in train_toxic:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")

    with open(output_toxic / "test.jsonl", "w", encoding="utf-8") as f:
        for entry in test_toxic:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")

    # Neutral TXT
    random.shuffle(all_neutral)
    split_idx = int(len(all_neutral) * 0.9)
    train_neutral = all_neutral[:split_idx]
    test_neutral = all_neutral[split_idx:]

    with open(output_neutral / "train.txt", "w", encoding="utf-8") as f:
        f.write("\n".join(train_neutral))

    with open(output_neutral / "test.txt", "w", encoding="utf-8") as f:
        f.write("\n".join(test_neutral))

    # ── 7. Stats finales ──

    for t in all_toxic:
        stats_after[t["label"]] += 1
    stats_after[0] = len(all_neutral)

    print("\n" + "=" * 60)
    print("RESULTAT FINAL")
    print("=" * 60)

    total = sum(stats_after.values())
    for label in sorted(stats_after.keys()):
        before = stats_before.get(label, 0)
        after = stats_after[label]
        pct = after / total * 100
        print(f"  {LABEL_NAMES[label]:12s}: {before:>6d} -> {after:>6d}  ({pct:5.1f}%)")

    print(f"  {'TOTAL':12s}: {sum(stats_before.values()):>6d} -> {total:>6d}")

    print(f"\nReclassifications:")
    for k, v in sorted(stats_reclassified.items()):
        print(f"  {k}: {v}")

    print(f"\nFichiers ecrits dans: {OUTPUT_DIR}")
    print(f"  toxic/train.jsonl : {len(train_toxic)} samples")
    print(f"  toxic/test.jsonl  : {len(test_toxic)} samples")
    print(f"  neutral/train.txt : {len(train_neutral)} lignes")
    print(f"  neutral/test.txt  : {len(test_neutral)} lignes")


if __name__ == "__main__":
    main()
