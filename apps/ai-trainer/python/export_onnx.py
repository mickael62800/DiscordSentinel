"""
Export ONNX CLI — transforme un checkpoint entraine en fichier .onnx.

Emet sur stdout une ligne JSON finale:
    {"event":"done","file_path":"...","file_size_bytes":123}
ou en cas d'erreur:
    {"event":"error","message":"..."}
"""

import argparse
import io
import json
import os
import sys
from pathlib import Path

import yaml
import torch

if sys.stdout.encoding != "utf-8":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))


def _emit(event: str, **payload) -> None:
    payload["event"] = event
    sys.stdout.write(json.dumps(payload, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def _load_config(model_type: str) -> dict:
    folder = "text" if model_type == "text-sentiment" else "vision"
    with open(SCRIPT_DIR / "configs" / folder / "train_config.yaml", encoding="utf-8") as f:
        return yaml.safe_load(f)


def export_text(data_root: Path) -> dict:
    import onnx
    from onnxsim import simplify
    from transformers import AutoModelForSequenceClassification, AutoTokenizer

    config = _load_config("text-sentiment")
    checkpoint_path = data_root / "text" / "checkpoints" / "best_model"
    if not checkpoint_path.exists():
        raise FileNotFoundError("Aucun checkpoint text. Lancez un entrainement d'abord.")

    _emit("phase", phase="chargement checkpoint")
    model = AutoModelForSequenceClassification.from_pretrained(str(checkpoint_path))
    tokenizer = AutoTokenizer.from_pretrained(str(checkpoint_path))
    model.eval()

    max_length = config["model"]["max_length"]
    dummy = tokenizer("Exemple", max_length=max_length, padding="max_length", truncation=True, return_tensors="pt")

    export_dir = data_root / "text" / "exports"
    export_dir.mkdir(parents=True, exist_ok=True)
    output_path = str(export_dir / "text_sentinel.onnx")

    _emit("phase", phase="conversion ONNX")
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
        _emit("phase", phase="simplification ONNX")
        onnx_model = onnx.load(output_path)
        simplified, ok = simplify(onnx_model)
        if ok:
            onnx.save(simplified, output_path)
    except Exception:
        pass

    tokenizer.save_pretrained(str(export_dir))
    return {"model_type": "text-sentiment", "file_path": output_path, "file_size_bytes": os.path.getsize(output_path)}


def export_vision(data_root: Path) -> dict:
    import onnx
    from onnxsim import simplify
    import torch.nn as nn
    from torchvision import models

    config = _load_config("image-classification")
    checkpoint_path = data_root / "vision" / "checkpoints" / "best_model.pt"
    if not checkpoint_path.exists():
        raise FileNotFoundError("Aucun checkpoint vision. Lancez un entrainement d'abord.")

    _emit("phase", phase="chargement checkpoint")
    checkpoint = torch.load(str(checkpoint_path), map_location="cpu")
    model = models.efficientnet_v2_s(weights=None)
    model.classifier[1] = nn.Linear(model.classifier[1].in_features, config["model"]["num_classes"])
    model.load_state_dict(checkpoint["model_state_dict"])
    model.eval()

    size = config["model"]["input_size"]
    dummy = torch.randn(1, 3, size, size)

    export_dir = data_root / "vision" / "exports"
    export_dir.mkdir(parents=True, exist_ok=True)
    output_path = str(export_dir / "vision_sentinel.onnx")

    _emit("phase", phase="conversion ONNX")
    torch.onnx.export(
        model, dummy, output_path,
        opset_version=config["export"]["opset_version"],
        input_names=["image"], output_names=["predictions"],
        dynamic_axes={"image": {0: "batch"}, "predictions": {0: "batch"}},
    )

    try:
        _emit("phase", phase="simplification ONNX")
        onnx_model = onnx.load(output_path)
        simplified, ok = simplify(onnx_model)
        if ok:
            onnx.save(simplified, output_path)
    except Exception:
        pass

    return {"model_type": "image-classification", "file_path": output_path, "file_size_bytes": os.path.getsize(output_path)}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-type", required=True, choices=["text-sentiment", "image-classification"])
    parser.add_argument("--data-root", required=True)
    args = parser.parse_args()

    data_root = Path(args.data_root).resolve()

    try:
        if args.model_type == "text-sentiment":
            result = export_text(data_root)
        else:
            result = export_vision(data_root)
        _emit("done", **result)
        return 0
    except Exception as e:
        _emit("error", message=str(e))
        return 1


if __name__ == "__main__":
    sys.exit(main())
