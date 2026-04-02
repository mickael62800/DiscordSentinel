"""
Export des modeles entraines vers ONNX pour inference Rust.
"""

import io
import logging
import os
import sys
from pathlib import Path

import yaml
import torch
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from constants import ModelType

# Fix Windows console encoding pour eviter les erreurs d'emoji
if sys.stdout.encoding != "utf-8":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")

logger = logging.getLogger("sentinel.ai.export")

router = APIRouter()

AI_ROOT = Path(__file__).resolve().parent.parent.parent


class ExportResult(BaseModel):
    """Resultat d'un export ONNX."""

    model_type: str
    file_path: str
    file_size_bytes: int


class ModelInfo(BaseModel):
    """Informations sur un modele disponible."""

    model_type: str
    file_path: str
    file_size_bytes: int | None = None
    last_modified: float | None = None
    status: str | None = None


def _load_config(model_type: ModelType) -> dict:
    """Charge la configuration YAML d'un type de modele."""
    if model_type == ModelType.TEXT_SENTIMENT:
        config_path = AI_ROOT / "training" / "text" / "configs" / "train_config.yaml"
    else:
        config_path = AI_ROOT / "training" / "vision" / "configs" / "train_config.yaml"

    if not config_path.exists():
        raise HTTPException(404, f"Fichier de configuration introuvable: {config_path.name}")

    try:
        with open(config_path) as f:
            return yaml.safe_load(f)
    except yaml.YAMLError as e:
        logger.error("Erreur parsing YAML %s: %s", config_path, e)
        raise HTTPException(500, f"Fichier de configuration invalide: {config_path.name}")


@router.post("/export/{model_type}")
async def export_onnx(model_type: ModelType) -> ExportResult:
    """Exporte un modele entraine au format ONNX."""
    if model_type == ModelType.TEXT_SENTIMENT:
        return _export_text()
    else:
        return _export_vision()


@router.get("/models")
async def list_models() -> list[ModelInfo]:
    """Liste les modeles exportes et checkpoints disponibles."""
    models: list[ModelInfo] = []

    text_onnx = AI_ROOT / "training" / "text" / "exports" / "text_sentinel.onnx"
    if text_onnx.exists():
        stat = text_onnx.stat()
        models.append(ModelInfo(
            model_type=ModelType.TEXT_SENTIMENT.value,
            file_path=str(text_onnx),
            file_size_bytes=stat.st_size,
            last_modified=stat.st_mtime,
        ))

    vision_onnx = AI_ROOT / "training" / "vision" / "exports" / "vision_sentinel.onnx"
    if vision_onnx.exists():
        stat = vision_onnx.stat()
        models.append(ModelInfo(
            model_type=ModelType.IMAGE_CLASSIFICATION.value,
            file_path=str(vision_onnx),
            file_size_bytes=stat.st_size,
            last_modified=stat.st_mtime,
        ))

    text_checkpoint = AI_ROOT / "training" / "text" / "checkpoints" / "best_model"
    if text_checkpoint.exists():
        models.append(ModelInfo(
            model_type="text-sentiment-checkpoint",
            file_path=str(text_checkpoint),
            status="pret pour export",
        ))

    vision_checkpoint = AI_ROOT / "training" / "vision" / "checkpoints" / "best_model.pt"
    if vision_checkpoint.exists():
        models.append(ModelInfo(
            model_type="image-classification-checkpoint",
            file_path=str(vision_checkpoint),
            file_size_bytes=vision_checkpoint.stat().st_size,
            status="pret pour export",
        ))

    return models


def _export_text() -> ExportResult:
    """Exporte le modele text-sentiment vers ONNX."""
    try:
        import onnx
        from onnxsim import simplify
        from transformers import AutoModelForSequenceClassification, AutoTokenizer

        config = _load_config(ModelType.TEXT_SENTIMENT)

        checkpoint_path = AI_ROOT / "training" / "text" / "checkpoints" / "best_model"
        if not checkpoint_path.exists():
            raise HTTPException(404, "Aucun checkpoint text trouve. Lancez un entrainement d'abord.")

        logger.info("Export text-sentiment: chargement du checkpoint")
        model = AutoModelForSequenceClassification.from_pretrained(str(checkpoint_path))
        tokenizer = AutoTokenizer.from_pretrained(str(checkpoint_path))
        model.eval()

        max_length = config["model"]["max_length"]
        dummy = tokenizer("Exemple", max_length=max_length, padding="max_length", truncation=True, return_tensors="pt")

        export_dir = AI_ROOT / "training" / "text" / "exports"
        export_dir.mkdir(parents=True, exist_ok=True)
        output_path = str(export_dir / "text_sentinel.onnx")

        torch.onnx.export(
            model,
            (dummy["input_ids"], dummy["attention_mask"]),
            output_path,
            opset_version=config["export"]["opset_version"],
            input_names=["input_ids", "attention_mask"],
            output_names=["predictions"],
            dynamic_axes={
                "input_ids": {0: "batch", 1: "sequence"},
                "attention_mask": {0: "batch", 1: "sequence"},
                "predictions": {0: "batch"},
            },
        )

        # Simplification ONNX (optionnelle)
        try:
            onnx_model = onnx.load(output_path)
            simplified, ok = simplify(onnx_model)
            if ok:
                onnx.save(simplified, output_path)
                logger.info("Export text-sentiment: modele ONNX simplifie")
        except Exception as e:
            logger.warning("Simplification ONNX text echouee (non bloquant): %s", e)

        tokenizer.save_pretrained(str(export_dir))
        file_size = os.path.getsize(output_path)
        logger.info("Export text-sentiment termine: %s (%d octets)", output_path, file_size)

        return ExportResult(
            model_type=ModelType.TEXT_SENTIMENT.value,
            file_path=output_path,
            file_size_bytes=file_size,
        )

    except HTTPException:
        raise
    except Exception as e:
        logger.exception("Erreur export text-sentiment")
        raise HTTPException(500, f"Erreur export text: {e}")


def _export_vision() -> ExportResult:
    """Exporte le modele image-classification vers ONNX."""
    try:
        import onnx
        from onnxsim import simplify
        import torch.nn as nn
        from torchvision import models

        config = _load_config(ModelType.IMAGE_CLASSIFICATION)

        checkpoint_path = AI_ROOT / "training" / "vision" / "checkpoints" / "best_model.pt"
        if not checkpoint_path.exists():
            raise HTTPException(404, "Aucun checkpoint vision trouve. Lancez un entrainement d'abord.")

        logger.info("Export image-classification: chargement du checkpoint")
        checkpoint = torch.load(str(checkpoint_path), map_location="cpu")

        model = models.efficientnet_v2_s(weights=None)
        model.classifier[1] = nn.Linear(model.classifier[1].in_features, config["model"]["num_classes"])
        model.load_state_dict(checkpoint["model_state_dict"])
        model.eval()

        size = config["model"]["input_size"]
        dummy = torch.randn(1, 3, size, size)

        export_dir = AI_ROOT / "training" / "vision" / "exports"
        export_dir.mkdir(parents=True, exist_ok=True)
        output_path = str(export_dir / "vision_sentinel.onnx")

        torch.onnx.export(
            model,
            dummy,
            output_path,
            opset_version=config["export"]["opset_version"],
            input_names=["image"],
            output_names=["predictions"],
            dynamic_axes={
                "image": {0: "batch"},
                "predictions": {0: "batch"},
            },
        )

        # Simplification ONNX (optionnelle)
        onnx_model = onnx.load(output_path)
        try:
            simplified, ok = simplify(onnx_model)
            if ok:
                onnx.save(simplified, output_path)
                logger.info("Export image-classification: modele ONNX simplifie")
        except Exception as e:
            logger.warning("Simplification ONNX vision echouee (non bloquant): %s", e)

        file_size = os.path.getsize(output_path)
        logger.info("Export image-classification termine: %s (%d octets)", output_path, file_size)

        return ExportResult(
            model_type=ModelType.IMAGE_CLASSIFICATION.value,
            file_path=output_path,
            file_size_bytes=file_size,
        )

    except HTTPException:
        raise
    except Exception as e:
        logger.exception("Erreur export image-classification")
        raise HTTPException(500, f"Erreur export vision: {e}")
