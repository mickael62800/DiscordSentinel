"""
Gestion des datasets — upload, listing, stats
"""

import json
import shutil
from pathlib import Path
from collections import Counter

from fastapi import APIRouter, UploadFile, File, HTTPException
from pydantic import BaseModel

router = APIRouter()

AI_ROOT = Path(__file__).resolve().parent.parent.parent
TEXT_DATASETS = AI_ROOT / "training" / "text" / "datasets"
VISION_DATASETS = AI_ROOT / "training" / "vision" / "datasets"


class DatasetInfo(BaseModel):
    model_type: str
    total_samples: int
    label_distribution: dict[str, int]
    last_updated: str | None


@router.get("/datasets")
async def list_datasets() -> list[DatasetInfo]:
    results = []

    # Text datasets
    text_info = _scan_text_datasets()
    results.append(text_info)

    # Vision datasets
    vision_info = _scan_vision_datasets()
    results.append(vision_info)

    return results


@router.post("/datasets/{model_type}/upload")
async def upload_dataset(model_type: str, file: UploadFile = File(...)):
    if model_type == "text-sentiment":
        target_dir = TEXT_DATASETS / "toxic"
    elif model_type == "image-classification":
        target_dir = VISION_DATASETS
    else:
        raise HTTPException(400, f"Type de modele inconnu: {model_type}")

    target_dir.mkdir(parents=True, exist_ok=True)
    target_path = target_dir / file.filename

    with open(target_path, "wb") as f:
        shutil.copyfileobj(file.file, f)

    return {"uploaded": file.filename, "size": target_path.stat().st_size}


@router.delete("/datasets/{model_type}")
async def clear_dataset(model_type: str):
    if model_type == "text-sentiment":
        target_dir = TEXT_DATASETS
    elif model_type == "image-classification":
        target_dir = VISION_DATASETS
    else:
        raise HTTPException(400, f"Type de modele inconnu: {model_type}")

    count = 0
    for f in target_dir.rglob("*"):
        if f.is_file() and f.name != ".gitkeep":
            f.unlink()
            count += 1

    return {"deleted": count}


def _scan_text_datasets() -> DatasetInfo:
    labels = Counter()
    label_names = {0: "neutral", 1: "anger", 2: "rage", 3: "threat", 4: "harassment"}
    last_mod = None

    neutral_dir = TEXT_DATASETS / "neutral"
    if neutral_dir.exists():
        for f in neutral_dir.iterdir():
            if f.suffix == ".txt":
                count = sum(1 for line in f.read_text(encoding="utf-8").splitlines() if line.strip())
                labels["neutral"] += count
                mod = f.stat().st_mtime
                if last_mod is None or mod > last_mod:
                    last_mod = mod

    toxic_dir = TEXT_DATASETS / "toxic"
    if toxic_dir.exists():
        for f in toxic_dir.iterdir():
            if f.suffix == ".jsonl":
                for line in f.read_text(encoding="utf-8").splitlines():
                    if line.strip():
                        entry = json.loads(line)
                        label_id = entry.get("label", 1)
                        labels[label_names.get(label_id, str(label_id))] += 1
                mod = f.stat().st_mtime
                if last_mod is None or mod > last_mod:
                    last_mod = mod
            elif f.suffix == ".txt":
                count = sum(1 for line in f.read_text(encoding="utf-8").splitlines() if line.strip())
                labels["anger"] += count

    from datetime import datetime
    return DatasetInfo(
        model_type="text-sentiment",
        total_samples=sum(labels.values()),
        label_distribution=dict(labels),
        last_updated=datetime.fromtimestamp(last_mod).isoformat() if last_mod else None,
    )


def _scan_vision_datasets() -> DatasetInfo:
    labels = Counter()
    last_mod = None
    image_exts = {".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"}

    for class_dir in VISION_DATASETS.iterdir():
        if class_dir.is_dir():
            class_name = class_dir.name
            for f in class_dir.iterdir():
                if f.suffix.lower() in image_exts:
                    labels[class_name] += 1
                    mod = f.stat().st_mtime
                    if last_mod is None or mod > last_mod:
                        last_mod = mod

    from datetime import datetime
    return DatasetInfo(
        model_type="image-classification",
        total_samples=sum(labels.values()),
        label_distribution=dict(labels),
        last_updated=datetime.fromtimestamp(last_mod).isoformat() if last_mod else None,
    )
