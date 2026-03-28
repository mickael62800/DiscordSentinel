"""
Export des modeles entraines vers ONNX pour inference Rust
"""

import os
from pathlib import Path

import sys
import io
import yaml
import torch
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

# Fix Windows console encoding pour eviter les erreurs d'emoji
if sys.stdout.encoding != 'utf-8':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')

router = APIRouter()

AI_ROOT = Path(__file__).resolve().parent.parent.parent


class ExportResult(BaseModel):
    model_type: str
    file_path: str
    file_size_bytes: int


@router.post("/export/{model_type}")
async def export_onnx(model_type: str) -> ExportResult:
    if model_type == "text-sentiment":
        return _export_text()
    elif model_type == "image-classification":
        return _export_vision()
    else:
        raise HTTPException(400, f"Type de modele inconnu: {model_type}")


@router.get("/models")
async def list_models():
    models = []

    text_onnx = AI_ROOT / "training" / "text" / "exports" / "text_sentinel.onnx"
    if text_onnx.exists():
        models.append({
            "model_type": "text-sentiment",
            "file_path": str(text_onnx),
            "file_size_bytes": text_onnx.stat().st_size,
            "last_modified": text_onnx.stat().st_mtime,
        })

    vision_onnx = AI_ROOT / "training" / "vision" / "exports" / "vision_sentinel.onnx"
    if vision_onnx.exists():
        models.append({
            "model_type": "image-classification",
            "file_path": str(vision_onnx),
            "file_size_bytes": vision_onnx.stat().st_size,
            "last_modified": vision_onnx.stat().st_mtime,
        })

    text_checkpoint = AI_ROOT / "training" / "text" / "checkpoints" / "best_model"
    if text_checkpoint.exists():
        models.append({
            "model_type": "text-sentiment-checkpoint",
            "file_path": str(text_checkpoint),
            "status": "pret pour export",
        })

    vision_checkpoint = AI_ROOT / "training" / "vision" / "checkpoints" / "best_model.pt"
    if vision_checkpoint.exists():
        models.append({
            "model_type": "image-classification-checkpoint",
            "file_path": str(vision_checkpoint),
            "file_size_bytes": vision_checkpoint.stat().st_size,
            "status": "pret pour export",
        })

    return models


def _export_text() -> ExportResult:
    try:
        import onnx
        from onnxsim import simplify
        from transformers import AutoModelForSequenceClassification, AutoTokenizer

        config_path = AI_ROOT / "training" / "text" / "configs" / "train_config.yaml"
        with open(config_path) as f:
            config = yaml.safe_load(f)

        checkpoint_path = AI_ROOT / "training" / "text" / "checkpoints" / "best_model"
        if not checkpoint_path.exists():
            raise HTTPException(404, "Aucun checkpoint text trouve. Lancez un entrainement d'abord.")

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

        try:
            onnx_model = onnx.load(output_path)
            simplified, ok = simplify(onnx_model)
            if ok:
                onnx.save(simplified, output_path)
        except Exception:
            pass  # Simplification optionnelle

        tokenizer.save_pretrained(str(export_dir))

        return ExportResult(
            model_type="text-sentiment",
            file_path=output_path,
            file_size_bytes=os.path.getsize(output_path),
        )

    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(500, f"Erreur export text: {e}")


def _export_vision() -> ExportResult:
    try:
        import onnx
        from onnxsim import simplify
        import torch.nn as nn
        from torchvision import models

        config_path = AI_ROOT / "training" / "vision" / "configs" / "train_config.yaml"
        with open(config_path) as f:
            config = yaml.safe_load(f)

        checkpoint_path = AI_ROOT / "training" / "vision" / "checkpoints" / "best_model.pt"
        if not checkpoint_path.exists():
            raise HTTPException(404, "Aucun checkpoint vision trouve. Lancez un entrainement d'abord.")

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

        onnx_model = onnx.load(output_path)
        try:
            simplified, ok = simplify(onnx_model)
            if ok:
                onnx.save(simplified, output_path)
        except Exception:
            pass

        return ExportResult(
            model_type="image-classification",
            file_path=output_path,
            file_size_bytes=os.path.getsize(output_path),
        )

    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(500, f"Erreur export vision: {e}")
