"""
Export du modele text entraine vers ONNX pour inference Rust
"""

import yaml
import torch
import onnx
from onnxsim import simplify
from transformers import AutoModelForSequenceClassification, AutoTokenizer


def export(checkpoint_path: str = "../checkpoints/best_model"):
    with open("../configs/train_config.yaml", "r") as f:
        config = yaml.safe_load(f)

    # Charger le modele
    model = AutoModelForSequenceClassification.from_pretrained(checkpoint_path)
    tokenizer = AutoTokenizer.from_pretrained(checkpoint_path)
    model.eval()

    # Dummy input
    max_length = config["model"]["max_length"]
    dummy = tokenizer(
        "Exemple de texte",
        max_length=max_length,
        padding="max_length",
        truncation=True,
        return_tensors="pt",
    )

    # Export
    output_path = config["export"]["output_path"]
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
        } if config["export"]["dynamic_axes"] else None,
    )

    # Simplifier
    if config["export"]["simplify"]:
        onnx_model = onnx.load(output_path)
        simplified, ok = simplify(onnx_model)
        if ok:
            onnx.save(simplified, output_path)
            print("Modele simplifie avec succes")

    # Exporter le tokenizer au format compatible HuggingFace tokenizers (Rust)
    import os
    export_dir = os.path.dirname(output_path)
    tokenizer_export_path = os.path.join(export_dir, "tokenizer.json")
    tokenizer.save_pretrained(export_dir)
    # Le fichier tokenizer.json est genere automatiquement par save_pretrained
    # C'est celui que le crate tokenizers (Rust) utilise
    print(f"Modele exporte: {output_path}")
    print(f"Tokenizer exporte: {tokenizer_export_path}")


if __name__ == "__main__":
    export()
