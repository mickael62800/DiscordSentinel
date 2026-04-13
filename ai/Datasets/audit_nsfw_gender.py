"""
Audit du biais de genre dans le dataset nsfw/ existant, via NudeNet ONNX.

Passe le modele NudeNet sur toutes les images de training/vision/datasets/nsfw/
et compte les detections par classe pour en tirer :
- combien d'images contiennent du contenu masculin explicite
- combien contiennent du contenu feminin explicite
- combien sont ambigues / neutres (selon NudeNet)

Ecrit un rapport JSON + un CSV "gender_tags.csv" (une ligne par image avec son
tag dominant : male / female / both / none).
"""

import csv
import json
import os
import sys
from collections import Counter
from pathlib import Path

import numpy as np
from huggingface_hub import hf_hub_download
from PIL import Image
import onnxruntime as ort

# --- Config ---
AI_ROOT = Path(__file__).resolve().parent.parent
NSFW_DIR = AI_ROOT / "training" / "vision" / "datasets" / "nsfw"
OUT_DIR = Path(__file__).parent / "audit_nsfw"
OUT_DIR.mkdir(parents=True, exist_ok=True)

CONF_THRESHOLD = 0.35  # seuil de detection NudeNet
IMG_SIZE = 320

# Labels NudeNet 320n (ordre officiel du modele)
NUDENET_LABELS = [
    "FEMALE_GENITALIA_COVERED",
    "FACE_FEMALE",
    "BUTTOCKS_EXPOSED",
    "FEMALE_BREAST_EXPOSED",
    "FEMALE_GENITALIA_EXPOSED",
    "MALE_BREAST_EXPOSED",
    "ANUS_EXPOSED",
    "FEET_EXPOSED",
    "BELLY_COVERED",
    "FEET_COVERED",
    "ARMPITS_COVERED",
    "ARMPITS_EXPOSED",
    "FACE_MALE",
    "BELLY_EXPOSED",
    "MALE_GENITALIA_EXPOSED",
    "ANUS_COVERED",
    "FEMALE_BREAST_COVERED",
    "BUTTOCKS_COVERED",
]

MALE_LABELS = {"MALE_GENITALIA_EXPOSED", "MALE_BREAST_EXPOSED", "FACE_MALE"}
FEMALE_LABELS = {
    "FEMALE_GENITALIA_EXPOSED",
    "FEMALE_BREAST_EXPOSED",
    "FACE_FEMALE",
}
STRONG_MALE = {"MALE_GENITALIA_EXPOSED"}
STRONG_FEMALE = {"FEMALE_GENITALIA_EXPOSED", "FEMALE_BREAST_EXPOSED"}


def letterbox(img: np.ndarray, new_shape: int = IMG_SIZE) -> tuple[np.ndarray, float, tuple[int, int]]:
    """Resize en gardant le ratio, padding a 320x320 (style YOLO)."""
    h, w = img.shape[:2]
    r = min(new_shape / h, new_shape / w)
    new_unpad = (int(round(w * r)), int(round(h * r)))
    dw, dh = new_shape - new_unpad[0], new_shape - new_unpad[1]
    dw /= 2
    dh /= 2
    resized = np.array(
        Image.fromarray(img).resize(new_unpad, Image.BILINEAR)
    )
    top, bottom = int(round(dh - 0.1)), int(round(dh + 0.1))
    left, right = int(round(dw - 0.1)), int(round(dw + 0.1))
    padded = np.full((new_shape, new_shape, 3), 114, dtype=np.uint8)
    padded[top : top + new_unpad[1], left : left + new_unpad[0]] = resized
    return padded, r, (left, top)


def preprocess(pil_img: Image.Image) -> np.ndarray:
    img = pil_img.convert("RGB")
    arr = np.array(img)
    lb, _, _ = letterbox(arr, IMG_SIZE)
    lb = lb.astype(np.float32) / 255.0
    lb = lb.transpose(2, 0, 1)  # HWC -> CHW
    return np.expand_dims(lb, 0)


def run_inference(session: ort.InferenceSession, pil_img: Image.Image) -> set[str]:
    """Retourne l'ensemble des labels detectes au-dessus du seuil."""
    x = preprocess(pil_img)
    in_name = session.get_inputs()[0].name
    out = session.run(None, {in_name: x})[0]
    # out shape: (1, N, 6) typiquement [x, y, w, h, conf, class] — varie selon export
    # Pour NudeNet 320n : (1, 22, 2100) (YOLO style) — boxes + scores per class
    # Strategie simple : inspecter la forme et extraire les scores max par classe
    arr = np.squeeze(out, 0)
    labels_found: set[str] = set()
    if arr.ndim == 2:
        # (22, N) : 4 bbox + 18 classes OU (N, 6) : bbox + conf + cls
        h, w = arr.shape
        if h == 22 and w > 22:
            # format (4+18, N) : rows 4..22 sont les scores par classe
            class_scores = arr[4:, :]  # (18, N)
            max_per_class = class_scores.max(axis=1)
            for i, score in enumerate(max_per_class):
                if score >= CONF_THRESHOLD and i < len(NUDENET_LABELS):
                    labels_found.add(NUDENET_LABELS[i])
        elif w == 6:
            # format (N, 6) : rows = detections
            for det in arr:
                if det[4] >= CONF_THRESHOLD:
                    cls_id = int(det[5])
                    if 0 <= cls_id < len(NUDENET_LABELS):
                        labels_found.add(NUDENET_LABELS[cls_id])
    return labels_found


def classify_gender(labels: set[str]) -> str:
    has_strong_m = bool(labels & STRONG_MALE)
    has_strong_f = bool(labels & STRONG_FEMALE)
    has_m = bool(labels & MALE_LABELS)
    has_f = bool(labels & FEMALE_LABELS)

    if has_strong_m and has_strong_f:
        return "both"
    if has_strong_m:
        return "male"
    if has_strong_f:
        return "female"
    if has_m and has_f:
        return "both_soft"
    if has_m:
        return "male_soft"
    if has_f:
        return "female_soft"
    return "none"


def main() -> None:
    print(f"Telechargement NudeNet 320n...")
    model_path = hf_hub_download(
        repo_id="deepghs/nudenet_onnx",
        filename="320n.onnx",
        token=os.environ.get("HF_TOKEN"),
    )
    print(f"  {model_path}")

    providers = ["CPUExecutionProvider"]
    if "CUDAExecutionProvider" in ort.get_available_providers():
        providers.insert(0, "CUDAExecutionProvider")
    print(f"Providers: {providers}")

    session = ort.InferenceSession(model_path, providers=providers)
    in_shape = session.get_inputs()[0].shape
    out_shape = session.get_outputs()[0].shape
    print(f"Input shape: {in_shape}, Output shape: {out_shape}")

    files = sorted(
        f for f in NSFW_DIR.iterdir()
        if f.suffix.lower() in {".jpg", ".jpeg", ".png", ".webp", ".bmp"}
    )
    print(f"Images a analyser: {len(files)}")

    gender_counts: Counter = Counter()
    label_counts: Counter = Counter()
    per_image: list[tuple[str, str, str]] = []

    errors = 0
    for i, fp in enumerate(files):
        if i % 200 == 0:
            print(f"  {i}/{len(files)}")
        try:
            with Image.open(fp) as img:
                labels = run_inference(session, img)
        except Exception as e:
            errors += 1
            continue
        for lb in labels:
            label_counts[lb] += 1
        gender = classify_gender(labels)
        gender_counts[gender] += 1
        per_image.append((fp.name, gender, "|".join(sorted(labels))))

    print(f"\nTermine. Erreurs: {errors}")
    print(f"\nRepartition par genre dominant:")
    total = sum(gender_counts.values())
    for g, c in gender_counts.most_common():
        pct = 100 * c / total if total else 0
        print(f"  {g:15s}: {c:5d}  ({pct:5.1f}%)")

    print(f"\nTop labels NudeNet (images >= {CONF_THRESHOLD}):")
    for lb, c in label_counts.most_common():
        print(f"  {lb:30s}: {c}")

    # Ecriture rapport
    report = {
        "total_images": len(files),
        "errors": errors,
        "gender_distribution": dict(gender_counts),
        "label_counts": dict(label_counts),
        "threshold": CONF_THRESHOLD,
    }
    (OUT_DIR / "audit_report.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )

    with open(OUT_DIR / "gender_tags.csv", "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f)
        w.writerow(["filename", "gender", "nudenet_labels"])
        for row in per_image:
            w.writerow(row)

    print(f"\nRapport ecrit dans: {OUT_DIR}")


if __name__ == "__main__":
    main()
