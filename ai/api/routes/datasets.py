"""
Gestion des datasets — upload, listing, stats.
"""

import json
import logging
import shutil
from datetime import datetime, timezone
from pathlib import Path
from collections import Counter

from fastapi import APIRouter, UploadFile, File, HTTPException
from pydantic import BaseModel

from constants import ModelType, MAX_UPLOAD_BYTES, TEXT_EXTENSIONS

logger = logging.getLogger("sentinel.ai.datasets")

router = APIRouter()

AI_ROOT = Path(__file__).resolve().parent.parent.parent
TEXT_DATASETS = AI_ROOT / "training" / "text" / "datasets"
VISION_DATASETS = AI_ROOT / "training" / "vision" / "datasets"


class DatasetInfo(BaseModel):
    """Informations agregees d'un dataset."""

    model_type: str
    total_samples: int
    label_distribution: dict[str, int]
    last_updated: str | None


class UploadResult(BaseModel):
    """Resultat d'un upload de fichier."""

    uploaded: str
    size: int


class DeleteResult(BaseModel):
    """Resultat d'une suppression de dataset."""

    deleted: int


@router.get("/datasets")
async def list_datasets() -> list[DatasetInfo]:
    """Liste les datasets disponibles avec leurs statistiques."""
    return [_scan_text_datasets(), _scan_vision_datasets()]


@router.post("/datasets/{model_type}/upload")
async def upload_dataset(model_type: ModelType, file: UploadFile = File(...)) -> UploadResult:
    """Upload un fichier de dataset pour le type de modele donne."""
    if model_type == ModelType.TEXT_SENTIMENT:
        target_dir = TEXT_DATASETS / "toxic"
    else:
        target_dir = VISION_DATASETS

    target_dir.mkdir(parents=True, exist_ok=True)

    # Validation du nom de fichier
    if not file.filename:
        raise HTTPException(400, "Nom de fichier manquant")
    safe_name = Path(file.filename).name
    if not safe_name or safe_name.startswith("."):
        raise HTTPException(400, "Nom de fichier invalide")
    target_path = target_dir / safe_name
    if target_path.resolve().parent != target_dir.resolve():
        raise HTTPException(400, "Nom de fichier invalide (path traversal)")

    # Limitation de taille : lecture par chunks
    total_size = 0
    with open(target_path, "wb") as f:
        while chunk := await file.read(8192):
            total_size += len(chunk)
            if total_size > MAX_UPLOAD_BYTES:
                f.close()
                target_path.unlink(missing_ok=True)
                raise HTTPException(
                    413,
                    f"Fichier trop volumineux (max {MAX_UPLOAD_BYTES // (1024 * 1024)} Mo)",
                )
            f.write(chunk)

    logger.info("Dataset uploade: %s (%d octets) pour %s", safe_name, total_size, model_type.value)
    return UploadResult(uploaded=safe_name, size=total_size)


@router.delete("/datasets/{model_type}")
async def clear_dataset(model_type: ModelType) -> DeleteResult:
    """Supprime tous les fichiers d'un dataset."""
    if model_type == ModelType.TEXT_SENTIMENT:
        target_dir = TEXT_DATASETS
    else:
        target_dir = VISION_DATASETS

    count = 0
    for f in target_dir.rglob("*"):
        if f.is_file() and f.name != ".gitkeep":
            f.unlink()
            count += 1

    logger.info("Dataset %s nettoye: %d fichiers supprimes", model_type.value, count)
    return DeleteResult(deleted=count)


def _scan_text_datasets() -> DatasetInfo:
    """Scanne les datasets texte et retourne les statistiques."""
    labels: Counter[str] = Counter()
    label_names = {0: "neutral", 1: "anger", 2: "rage", 3: "threat", 4: "harassment"}
    last_mod: float | None = None

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
            if f.suffix not in TEXT_EXTENSIONS:
                continue
            if f.suffix == ".jsonl":
                skipped = 0
                for line in f.read_text(encoding="utf-8").splitlines():
                    if line.strip():
                        try:
                            entry = json.loads(line)
                        except json.JSONDecodeError:
                            skipped += 1
                            continue
                        label_id = entry.get("label", 1)
                        labels[label_names.get(label_id, str(label_id))] += 1
                if skipped > 0:
                    logger.warning("Dataset %s: %d lignes JSONL ignorees (malformees)", f.name, skipped)
            elif f.suffix == ".txt":
                count = sum(1 for line in f.read_text(encoding="utf-8").splitlines() if line.strip())
                labels["anger"] += count
            mod = f.stat().st_mtime
            if last_mod is None or mod > last_mod:
                last_mod = mod

    return DatasetInfo(
        model_type=ModelType.TEXT_SENTIMENT.value,
        total_samples=sum(labels.values()),
        label_distribution=dict(labels),
        last_updated=datetime.fromtimestamp(last_mod, tz=timezone.utc).isoformat() if last_mod else None,
    )


def _scan_vision_datasets() -> DatasetInfo:
    """Scanne les datasets vision et retourne les statistiques."""
    labels: Counter[str] = Counter()
    last_mod: float | None = None
    image_exts = {".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp"}

    if not VISION_DATASETS.exists():
        return DatasetInfo(
            model_type=ModelType.IMAGE_CLASSIFICATION.value,
            total_samples=0,
            label_distribution={},
            last_updated=None,
        )

    for class_dir in VISION_DATASETS.iterdir():
        if class_dir.is_dir():
            class_name = class_dir.name
            for f in class_dir.iterdir():
                if f.suffix.lower() in image_exts:
                    labels[class_name] += 1
                    mod = f.stat().st_mtime
                    if last_mod is None or mod > last_mod:
                        last_mod = mod

    return DatasetInfo(
        model_type=ModelType.IMAGE_CLASSIFICATION.value,
        total_samples=sum(labels.values()),
        label_distribution=dict(labels),
        last_updated=datetime.fromtimestamp(last_mod, tz=timezone.utc).isoformat() if last_mod else None,
    )
