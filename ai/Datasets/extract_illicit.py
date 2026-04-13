"""
Telecharge et integre des images illicites (armes, drogues, gore) dans
training/vision/datasets/illicit/.

Sources (tous non-gated sur HuggingFace) :
  - fcakyon/gun-object-detection              (firearms, ~88 MB)
  - TrainingDataPro/people-with-guns-...      (firearms, ~66 MB)
  - Simuletic/cctv-knife-detection-dataset    (knives, ~167 MB)
  - Ultralytics/Medical-pills                 (pills, ~8 MB)
  - NeuralShell/Gore-Blood-Dataset-v1.0       (gore, ~189 MB)

Pour chaque source : telecharge, extrait les images (ou lit direct), SHA1 dedup
contre l'existant, copie dans illicit/ avec prefixe de categorie.
"""

import hashlib
import io
import os
import shutil
import zipfile
from pathlib import Path

from huggingface_hub import hf_hub_download, snapshot_download
from PIL import Image

AI_ROOT = Path(__file__).resolve().parent.parent
ILLICIT_DIR = AI_ROOT / "training" / "vision" / "datasets" / "illicit"
ILLICIT_DIR.mkdir(parents=True, exist_ok=True)

TOKEN = os.environ.get("HF_TOKEN")

IMG_EXT = {".jpg", ".jpeg", ".png", ".webp", ".bmp"}

# Cap par categorie
CAPS = {
    "firearm": 800,
    "knife": 500,
    "pill": 300,
    "gore": 500,
}


def load_existing_hashes() -> set[str]:
    print(f"  indexation hashes existants dans {ILLICIT_DIR.name}/...")
    seen = set()
    for fp in ILLICIT_DIR.iterdir():
        if fp.is_file():
            try:
                seen.add(hashlib.sha1(fp.read_bytes()).hexdigest())
            except Exception:
                pass
    print(f"  {len(seen)} hashes indexes")
    return seen


def is_valid_image(data: bytes) -> bool:
    try:
        with Image.open(io.BytesIO(data)) as img:
            img.verify()
        return True
    except Exception:
        return False


def save_if_new(data: bytes, prefix: str, ext: str, seen: set[str]) -> bool:
    if not is_valid_image(data):
        return False
    h = hashlib.sha1(data).hexdigest()
    if h in seen:
        return False
    seen.add(h)
    out = ILLICIT_DIR / f"{prefix}_{h[:16]}{ext.lower()}"
    out.write_bytes(data)
    return True


def process_zip_member(zf: zipfile.ZipFile, member: str, prefix: str, seen: set[str]) -> bool:
    ext = Path(member).suffix.lower()
    if ext not in IMG_EXT:
        return False
    try:
        data = zf.read(member)
    except Exception:
        return False
    return save_if_new(data, prefix, ext, seen)


def process_zip(zip_path: Path, prefix: str, cap: int, seen: set[str]) -> int:
    kept = 0
    with zipfile.ZipFile(zip_path) as zf:
        members = [m for m in zf.namelist() if Path(m).suffix.lower() in IMG_EXT]
        print(f"    zip {zip_path.name}: {len(members)} images")
        for m in members:
            if kept >= cap:
                break
            if process_zip_member(zf, m, prefix, seen):
                kept += 1
    return kept


def dl_zip(repo_id: str, filename: str, repo_type: str = "dataset") -> Path:
    return Path(hf_hub_download(repo_id, filename, repo_type=repo_type, token=TOKEN))


# ──────────────────────────────────────────────────────────────────────

def category_firearms(seen: set[str]) -> int:
    print("\n=== FIREARMS ===")
    cap = CAPS["firearm"]
    kept = 0

    # 1. fcakyon/gun-object-detection : train.zip + valid.zip
    for fname in ["data/train.zip", "data/valid.zip"]:
        if kept >= cap:
            break
        try:
            zp = dl_zip("fcakyon/gun-object-detection", fname)
            kept += process_zip(zp, "firearm", cap - kept, seen)
            print(f"    after {fname}: kept={kept}")
        except Exception as e:
            print(f"    ! erreur {fname}: {e}")

    # 2. TrainingDataPro : single zip
    if kept < cap:
        try:
            zp = dl_zip(
                "TrainingDataPro/people-with-guns-segmentation-and-detection",
                "data/people-with-guns-segmentation-and-detection.zip",
            )
            kept += process_zip(zp, "firearm", cap - kept, seen)
            print(f"    after TrainingDataPro: kept={kept}")
        except Exception as e:
            print(f"    ! erreur TrainingDataPro: {e}")

    return kept


def category_knives(seen: set[str]) -> int:
    print("\n=== KNIVES ===")
    cap = CAPS["knife"]
    kept = 0
    # Ce dataset est organise en fichiers individuels, pas un zip.
    # On snapshot tout le repo puis on scan.
    try:
        print("    snapshot_download Simuletic/cctv-knife-detection-dataset...")
        root = Path(
            snapshot_download(
                "Simuletic/cctv-knife-detection-dataset",
                repo_type="dataset",
                token=TOKEN,
                allow_patterns=["*.png", "*.jpg", "*.jpeg"],
            )
        )
        for fp in root.rglob("*"):
            if kept >= cap:
                break
            if fp.is_file() and fp.suffix.lower() in IMG_EXT:
                try:
                    data = fp.read_bytes()
                except Exception:
                    continue
                if save_if_new(data, "knife", fp.suffix, seen):
                    kept += 1
        print(f"    kept={kept}")
    except Exception as e:
        print(f"    ! erreur knives: {e}")
    return kept


def category_pills(seen: set[str]) -> int:
    print("\n=== PILLS ===")
    cap = CAPS["pill"]
    kept = 0
    try:
        print("    snapshot_download Ultralytics/Medical-pills...")
        root = Path(
            snapshot_download(
                "Ultralytics/Medical-pills",
                repo_type="dataset",
                token=TOKEN,
                allow_patterns=["*.jpg", "*.jpeg", "*.png"],
            )
        )
        for fp in root.rglob("*"):
            if kept >= cap:
                break
            if fp.is_file() and fp.suffix.lower() in IMG_EXT:
                try:
                    data = fp.read_bytes()
                except Exception:
                    continue
                if save_if_new(data, "pill", fp.suffix, seen):
                    kept += 1
        print(f"    kept={kept}")
    except Exception as e:
        print(f"    ! erreur pills: {e}")
    return kept


def category_gore(seen: set[str]) -> int:
    print("\n=== GORE ===")
    cap = CAPS["gore"]
    kept = 0
    # 3 zips
    for fname in ["mixed-dataset.zip", "new-dataset.zip", "old-dataset.zip"]:
        if kept >= cap:
            break
        try:
            zp = dl_zip("NeuralShell/Gore-Blood-Dataset-v1.0", fname)
            kept += process_zip(zp, "gore", cap - kept, seen)
            print(f"    after {fname}: kept={kept}")
        except Exception as e:
            print(f"    ! erreur {fname}: {e}")
    return kept


def main() -> None:
    seen = load_existing_hashes()

    totals: dict[str, int] = {}
    totals["firearm"] = category_firearms(seen)
    totals["knife"] = category_knives(seen)
    totals["pill"] = category_pills(seen)
    totals["gore"] = category_gore(seen)

    print("\n" + "=" * 50)
    print("BILAN")
    print("=" * 50)
    grand_total = 0
    for cat, n in totals.items():
        print(f"  {cat:10s}: +{n:4d} images")
        grand_total += n
    print(f"  {'TOTAL':10s}: +{grand_total} images ajoutees a illicit/")
    print(f"\n  Dossier: {ILLICIT_DIR}")


if __name__ == "__main__":
    main()
