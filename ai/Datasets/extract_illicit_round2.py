"""
Round 2 : ajoute des images aux categories sous-alimentees de illicit/.

Sources :
  - phil20/weapon (knife/ : 199 images, rifle/pistol/grenade pour backup weapons)
  - Besedo/artificial_weapon (weapon/ : 602 images, mixed)
  - AIQUADRANT/Narcotics (parquet, drugs)
  - henriquequeirozcunha/violence-detection-dataset (parquet, violence)
"""

import hashlib
import io
import os
from pathlib import Path

from huggingface_hub import hf_hub_download, snapshot_download
from PIL import Image

AI_ROOT = Path(__file__).resolve().parent.parent
ILLICIT_DIR = AI_ROOT / "training" / "vision" / "datasets" / "illicit"
TOKEN = os.environ.get("HF_TOKEN")
IMG_EXT = {".jpg", ".jpeg", ".png", ".webp", ".bmp"}


def load_existing_hashes() -> set[str]:
    print(f"Indexation de {ILLICIT_DIR.name}/...")
    seen = set()
    for fp in ILLICIT_DIR.iterdir():
        if fp.is_file():
            try:
                seen.add(hashlib.sha1(fp.read_bytes()).hexdigest())
            except Exception:
                pass
    print(f"  {len(seen)} hashes")
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


def category_phil20_knives(seen: set[str]) -> dict[str, int]:
    """Extrait UNIQUEMENT le sous-dossier knife/ (et bonus rifle/pistol/grenade)."""
    print("\n=== phil20/weapon ===")
    root = Path(
        snapshot_download(
            "phil20/weapon",
            repo_type="dataset",
            token=TOKEN,
            allow_patterns=["knife/*", "rifle/*", "pistol/*", "grenade/*"],
        )
    )
    counts = {"knife": 0, "firearm": 0}
    for fp in root.rglob("*"):
        if not (fp.is_file() and fp.suffix.lower() in IMG_EXT):
            continue
        subdir = fp.parent.name.lower()
        if subdir == "knife":
            prefix = "knife"
            key = "knife"
        elif subdir in ("rifle", "pistol", "grenade"):
            prefix = "firearm"  # regrouper sous firearm pour cohérence
            key = "firearm"
        else:
            continue
        try:
            data = fp.read_bytes()
        except Exception:
            continue
        if save_if_new(data, prefix, fp.suffix, seen):
            counts[key] += 1
    print(f"  knife: +{counts['knife']}, firearm: +{counts['firearm']}")
    return counts


def category_besedo(seen: set[str]) -> int:
    """Besedo/artificial_weapon : uniquement le sous-dossier weapon/ (pas no_weapon)."""
    print("\n=== Besedo/artificial_weapon ===")
    root = Path(
        snapshot_download(
            "Besedo/artificial_weapon",
            repo_type="dataset",
            token=TOKEN,
            allow_patterns=["data/train/weapon/*", "data/test/weapon/*", "data/validation/weapon/*"],
        )
    )
    kept = 0
    for fp in root.rglob("*"):
        if fp.is_file() and fp.suffix.lower() in IMG_EXT:
            parts = [p.lower() for p in fp.parts]
            if "weapon" not in parts or "no_weapon" in parts:
                continue
            try:
                data = fp.read_bytes()
            except Exception:
                continue
            if save_if_new(data, "weapon", fp.suffix, seen):
                kept += 1
    print(f"  weapon: +{kept}")
    return kept


def process_parquet_image_column(
    parquet_path: Path,
    prefix: str,
    seen: set[str],
    cap: int,
    label_filter=None,  # callable(row) -> bool, ou None
) -> int:
    """Lit un parquet HF image dataset et extrait les bytes."""
    import pyarrow.parquet as pq

    table = pq.read_table(parquet_path)
    cols = table.column_names
    # Trouver la colonne image
    img_col = None
    for c in ["image", "img", "images"]:
        if c in cols:
            img_col = c
            break
    if img_col is None:
        print(f"    ! pas de colonne image dans {parquet_path.name}, cols={cols}")
        return 0

    rows = table.to_pylist()
    kept = 0
    for row in rows:
        if kept >= cap:
            break
        if label_filter and not label_filter(row):
            continue
        img_obj = row[img_col]
        if img_obj is None:
            continue
        # HF image column : {"bytes": b"...", "path": None}
        if isinstance(img_obj, dict) and "bytes" in img_obj:
            data = img_obj["bytes"]
        elif isinstance(img_obj, bytes):
            data = img_obj
        else:
            continue
        if not data:
            continue
        # Deviner extension via PIL
        try:
            with Image.open(io.BytesIO(data)) as img:
                fmt = (img.format or "JPEG").lower()
            ext = "." + ("jpg" if fmt == "jpeg" else fmt)
        except Exception:
            continue
        if save_if_new(data, prefix, ext, seen):
            kept += 1
    return kept


def category_narcotics(seen: set[str]) -> int:
    print("\n=== AIQUADRANT/Narcotics ===")
    try:
        pq_path = Path(
            hf_hub_download(
                "AIQUADRANT/Narcotics",
                "data/data-00000-of-00001.parquet",
                repo_type="dataset",
                token=TOKEN,
            )
        )
    except Exception as e:
        print(f"    ! erreur download: {e}")
        return 0
    n = process_parquet_image_column(pq_path, "drug", seen, cap=500)
    print(f"  drug: +{n}")
    return n


def category_violence(seen: set[str]) -> int:
    """henriquequeirozcunha/violence-detection-dataset : 6 parquets de ~430 MB.
    On ne prend que le premier pour eviter de tout DL (~3.4 GB total)."""
    print("\n=== henriquequeirozcunha/violence-detection-dataset ===")
    try:
        pq_path = Path(
            hf_hub_download(
                "henriquequeirozcunha/violence-detection-dataset",
                "data/train-00000-of-00006-1b45c026e07ad7d5.parquet",
                repo_type="dataset",
                token=TOKEN,
            )
        )
    except Exception as e:
        print(f"    ! erreur download: {e}")
        return 0

    # Filtrer : ne garder que les lignes 'violence' (pas 'non-violence')
    def filt(row):
        lab = row.get("label", row.get("labels", ""))
        if isinstance(lab, int):
            return lab == 1  # convention habituelle : 1 = violent
        if isinstance(lab, str):
            return "violen" in lab.lower() and "non" not in lab.lower()
        return True  # accepter si pas de label

    n = process_parquet_image_column(pq_path, "violence", seen, cap=500, label_filter=filt)
    print(f"  violence: +{n}")
    return n


def main() -> None:
    seen = load_existing_hashes()

    counts = {
        "knife": 0,
        "firearm": 0,
        "weapon": 0,
        "drug": 0,
        "violence": 0,
    }

    r = category_phil20_knives(seen)
    counts["knife"] += r["knife"]
    counts["firearm"] += r["firearm"]

    counts["weapon"] = category_besedo(seen)
    counts["drug"] = category_narcotics(seen)
    counts["violence"] = category_violence(seen)

    print("\n" + "=" * 50)
    print("BILAN ROUND 2")
    print("=" * 50)
    total = 0
    for cat, n in counts.items():
        print(f"  {cat:10s}: +{n:4d}")
        total += n
    print(f"  {'TOTAL':10s}: +{total}")
    print(f"\n  Dossier: {ILLICIT_DIR}")


if __name__ == "__main__":
    main()
