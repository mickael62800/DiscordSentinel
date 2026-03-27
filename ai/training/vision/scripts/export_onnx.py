"""
Export du modele vision entraine vers ONNX pour inference Rust
"""

import yaml
import torch
import onnx
from onnxsim import simplify
from train import build_model


def export(checkpoint_path: str = "../checkpoints/best_model.pt"):
    with open("../configs/train_config.yaml", "r") as f:
        config = yaml.safe_load(f)

    # Charger le modele
    checkpoint = torch.load(checkpoint_path, map_location="cpu", weights_only=False)
    model = build_model(config)
    model.load_state_dict(checkpoint["model_state_dict"])
    model.eval()

    # Dummy input
    input_size = config["model"]["input_size"]
    dummy = torch.randn(1, 3, input_size, input_size)

    # Export
    output_path = config["export"]["output_path"]
    torch.onnx.export(
        model,
        dummy,
        output_path,
        opset_version=config["export"]["opset_version"],
        input_names=["image"],
        output_names=["predictions"],
        dynamic_axes={"image": {0: "batch"}, "predictions": {0: "batch"}}
        if config["export"]["dynamic_axes"] else None,
    )

    # Simplifier
    if config["export"]["simplify"]:
        onnx_model = onnx.load(output_path)
        simplified, ok = simplify(onnx_model)
        if ok:
            onnx.save(simplified, output_path)
            print("Modele simplifie avec succes")

    print(f"Modele exporte: {output_path}")


if __name__ == "__main__":
    export()
