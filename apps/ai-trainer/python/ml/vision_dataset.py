"""Dataset PyTorch pour la classification d'images."""

import logging
from pathlib import Path
from typing import Any

from PIL import Image
from torch.utils.data import Dataset

from constants import VISION_EXTENSIONS

logger = logging.getLogger("sentinel.trainer.dataset.vision")

CLASS_MAP: dict[str, int] = {
    "safe": 0,
    "nsfw": 1,
    "illicit": 2,
}


class VisionSentinelDataset(Dataset):
    """root_dir/{safe,nsfw,illicit}/*.{jpg,png,webp,bmp}."""

    def __init__(self, root_dir: str, transform: Any = None) -> None:
        self.root_dir = Path(root_dir)
        self.transform = transform
        self.samples: list[tuple[str, int]] = []

        for class_name, label in CLASS_MAP.items():
            class_dir = self.root_dir / class_name
            if not class_dir.exists():
                continue
            for img_path in class_dir.iterdir():
                if img_path.suffix.lower() in VISION_EXTENSIONS:
                    self.samples.append((str(img_path), label))

    def __len__(self) -> int:
        return len(self.samples)

    def __getitem__(self, idx: int) -> tuple[Any, int]:
        img_path, label = self.samples[idx]
        with Image.open(img_path) as img:
            image = img.convert("RGB")
        if self.transform:
            image = self.transform(image)
        return image, label
