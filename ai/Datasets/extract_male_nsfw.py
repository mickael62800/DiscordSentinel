"""
Etape 2 : extraire les images a dominante masculine depuis nsfw_dataset_v1.zip
et les ajouter au dataset nsfw/ existant.

Flux :
  1. Ouvrir le zip en streaming (sans extraire tout sur disque)
  2. Pour chaque image, faire tourner NudeNet
  3. Garder les images avec MALE_GENITALIA_EXPOSED ou MALE_BREAST_EXPOSED >= 0.35
     et PAS de FEMALE_GENITALIA_EXPOSED ni FEMALE_BREAST_EXPOSED (eviter les mixtes)
  4. SHA1 du fichier pour dedup contre nsfw/ existant
  5. Copier dans training/vision/datasets/nsfw/ avec prefixe male_
  6. Cap a MAX_MALE images (defaut 1000)
"""

import hashlib
import io
import os
import sys
import zipfile
from collections import Counter
from pathlib import Path

import numpy as np
import onnxruntime as ort
from huggingface_hub import hf_hub_download
from PIL import Image

AI_ROOT = Path(__file__).resolve().parent.parent
NSFW_DIR = AI_ROOT / "training" / "vision" / "datasets" / "nsfw"

ZIP_CACHE = Path(os.environ.get("USERPROFILE", "")) / ".cache" / "huggingface" / "hub"

CONF = 0.35
IMG_SIZE = 320
MAX_MALE = 1000

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

MALE_HARD = {"MALE_GENITALIA_EXPOSED", "MALE_BREAST_EXPOSED"}
FEMALE_HARD = {"FEMALE_GENITALIA_EXPOSED", "FEMALE_BREAST_EXPOSED"}


def letterbox_pil(img: Image.Image, size: int = IMG_SIZE) -> np.ndarray:
    w, h = img.size
    r = min(size / h, size / w)
    nw, nh = int(round(w * r)), int(round(h * r))
    resized = img.resize((nw, nh), Image.BILINEAR)
    canvas = Image.new("RGB", (size, size), (114, 114, 114))
    canvas.paste(resized, ((size - nw) // 2, (size - nh) // 2))
    arr = np.array(canvas, dtype=np.float32) / 255.0
    return arr.transpose(2, 0, 1)[None, ...]


def infer(session: ort.InferenceSession, img: Image.Image) -> dict[str, float]:
    x = letterbox_pil(img)
    out = session.run(None, {session.get_inputs()[0].name: x})[0]
    arr = np.squeeze(out, 0)  # (22, N)
    if arr.shape[0] != 22:
        return {}
    class_scores = arr[4:, :]  # (18, N)
    max_per_class = class_scores.max(axis=1)
    return {
        NUDENET_LABELS[i]: float(max_per_class[i])
        for i in range(len(NUDENET_LABELS))
    }


def load_existing_hashes() -> set[str]:
    """Hash SHA1 des fichiers existants pour dedup."""
    seen = set()
    for fp in NSFW_DIR.iterdir():
        if fp.is_file():
            try:
                h = hashlib.sha1(fp.read_bytes()).hexdigest()
                seen.add(h)
            except Exception:
                pass
    return seen


def main() -> None:
    print("Chargement NudeNet...")
    model_path = hf_hub_download(
        "deepghs/nudenet_onnx", "320n.onnx",
        token=os.environ.get("HF_TOKEN"),
    )
    providers = ["CPUExecutionProvider"]
    if "CUDAExecutionProvider" in ort.get_available_providers():
        providers.insert(0, "CUDAExecutionProvider")
    session = ort.InferenceSession(model_path, providers=providers)
    print(f"  providers: {providers}")

    print("Recuperation du zip source...")
    zip_path = hf_hub_download(
        "deepghs/nsfw_detect", "nsfw_dataset_v1.zip",
        repo_type="dataset",
        token=os.environ.get("HF_TOKEN"),
    )
    print(f"  {zip_path}")

    print("Indexation des hashes existants (nsfw/)...")
    existing = load_existing_hashes()
    print(f"  {len(existing)} hashes indexes")

    NSFW_DIR.mkdir(parents=True, exist_ok=True)

    kept = 0
    processed = 0
    skipped_female = 0
    skipped_nomale = 0
    skipped_dup = 0
    skipped_err = 0

    class_seen: Counter = Counter()  # sous-dossiers rencontres dans le zip

    with zipfile.ZipFile(zip_path) as zf:
        # Lister pour avoir une progression
        members = [m for m in zf.namelist() if m.lower().endswith((".jpg", ".jpeg", ".png", ".webp", ".bmp"))]
        print(f"  {len(members)} images dans le zip")

        for i, member in enumerate(members):
            if kept >= MAX_MALE:
                break
            if i % 500 == 0:
                print(f"  {i}/{len(members)}  kept={kept}  dup={skipped_dup}  female={skipped_female}  nomale={skipped_nomale}")

            # Extraire le "sous-dossier" (premiere partie du path) pour stats
            parts = member.split("/")
            cls = parts[1] if len(parts) > 1 else parts[0]
            class_seen[cls] += 1

            try:
                raw = zf.read(member)
            except Exception:
                skipped_err += 1
                continue

            # Dedup avant inference (plus rapide)
            h = hashlib.sha1(raw).hexdigest()
            if h in existing:
                skipped_dup += 1
                continue

            try:
                img = Image.open(io.BytesIO(raw)).convert("RGB")
            except Exception:
                skipped_err += 1
                continue

            processed += 1
            scores = infer(session, img)
            if not scores:
                skipped_err += 1
                continue

            has_male = any(scores.get(k, 0) >= CONF for k in MALE_HARD)
            has_female = any(scores.get(k, 0) >= CONF for k in FEMALE_HARD)

            if not has_male:
                skipped_nomale += 1
                continue
            if has_female:
                skipped_female += 1
                continue

            # Garder : copier dans nsfw/ avec prefixe male_ + hash
            ext = Path(member).suffix.lower() or ".jpg"
            out_name = f"male_{h[:16]}{ext}"
            (NSFW_DIR / out_name).write_bytes(raw)
            existing.add(h)
            kept += 1

    print("\n" + "=" * 50)
    print("RESULTATS")
    print("=" * 50)
    print(f"  Examines (decodes + NudeNet): {processed}")
    print(f"  Gardes (male only)          : {kept}")
    print(f"  Rejetes - pas de male       : {skipped_nomale}")
    print(f"  Rejetes - male+female       : {skipped_female}")
    print(f"  Rejetes - doublons SHA1     : {skipped_dup}")
    print(f"  Erreurs decodage            : {skipped_err}")
    print(f"\n  Classes vues dans le zip:")
    for c, n in class_seen.most_common(10):
        print(f"    {c:20s}: {n}")
    print(f"\n  Nouveaux fichiers dans: {NSFW_DIR}")


if __name__ == "__main__":
    main()
